mod commands;
mod engine;
mod events;
mod tui;
mod types;

use dotenvy::dotenv;
use mlua::Lua;
use poise::serenity_prelude as serenity;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use types::{AdminCommand, BotEvent, ConfigType, LogLevel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // 1. CHANNELS
    // TUI <-> Bot communication
    let (tx_to_tui, rx_from_tui) = mpsc::unbounded_channel::<BotEvent>();
    let (tx_to_bot, mut rx_from_tui_cmd) = mpsc::unbounded_channel::<AdminCommand>();

    // SMUGGLE CHANNEL: Passes the Lua instance from the async bot to the sync admin loop
    let (init_tx, mut init_rx) = mpsc::unbounded_channel::<Arc<Mutex<Lua>>>();

    // [ADDITION 1] Create the Config Registry
    let config_registry: types::ConfigRegistry = Arc::new(Mutex::new(HashMap::new()));
    let registry_for_engine = config_registry.clone();
    let registry_for_tui = config_registry.clone();
    let registry_for_loop = config_registry.clone();

    // Interval registry — shared between engine init and the admin reload loop
    let interval_registry: types::IntervalRegistry = Arc::new(Mutex::new(HashMap::new()));
    let registry_for_engine_intervals = interval_registry.clone();
    let registry_for_loop_intervals = interval_registry.clone();

    // Discord state shared across the bot and TUI (e.g. channel list)
    let discord_state: types::SharedDiscordState =
        Arc::new(Mutex::new(types::DiscordState::default()));
    let state_for_engine = discord_state.clone();
    let state_for_tui = discord_state.clone();

    // 2. SPAWN BOT THREAD
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();

        let tx_for_setup = tx_to_tui.clone();
        let tx_for_loop = tx_to_tui.clone();

        rt.block_on(async move {
            let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
            let intents = serenity::GatewayIntents::non_privileged()
                | serenity::GatewayIntents::MESSAGE_CONTENT
                | serenity::GatewayIntents::GUILD_MEMBERS;

            let framework = poise::Framework::builder()
                .options(poise::FrameworkOptions {
                    commands: vec![commands::reload(), commands::sync()],
                    prefix_options: poise::PrefixFrameworkOptions {
                        prefix: Some("!".into()),
                        ..Default::default()
                    },
                    event_handler: |ctx, event, framework, data| {
                        Box::pin(events::event_handler(ctx, event, framework, data))
                    },
                    ..Default::default()
                })
                .setup(move |ctx, _ready, framework| {
                    Box::pin(async move {
                        poise::builtins::register_globally(ctx, &framework.options().commands)
                            .await?;

                        let tx_for_engine = tx_for_setup.clone();

                        let data_result = engine::init(
                            ctx,
                            tx_for_engine.clone(),
                            registry_for_engine,
                            state_for_engine,
                            registry_for_engine_intervals,
                        )
                        .await;

                        match data_result {
                            Ok(data) => {
                                let commands_result = {
                                    let lua = data.lua.lock().unwrap();
                                    engine::read_slash_commands(&lua)
                                }; 

                                let sync_report = match commands_result {
                                    Ok(cmds) => engine::upload_slash_commands(&ctx.http, cmds)
                                        .await
                                        .unwrap_or_else(|e| format!("Sync failed: {}", e)),
                                    Err(e) => format!("Lua Read Error: {}", e),
                                };

                                let _ = tx_for_setup.send(BotEvent::Log(LogLevel::Info, sync_report));
                                let _ = tx_for_setup.send(BotEvent::Log(LogLevel::Info, "Bot is Online!".into()));

                                // Send the Lua instance to the Admin Loop
                                let _ = init_tx.send(data.lua.clone());

                                Ok(data)
                            }
                            Err(e) => {
                                let _ = tx_for_setup
                                    .send(BotEvent::Log(LogLevel::Error, format!("CRITICAL ERROR: {}", e)));
                                Err(e)
                            }
                        }
                    })
                })
                .build();

            let mut client = serenity::ClientBuilder::new(token, intents)
                .framework(framework)
                .await
                .unwrap();

            let shard_manager = client.shard_manager.clone();
            
            // --- THE FIX IS HERE ---
            // We grab a clone of the HTTP client immediately after the client is built
            let http_for_loop = client.http.clone();

            tokio::spawn(async move {
                if let Err(why) = client.start().await {
                    println!("Client error: {:?}", why);
                }
            });

            // 3. ADMIN LOOP
            if let Some(lua_instance) = init_rx.recv().await {
                while let Some(cmd) = rx_from_tui_cmd.recv().await {
                    match cmd {
                        AdminCommand::Shutdown => {
                            let _ = tx_for_loop.send(BotEvent::Log(LogLevel::Info, "Shutting down...".into()));
                            shard_manager.shutdown_all().await;
                            break;
                        }
                        AdminCommand::Reload => {
                            let (level, report) = {
                                let lua = lua_instance.lock().unwrap();
                                engine::load_plugins(&lua, &registry_for_loop_intervals)
                            };
                            let _ = tx_for_loop.send(BotEvent::Log(level, report));

                            // Re-sync slash commands after reload
                            let cmds = {
                                let lua = lua_instance.lock().unwrap();
                                engine::read_slash_commands(&lua)
                            };
                            let http = http_for_loop.clone();
                            let tx = tx_for_loop.clone();
                            tokio::spawn(async move {
                                let msg = match cmds {
                                    Ok(c) => engine::upload_slash_commands(&http, c)
                                        .await
                                        .unwrap_or_else(|e| format!("Sync failed: {}", e)),
                                    Err(e) => format!("Lua read error during sync: {}", e),
                                };
                                let _ = tx.send(BotEvent::Log(LogLevel::Info, msg));
                            });
                        }
                        AdminCommand::RefreshCache => {
                            // Print the loading message
                            let _ = tx_for_loop.send(BotEvent::Log(LogLevel::Info, "Fetching latest roles and channels...".into()));

                            // Pass our cloned HTTP client directly into the thread
                            let http_clone = http_for_loop.clone(); 
                            let state_clone = discord_state.clone();
                            let tx = tx_for_loop.clone();

                            tokio::spawn(async move {
                                use poise::serenity_prelude as serenity;
                                
                                if let Ok(guilds) = http_clone.get_guilds(None, None).await {
                                    let mut new_channels: Vec<(String, String)> = Vec::new();
                                    let mut new_categories: Vec<(String, String)> = Vec::new();
                                    let mut new_roles: Vec<crate::types::DiscordRole> = Vec::new();

                                    for guild_info in guilds {
                                        let guild_id = guild_info.id;

                                        // 1. Fetch Channels & Categories
                                        if let Ok(channels) = guild_id.channels(&http_clone).await {
                                            for (id, channel) in channels {
                                                if channel.kind == serenity::ChannelType::Category {
                                                    new_categories.push((id.to_string(), channel.name));
                                                } else {
                                                    new_channels.push((id.to_string(), channel.name));
                                                }
                                            }
                                        }

                                        // 2. Fetch Roles
                                        if let Ok(roles) = guild_id.roles(&http_clone).await {
                                            for (id, role) in roles {
                                                let r = ((role.colour.0 >> 16) & 255) as u8;
                                                let g = ((role.colour.0 >> 8) & 255) as u8;
                                                let b = (role.colour.0 & 255) as u8;

                                                new_roles.push(crate::types::DiscordRole {
                                                    id: id.to_string(),
                                                    name: role.name,
                                                    color: (r, g, b),
                                                });
                                            }
                                        }
                                    }

                                    // 3. Lock the state and hot-swap the fresh data
                                    let (c_count, cat_count, r_count) = {
                                        let mut state = state_clone.lock().unwrap();
                                        state.channels = new_channels;
                                        state.categories = new_categories;
                                        state.roles = new_roles;
                                        (state.channels.len(), state.categories.len(), state.roles.len())
                                    };

                                    let _ = tx.send(BotEvent::Log(LogLevel::Info, format!(
                                        "Cached {} channels, {} categories, {} roles.",
                                        c_count, cat_count, r_count
                                    )));
                                } else {
                                    let _ = tx.send(BotEvent::Log(LogLevel::Error, "Failed to fetch Discord cache!".into()));
                                }
                            });
                        }
                        AdminCommand::SaveConfig { plugin, key, value } => {
                            let db_key = format!("config:{}:{}", plugin, key);

                            let _ = lua_instance
                                .lock()
                                .unwrap()
                                .globals()
                                .get::<_, mlua::Table>("navi")
                                .and_then(|n| n.get::<_, mlua::Table>("db"))
                                .and_then(|db| db.get::<_, mlua::Function>("set"))
                                .and_then(|set_fn| {
                                    set_fn.call::<_, ()>((db_key.clone(), value.clone()))
                                });

                            let mut registry = registry_for_loop.lock().unwrap();
                            if let Some(schema) = registry.get_mut(&plugin) {
                                if let Some(field) = schema.fields.iter_mut().find(|f| f.key == key)
                                {
                                    field.default_value = value.clone();
                                }
                            }

                            let _ = tx_for_loop.send(BotEvent::Log(LogLevel::Info, format!(
                                "Saved config: {} -> {}",
                                db_key, value
                            )));
                        }
                        AdminCommand::AppendListItem { plugin, key, item } => {
                            let db_key_prefix = format!("config:{}:{}", plugin, key);
                            let count_key = format!("{}:_count", db_key_prefix);

                            let json = serde_json::to_string(&item).unwrap_or_default();
                            let _ = lua_instance.lock().unwrap()
                                .globals()
                                .get::<_, mlua::Table>("navi")
                                .and_then(|navi| {
                                    let db_get = navi.get::<_, mlua::Function>("_db_get_raw")?;
                                    let db_set = navi.get::<_, mlua::Function>("_db_set_raw")?;
                                    let count: usize = db_get
                                        .call::<_, mlua::Value>(count_key.clone())
                                        .ok()
                                        .and_then(|v| {
                                            if let mlua::Value::String(s) = v {
                                                s.to_str().ok().and_then(|s| s.parse().ok())
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or(0);
                                    let item_key = format!("{}:{}", db_key_prefix, count);
                                    db_set.call::<_, ()>((item_key, json))?;
                                    db_set.call::<_, ()>((count_key, (count + 1).to_string()))
                                });

                            let mut registry = registry_for_loop.lock().unwrap();
                            if let Some(schema) = registry.get_mut(&plugin) {
                                if let Some(field) = schema.fields.iter_mut().find(|f| {
                                    f.key == key && f.field_type == ConfigType::List
                                }) {
                                    field.list_items.push(item);
                                }
                            }

                            let _ = tx_for_loop.send(BotEvent::Log(
                                LogLevel::Info,
                                format!("List item appended: {}:{}", plugin, key),
                            ));
                        }
                        AdminCommand::UpdateListItem { plugin, key, index, item } => {
                            let db_key_prefix = format!("config:{}:{}", plugin, key);
                            let item_key = format!("{}:{}", db_key_prefix, index);

                            let json = serde_json::to_string(&item).unwrap_or_default();
                            let _ = lua_instance.lock().unwrap()
                                .globals()
                                .get::<_, mlua::Table>("navi")
                                .and_then(|navi| navi.get::<_, mlua::Function>("_db_set_raw"))
                                .and_then(|db_set| db_set.call::<_, ()>((item_key, json)));

                            let mut registry = registry_for_loop.lock().unwrap();
                            if let Some(schema) = registry.get_mut(&plugin) {
                                if let Some(field) = schema.fields.iter_mut().find(|f| {
                                    f.key == key && f.field_type == ConfigType::List
                                }) {
                                    if index < field.list_items.len() {
                                        field.list_items[index] = item;
                                    }
                                }
                            }

                            let _ = tx_for_loop.send(BotEvent::Log(
                                LogLevel::Info,
                                format!("List item updated: {}:{}[{}]", plugin, key, index),
                            ));
                        }
                        AdminCommand::DeleteListItem { plugin, key, index } => {
                            let db_key_prefix = format!("config:{}:{}", plugin, key);
                            let count_key = format!("{}:_count", db_key_prefix);

                            let deletion_done = {
                                let lua = lua_instance.lock().unwrap();
                                let result = if let Ok(navi) = lua.globals().get::<_, mlua::Table>("navi") {
                                    let db_get = navi.get::<_, mlua::Function>("_db_get_raw").ok();
                                    let db_set = navi.get::<_, mlua::Function>("_db_set_raw").ok();
                                    if let (Some(db_get), Some(db_set)) = (db_get, db_set) {
                                        let count: usize = db_get
                                            .call::<_, mlua::Value>(count_key.clone())
                                            .ok()
                                            .and_then(|v| {
                                                if let mlua::Value::String(s) = v {
                                                    s.to_str().ok().and_then(|s| s.parse().ok())
                                                } else {
                                                    None
                                                }
                                            })
                                            .unwrap_or(0);

                                        if index < count {
                                            // Collect surviving items
                                            let mut survivors: Vec<String> = Vec::new();
                                            for i in 0..count {
                                                if i != index {
                                                    let ikey = format!("{}:{}", db_key_prefix, i);
                                                    if let Ok(mlua::Value::String(s)) =
                                                        db_get.call::<_, mlua::Value>(ikey)
                                                    {
                                                        if let Ok(json) = s.to_str() {
                                                            survivors.push(json.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                            // Rewrite repacked items
                                            for (new_idx, json) in survivors.iter().enumerate() {
                                                let ikey = format!("{}:{}", db_key_prefix, new_idx);
                                                let _ = db_set.call::<_, ()>((ikey, json.clone()));
                                            }
                                            let _ = db_set.call::<_, ()>((
                                                count_key,
                                                survivors.len().to_string(),
                                            ));
                                            true
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                };
                                result
                            }; // lua lock released here

                            if deletion_done {
                                let mut registry = registry_for_loop.lock().unwrap();
                                if let Some(schema) = registry.get_mut(&plugin) {
                                    if let Some(field) = schema.fields.iter_mut().find(|f| {
                                        f.key == key && f.field_type == ConfigType::List
                                    }) {
                                        if index < field.list_items.len() {
                                            field.list_items.remove(index);
                                        }
                                    }
                                }
                            }

                            let _ = tx_for_loop.send(BotEvent::Log(
                                LogLevel::Info,
                                format!("List item deleted: {}:{}[{}]", plugin, key, index),
                            ));
                        }
                    }
                }
            } else {
                let _ = tx_for_loop.send(BotEvent::Log(
                    LogLevel::Warn,
                    "Admin loop failed to capture Lua instance.".into(),
                ));
            }
        });
    });

    // 4. RUN TUI (Main Thread)
    tui::run(tx_to_bot, rx_from_tui, registry_for_tui, state_for_tui)?;

    Ok(())
}
