mod commands;
mod engine;
mod events;
mod tui;
mod types;

use dotenvy::dotenv;
use poise::serenity_prelude as serenity;
use tokio::sync::mpsc;
use types::{AdminCommand, BotEvent}; // Import the new types

// We REMOVE #[tokio::main] because we are managing threads manually now!
fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // 1. Create Channels (The "Phone Lines")
    let (tx_to_tui, rx_from_tui) = mpsc::unbounded_channel::<BotEvent>();
    let (tx_to_bot, mut rx_from_tui_cmd) = mpsc::unbounded_channel::<AdminCommand>();

    // 2. Spawn Bot Thread
    std::thread::spawn(move || {
        // Start the Tokio Runtime
        let rt = tokio::runtime::Runtime::new().unwrap();

        // --- CLONING STATION ---
        // We need separate copies of the "Phone" for different parts of the code.

        // Copy 1: Goes into the Framework Setup (consumed by the move)
        let tx_for_setup = tx_to_tui.clone();

        // Copy 2: Stays here for the Admin Loop
        let tx_for_loop = tx_to_tui.clone();

        rt.block_on(async move {
            let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
            let intents = serenity::GatewayIntents::non_privileged()
                | serenity::GatewayIntents::MESSAGE_CONTENT;

            // Define Framework
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
                        let data = engine::init(ctx, tx_for_engine).await?;

                        // 1. Get commands (Force drop lock immediately)
                        let commands = {
                            let lua = data.lua.lock().unwrap();
                            engine::read_slash_commands(&lua)?
                        };
                        // <--- The Lock is 100% dead here

                        // 2. Upload
                        let sync_report =
                            engine::upload_slash_commands(&ctx.http, commands).await?;

                        let _ = tx_for_setup.send(BotEvent::Log(sync_report));
                        let _ = tx_for_setup.send(BotEvent::Log("✅ Bot is Online!".into()));

                        Ok(data)
                    })
                })
                .build();

            // FIX #1: Added 'mut' here
            let mut client = serenity::ClientBuilder::new(token, intents)
                .framework(framework)
                .await
                .unwrap();

            let shard_manager = client.shard_manager.clone();

            // Spawn the Client (Bot)
            tokio::spawn(async move {
                if let Err(why) = client.start().await {
                    println!("Client error: {:?}", why);
                }
            });

            // 3. Listen for Admin Commands (from TUI)
            // FIX #2: We use 'tx_for_loop' here, which was NOT moved into the framework
            while let Some(cmd) = rx_from_tui_cmd.recv().await {
                match cmd {
                    AdminCommand::Shutdown => {
                        let _ = tx_for_loop.send(BotEvent::Log("🔴 Shutting down...".into()));
                        shard_manager.shutdown_all().await;
                        break;
                    }
                    AdminCommand::Reload => {
                        let _ = tx_for_loop.send(BotEvent::Log(
                            "🔄 Reload triggered via TUI (Not implemented yet)".into(),
                        ));
                    }
                    _ => {}
                }
            }
        });
    });

    // 3. Start TUI (Main Thread)
    // This blocks until the user presses 'q'
    tui::run(tx_to_bot, rx_from_tui)?;

    Ok(())
}
