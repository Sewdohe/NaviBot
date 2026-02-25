use mlua::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use rusqlite::{Connection, OptionalExtension};
use tokio::sync::mpsc::UnboundedSender;
use crate::types::{
    BotEvent, LogLevel, ConfigField, ConfigItemSchema, ConfigRegistry, ConfigType, PluginSchema,
};

pub fn register(
    lua: &Lua,
    navi: &LuaTable,
    db: Arc<Mutex<Connection>>,
    config_registry: ConfigRegistry,
    tui_tx: UnboundedSender<BotEvent>,
) -> LuaResult<()> {
    let registry_for_lua = config_registry.clone();
    let db_for_config = db.clone();
    let tx_config = tui_tx.clone();

    navi.set(
        "register_config",
        lua.create_function(move |lua_inner, (plugin_name, schema): (String, mlua::Table)| {
            let mut fields = Vec::new();

            for pair in schema.pairs::<mlua::Integer, mlua::Table>() {
                let (_, field_table) = pair?;

                let key: String = field_table.get("key")?;
                let name: String = field_table.get("name").unwrap_or_else(|_| key.clone());
                let description: String = field_table.get("description").unwrap_or_default();
                let type_str: String = field_table
                    .get("type")
                    .unwrap_or_else(|_| "string".to_string());

                let default_value: String = match field_table.get::<_, mlua::Value>("default") {
                    Ok(mlua::Value::String(s)) => s.to_str()?.to_string(),
                    Ok(mlua::Value::Integer(i)) => i.to_string(),
                    Ok(mlua::Value::Number(n)) => n.to_string(),
                    Ok(mlua::Value::Boolean(b)) => b.to_string(),
                    _ => "".to_string(),
                };

                let field_type = match type_str.as_str() {
                    "number" => ConfigType::Number,
                    "boolean" => ConfigType::Boolean,
                    "channel" => ConfigType::Channel,
                    "role" => ConfigType::Role,
                    "category" => ConfigType::Category,
                    "list" => ConfigType::List,
                    _ => ConfigType::String,
                };

                // Parse item_schema for List fields
                let item_schema: Vec<ConfigItemSchema> = if field_type == ConfigType::List {
                    let schema_table: mlua::Table = field_table
                        .get("item_schema")
                        .unwrap_or_else(|_| lua_inner.create_table().unwrap());
                    schema_table
                        .pairs::<mlua::Integer, mlua::Table>()
                        .filter_map(|p| p.ok())
                        .map(|(_, t)| {
                            let sub_key: String = t.get("key").unwrap_or_default();
                            let sub_name: String =
                                t.get("name").unwrap_or_else(|_| sub_key.clone());
                            let sub_type_str: String =
                                t.get("type").unwrap_or_else(|_| "string".into());
                            let sub_type = match sub_type_str.as_str() {
                                "number" => ConfigType::Number,
                                "boolean" => ConfigType::Boolean,
                                "channel" => ConfigType::Channel,
                                "role" => ConfigType::Role,
                                "category" => ConfigType::Category,
                                _ => ConfigType::String,
                            };
                            ConfigItemSchema {
                                key: sub_key,
                                name: sub_name,
                                field_type: sub_type,
                            }
                        })
                        .collect()
                } else {
                    vec![]
                };

                // Load existing list items from DB for List fields
                let list_items: Vec<HashMap<String, String>> = if field_type == ConfigType::List {
                    let count_key = format!("config:{}:{}:_count", plugin_name, key);
                    let count: usize = if let Ok(conn) = db_for_config.lock() {
                        conn.query_row(
                            "SELECT value FROM kv_store WHERE key = ?1",
                            [&count_key],
                            |row| row.get::<_, String>(0),
                        )
                        .optional()
                        .unwrap_or(None)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0)
                    } else {
                        0
                    };

                    (0..count)
                        .filter_map(|i| {
                            let item_key = format!("config:{}:{}:{}", plugin_name, key, i);
                            if let Ok(conn) = db_for_config.lock() {
                                conn.query_row(
                                    "SELECT value FROM kv_store WHERE key = ?1",
                                    [&item_key],
                                    |row| row.get::<_, String>(0),
                                )
                                .optional()
                                .ok()
                                .flatten()
                                .and_then(|json| {
                                    serde_json::from_str::<HashMap<String, String>>(&json).ok()
                                })
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    vec![]
                };

                // DIRECT SQLITE ACCESS - Safely read from the database to see if a value exists
                // (only for scalar fields; List fields use list_items above)
                let final_value = if field_type != ConfigType::List {
                    let db_key = format!("config:{}:{}", plugin_name, key);
                    let actual_value: Option<String> = if let Ok(conn) = db_for_config.lock() {
                        conn.query_row(
                            "SELECT value FROM kv_store WHERE key = ?1",
                            [&db_key],
                            |row| row.get::<_, String>(0),
                        )
                        .optional()
                        .unwrap_or(None)
                    } else {
                        None
                    };

                    if let Some(val) = actual_value {
                        val
                    } else {
                        if let Ok(conn) = db_for_config.lock() {
                            let _ = conn.execute(
                                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                                (&db_key, &default_value),
                            );
                        }
                        default_value.clone()
                    }
                } else {
                    String::new()
                };

                fields.push(ConfigField {
                    key: key.clone(),
                    name,
                    description,
                    field_type,
                    default_value: final_value,
                    item_schema,
                    list_items,
                });
            }

            let plugin_schema = PluginSchema { fields };

            {
                let mut registry = registry_for_lua.lock().unwrap();
                registry.insert(plugin_name.clone(), plugin_schema);
            }

            let _ = tx_config.send(BotEvent::Log(
                LogLevel::Info,
                format!("Registered config schema for plugin: {}", plugin_name),
            ));

            Ok(())
        })?,
    )?;

    Ok(())
}
