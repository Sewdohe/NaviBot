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
use types::{AdminCommand, BotEvent};

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
    let discord_state: types::SharedDiscordState = Arc::new(Mutex::new(types::DiscordState::default()));
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
                | serenity::GatewayIntents::MESSAGE_CONTENT;

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

                        // Error Handling Wrapper
                        // let data_result = engine::init(ctx, tx_for_engine.clone()).await;
                        let data_result =
                            engine::init(ctx, tx_for_engine.clone(), registry_for_engine, state_for_engine).await;

                        match data_result {
                            Ok(data) => {
                                // --- FIX: SCOPED LOCK FOR SYNC ---
                                // 1. Read commands (Lock Lua, Read, Drop Lock)
                                let commands_result = {
                                    let lua = data.lua.lock().unwrap();
                                    engine::read_slash_commands(&lua)
                                }; // <--- Lock is dropped here

                                // 2. Upload commands (Async, no lock held)
                                let sync_report = match commands_result {
                                    Ok(cmds) => engine::upload_slash_commands(&ctx.http, cmds)
                                        .await
                                        .unwrap_or_else(|e| format!("Sync failed: {}", e)),
                                    Err(e) => format!("Lua Read Error: {}", e),
                                };

                                let _ = tx_for_setup.send(BotEvent::Log(sync_report));
                                let _ =
                                    tx_for_setup.send(BotEvent::Log("✅ Bot is Online!".into()));

                                // Send the Lua instance to the Admin Loop
                                let _ = init_tx.send(data.lua.clone());

                                Ok(data)
                            }
                            Err(e) => {
                                let _ = tx_for_setup
                                    .send(BotEvent::Log(format!("🔥 CRITICAL ERROR: {}", e)));
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

            tokio::spawn(async move {
                if let Err(why) = client.start().await {
                    println!("Client error: {:?}", why);
                }
            });

            // 3. ADMIN LOOP
            // Wait for Lua to arrive before processing commands
            if let Some(lua_instance) = init_rx.recv().await {
                while let Some(cmd) = rx_from_tui_cmd.recv().await {
                    match cmd {
                        AdminCommand::Shutdown => {
                            let _ = tx_for_loop.send(BotEvent::Log("🔴 Shutting down...".into()));
                            shard_manager.shutdown_all().await;
                            break;
                        }
                        AdminCommand::Reload => {
                            let report = {
                                let lua = lua_instance.lock().unwrap();
                                engine::load_plugins(&lua)
                            };
                            let _ = tx_for_loop.send(BotEvent::Log(report));
                        }
                        AdminCommand::SaveConfig { plugin, key, value } => {
                            let db_key = format!("config:{}:{}", plugin, key);
                            
                            // 1. Chained one-liner to satisfy the borrow checker
                            // This locks the Mutex, calls the Lua function, and drops the lock instantly.
                            let _ = lua_instance.lock().unwrap().globals()
                                .get::<_, mlua::Table>("navi")
                                .and_then(|n| n.get::<_, mlua::Table>("db"))
                                .and_then(|db| db.get::<_, mlua::Function>("set"))
                                .and_then(|set_fn| set_fn.call::<_, ()>((db_key.clone(), value.clone())));
                            
                            // 2. Update the TUI visual registry
                            let mut registry = registry_for_loop.lock().unwrap();
                            if let Some(schema) = registry.get_mut(&plugin) {
                                if let Some(field) = schema.fields.iter_mut().find(|f| f.key == key) {
                                    field.default_value = value.clone();
                                }
                            }
                            
                            let _ = tx_for_loop.send(BotEvent::Log(format!("💾 Saved config: {} -> {}", db_key, value)));
                        }
                    }
                }
            } else {
                let _ = tx_for_loop.send(BotEvent::Log(
                    "⚠️ Admin loop failed to capture Lua instance.".into(),
                ));
            }
        });
    });

    // 4. RUN TUI (Main Thread)
    // This must be OUTSIDE the spawn block
    // tui::run(tx_to_bot, rx_from_tui)?;
    tui::run(tx_to_bot, rx_from_tui, registry_for_tui, state_for_tui)?;

    Ok(())
}
