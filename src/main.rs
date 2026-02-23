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
use types::{AdminCommand, BotEvent, LogLevel};

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
                                engine::load_plugins(&lua)
                            };
                            let _ = tx_for_loop.send(BotEvent::Log(level, report));
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