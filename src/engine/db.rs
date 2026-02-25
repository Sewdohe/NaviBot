use mlua::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use rusqlite::{Connection, OptionalExtension};
use super::util::{json_to_lua, lua_to_json};

pub fn register(
    lua: &Lua,
    navi: &LuaTable,
    db: Arc<Mutex<Connection>>,
) -> LuaResult<()> {
    // --- JSON ---
    let json_table = lua.create_table()?;

    json_table.set("decode", lua.create_function(|lua, s: String| {
        let val: serde_json::Value = serde_json::from_str(&s).map_err(LuaError::external)?;
        json_to_lua(lua, val)
    })?)?;

    json_table.set("encode", lua.create_function(|_, val: mlua::Value| {
        let json = lua_to_json(val);
        serde_json::to_string(&json).map_err(LuaError::external)
    })?)?;

    navi.set("json", json_table)?;

    // --- DATABASE QUERYING ---
    let db_conn_query = db.clone();
    navi.set(
        "_db_query_raw",
        lua.create_function(move |lua, sql: String| {
            let conn = db_conn_query.lock().unwrap();

            let mut stmt = conn.prepare(&sql).map_err(mlua::Error::external)?;

            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0).unwrap_or_default(),
                        row.get::<_, String>(1).unwrap_or_default(),
                    ))
                })
                .map_err(mlua::Error::external)?;

            let result_table = lua.create_table()?;
            for (i, row_result) in rows.enumerate() {
                if let Ok((k, v)) = row_result {
                    let row_table = lua.create_table()?;
                    row_table.set("key", k)?;
                    row_table.set("value", v)?;
                    result_table.set(i + 1, row_table)?;
                }
            }

            Ok(result_table)
        })?,
    )?;

    // --- DB API ---
    let db_conn_set = db.clone();
    navi.set(
        "_db_set_raw",
        lua.create_function(move |_, (key, value): (String, String)| {
            let conn = db_conn_set.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                (key, value),
            )
            .map_err(mlua::Error::external)?;
            Ok(())
        })?,
    )?;

    let db_conn_get = db.clone();
    navi.set(
        "_db_get_raw",
        lua.create_function(move |lua, key: String| {
            let conn = db_conn_get.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT value FROM kv_store WHERE key = ?1")
                .map_err(mlua::Error::external)?;
            let result: Option<String> = stmt
                .query_row([key], |row| row.get(0))
                .optional()
                .map_err(mlua::Error::external)?;

            match result {
                Some(val) => Ok(mlua::Value::String(lua.create_string(&val)?)),
                None => Ok(mlua::Value::Nil),
            }
        })?,
    )?;

    // JSON decode: string -> Lua table (used by get_list)
    navi.set(
        "_json_decode",
        lua.create_function(|lua, json: String| {
            let map: HashMap<String, String> =
                serde_json::from_str(&json).map_err(mlua::Error::external)?;
            let t = lua.create_table()?;
            for (k, v) in map {
                t.set(k, v)?;
            }
            Ok(t)
        })?,
    )?;

    // Build the Smart Lua Wrapper
    lua.load(
        r#"
        navi.db = {}

        function navi.db.set(key, value)
            if not string.find(key, ":") then
                local info = debug.getinfo(2, "S")
                local source = info and info.short_src or "unknown"
                local plugin = string.match(source, "([^/\\]+)%.lua$") or "global"
                key = plugin .. ":" .. key
            end
            navi._db_set_raw(key, tostring(value))
        end

        function navi.db.get(key)
            if not string.find(key, ":") then
                local info = debug.getinfo(2, "S")
                local source = info and info.short_src or "unknown"
                local plugin = string.match(source, "([^/\\]+)%.lua$") or "global"
                key = plugin .. ":" .. key
            end
            return navi._db_get_raw(key)
        end

        -- NEW: Pass the SQL string directly to the raw pipe
        function navi.db.query(sql)
            return navi._db_query_raw(sql)
        end

        -- Read all items of a list config field
        -- Keys are stored as config:plugin:key:N in the DB, matching SaveConfig's convention
        function navi.db.get_list(key)
            if not string.find(key, ":") then
                local info = debug.getinfo(2, "S")
                local src = info and info.short_src or "unknown"
                src = src:match('"([^"]+)"') or src
                src = src:gsub("plugins[/\\]", "")
                local plugin = src:match("([^/\\]+)%.lua$") or "global"
                key = "config:" .. plugin .. ":" .. key
            end
            local count = tonumber(navi._db_get_raw(key .. ":_count")) or 0
            local result = {}
            for i = 0, count - 1 do
                local json = navi._db_get_raw(key .. ":" .. i)
                if json then
                    table.insert(result, navi._json_decode(json))
                end
            end
            return result
        end
    "#,
    )
    .exec()?;

    Ok(())
}
