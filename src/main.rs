mod commands;
mod engine;
mod events;
mod tui;
mod types;

use dotenvy::dotenv;
use mlua::Lua;
use poise::serenity_prelude as serenity;
use std::sync::{Arc, Mutex};
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
                        let data_result = engine::init(ctx, tx_for_engine.clone()).await;

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
                        _ => {}
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
    tui::run(tx_to_bot, rx_from_tui)?;

    Ok(())
}
