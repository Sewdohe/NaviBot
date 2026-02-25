use mlua::prelude::*;

pub(super) fn json_to_lua<'lua>(lua: &'lua Lua, val: serde_json::Value) -> LuaResult<mlua::Value<'lua>> {
    match val {
        serde_json::Value::Null        => Ok(mlua::Value::Nil),
        serde_json::Value::Bool(b)     => Ok(mlua::Value::Boolean(b)),
        serde_json::Value::Number(n)   => {
            if let Some(i) = n.as_i64() { Ok(mlua::Value::Integer(i)) }
            else { Ok(mlua::Value::Number(n.as_f64().unwrap_or(0.0))) }
        }
        serde_json::Value::String(s)   => Ok(mlua::Value::String(lua.create_string(&s)?)),
        serde_json::Value::Array(arr)  => {
            let t = lua.create_table()?;
            for (i, v) in arr.into_iter().enumerate() {
                t.set(i + 1, json_to_lua(lua, v)?)?;
            }
            Ok(mlua::Value::Table(t))
        }
        serde_json::Value::Object(obj) => {
            let t = lua.create_table()?;
            for (k, v) in obj { t.set(k, json_to_lua(lua, v)?)?; }
            Ok(mlua::Value::Table(t))
        }
    }
}

pub(super) fn lua_to_json<'lua>(val: mlua::Value<'lua>) -> serde_json::Value {
    match val {
        mlua::Value::Nil           => serde_json::Value::Null,
        mlua::Value::Boolean(b)    => serde_json::Value::Bool(b),
        mlua::Value::Integer(i)    => serde_json::Value::Number(i.into()),
        mlua::Value::Number(f)     => serde_json::Number::from_f64(f)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null),
        mlua::Value::String(s)     => serde_json::Value::String(
                                        s.to_str().unwrap_or("").to_string()),
        mlua::Value::Table(t)      => {
            let len = t.raw_len();
            if len > 0 {
                let arr: Vec<_> = (1..=len)
                    .filter_map(|i| t.get::<_, mlua::Value>(i).ok())
                    .map(lua_to_json)
                    .collect();
                if arr.len() == len { return serde_json::Value::Array(arr); }
            }
            let mut map = serde_json::Map::new();
            for pair in t.pairs::<mlua::Value, mlua::Value>().flatten() {
                let key = match pair.0 {
                    mlua::Value::String(s) => s.to_str().unwrap_or("").to_string(),
                    mlua::Value::Integer(i) => i.to_string(),
                    _ => continue,
                };
                map.insert(key, lua_to_json(pair.1));
            }
            serde_json::Value::Object(map)
        }
        _ => serde_json::Value::Null,
    }
}
