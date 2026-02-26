use mlua::prelude::*;
use rusqlite::{Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

use crate::types::{
    BotEvent, ConfigField, ConfigItemSchema, ConfigRegistry, ConfigType, LogLevel, PermLevel,
    PluginSchema, SharedDiscordState,
};

fn load_mappings(db: &Arc<Mutex<Connection>>) -> Vec<HashMap<String, String>> {
    let conn = db.lock().unwrap();
    let count: usize = conn
        .query_row(
            "SELECT value FROM kv_store WHERE key = ?1",
            ["config:permissions:mappings:_count"],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .unwrap_or(None)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    (0..count)
        .filter_map(|i| {
            let item_key = format!("config:permissions:mappings:{}", i);
            conn.query_row(
                "SELECT value FROM kv_store WHERE key = ?1",
                [&item_key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str::<HashMap<String, String>>(&json).ok())
        })
        .collect()
}

/// Extract (user_id, guild_id, member_roles) from a Lua ctx table.
/// Returns None if guild_id or user_id are nil/absent (e.g. DM context).
fn extract_ctx(ctx: &LuaTable) -> LuaResult<Option<(String, String, Vec<String>)>> {
    let guild_id = match ctx.get::<_, LuaValue>("guild_id")? {
        LuaValue::String(s) => s.to_str()?.to_string(),
        LuaValue::Integer(i) => i.to_string(),
        _ => return Ok(None),
    };
    let user_id = match ctx.get::<_, LuaValue>("user_id")? {
        LuaValue::String(s) => s.to_str()?.to_string(),
        LuaValue::Integer(i) => i.to_string(),
        _ => return Ok(None),
    };
    let member_roles: Vec<String> = match ctx.get::<_, LuaValue>("member_roles")? {
        LuaValue::Table(t) => t
            .pairs::<LuaInteger, LuaValue>()
            .filter_map(|p| p.ok())
            .filter_map(|(_, v)| match v {
                LuaValue::String(s) => Some(s.to_str().unwrap_or("").to_string()),
                LuaValue::Integer(i) => Some(i.to_string()),
                _ => None,
            })
            .collect(),
        _ => vec![],
    };
    Ok(Some((user_id, guild_id, member_roles)))
}

fn user_level(
    user_id: &str,
    guild_id: &str,
    member_roles: &[String],
    discord_state: &SharedDiscordState,
    db: &Arc<Mutex<Connection>>,
) -> PermLevel {
    // Owner check — no role mapping needed
    {
        let state = discord_state.lock().unwrap();
        if state
            .guild_owners
            .get(guild_id)
            .map(|o| o == user_id)
            .unwrap_or(false)
        {
            return PermLevel::Owner;
        }
    }

    let mappings = load_mappings(db);
    let role_set: HashSet<&str> = member_roles.iter().map(|s| s.as_str()).collect();
    let mut best = PermLevel::User;
    for m in &mappings {
        if let (Some(role_id), Some(level_str)) = (m.get("role_id"), m.get("level")) {
            if role_set.contains(role_id.as_str()) {
                if let Some(lvl) = PermLevel::from_str(level_str) {
                    if lvl > best {
                        best = lvl;
                    }
                }
            }
        }
    }
    best
}

pub fn init(
    lua: &Lua,
    navi: &LuaTable,
    db: Arc<Mutex<Connection>>,
    config_registry: ConfigRegistry,
    discord_state: SharedDiscordState,
    tui_tx: UnboundedSender<BotEvent>,
) -> LuaResult<()> {
    // Register config schema directly into the registry (same structure as navi.register_config)
    let mappings = load_mappings(&db);
    let schema = PluginSchema {
        fields: vec![ConfigField {
            key: "mappings".to_string(),
            name: "Role → Permission Level".to_string(),
            description:
                "Assign roles to levels: helper / moderator / admin  (owner = server owner, automatic)"
                    .to_string(),
            field_type: ConfigType::List,
            default_value: String::new(),
            item_schema: vec![
                ConfigItemSchema {
                    key: "role_id".to_string(),
                    name: "Discord Role".to_string(),
                    field_type: ConfigType::Role,
                },
                ConfigItemSchema {
                    key: "level".to_string(),
                    name: "Level (helper/moderator/admin)".to_string(),
                    field_type: ConfigType::String,
                },
            ],
            list_items: mappings,
        }],
    };
    config_registry
        .lock()
        .unwrap()
        .insert("permissions".to_string(), schema);
    let _ = tui_tx.send(BotEvent::Log(
        LogLevel::Info,
        "Permissions system ready. Owner is automatic; map roles for other levels.".into(),
    ));

    // navi.check_perm(ctx, level) -> bool
    {
        let db = db.clone();
        let state = discord_state.clone();
        let tx = tui_tx.clone();
        navi.set(
            "check_perm",
            lua.create_function(move |_, (ctx_table, level_str): (LuaTable, String)| {
                let required = match PermLevel::from_str(&level_str) {
                    Some(l) => l,
                    None => {
                        let _ = tx.send(BotEvent::Log(
                            LogLevel::Warn,
                            format!("[perms] Unknown level: '{}'", level_str),
                        ));
                        return Ok(false);
                    }
                };
                let (user_id, guild_id, roles) = match extract_ctx(&ctx_table)? {
                    Some(v) => v,
                    None => return Ok(false),
                };
                Ok(user_level(&user_id, &guild_id, &roles, &state, &db) >= required)
            })?,
        )?;
    }

    // navi.require_perm(ctx, level) -> bool  (replies with denial if denied)
    {
        let db = db.clone();
        let state = discord_state.clone();
        let tx = tui_tx.clone();
        navi.set(
            "require_perm",
            lua.create_function(move |_, (ctx_table, level_str): (LuaTable, String)| {
                let required = match PermLevel::from_str(&level_str) {
                    Some(l) => l,
                    None => {
                        let _ = tx.send(BotEvent::Log(
                            LogLevel::Warn,
                            format!("[perms] Unknown level: '{}'", level_str),
                        ));
                        return Ok(false);
                    }
                };
                let (user_id, guild_id, roles) = match extract_ctx(&ctx_table)? {
                    Some(v) => v,
                    None => return Ok(false),
                };
                if user_level(&user_id, &guild_id, &roles, &state, &db) >= required {
                    return Ok(true);
                }
                let reply_fn: LuaFunction = ctx_table.get("reply")?;
                reply_fn.call::<_, ()>((
                    format!("❌ You need **{}** permission to use this.", level_str),
                    true,
                ))?;
                Ok(false)
            })?,
        )?;
    }

    // navi.get_perm_level(ctx) -> string  (never nil; defaults to "user")
    {
        let db = db.clone();
        let state = discord_state.clone();
        navi.set(
            "get_perm_level",
            lua.create_function(move |_, ctx_table: LuaTable| {
                let (user_id, guild_id, roles) = match extract_ctx(&ctx_table)? {
                    Some(v) => v,
                    None => return Ok("user".to_string()),
                };
                Ok(user_level(&user_id, &guild_id, &roles, &state, &db)
                    .as_str()
                    .to_string())
            })?,
        )?;
    }

    Ok(())
}
