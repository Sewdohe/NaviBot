use crate::types::{Data, Error};
use poise::serenity_prelude as serenity;
use mlua::{Table, Function};

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    
    // 1. MESSAGE HANDLER
    if let serenity::FullEvent::Message { new_message } = event {
        if new_message.author.bot { return Ok(()); }

        let content = new_message.content.clone();
        let author = new_message.author.name.clone();
        let channel_id = new_message.channel_id.get();

        {
            let lua = data.lua.lock().unwrap();

            // We use match to check if 'on_message' exists without crashing
            let func: Option<Function> = match lua.globals().get("on_message") {
                Ok(f) => Some(f),
                Err(_) => None,
            };

            if let Some(f) = func {
                let msg_table = lua.create_table()?;
                msg_table.set("content", content)?;
                msg_table.set("channel_id", channel_id)?;
                msg_table.set("message_id", new_message.id.get())?;
                msg_table.set("author", author)?;
                msg_table.set("author_id", new_message.author.id.get())?;
                msg_table.set("author_avatar", new_message.author.face())?;

                let mentions = lua.create_table()?;
                for (i, user) in new_message.mentions.iter().enumerate() {
                    let u = lua.create_table()?;
                    u.set("name", user.name.clone())?;
                    u.set("id", user.id.get())?;
                    u.set("avatar", user.face())?;
                    mentions.set(i + 1, u)?;
                }
                msg_table.set("mentions", mentions)?;

                let attachments = lua.create_table()?;
                for (i, attachment) in new_message.attachments.iter().enumerate() {
                    attachments.set(i + 1, attachment.url.clone())?;
                }
                msg_table.set("attachments", attachments)?;

                if let Err(e) = f.call::<_, ()>(msg_table) {
                    println!("❌ Lua Error: {}", e);
                }
            }
        }
    }

    // 2. SLASH COMMAND HANDLER
    if let serenity::FullEvent::InteractionCreate { interaction } = event {
        if let serenity::Interaction::Command(command) = interaction {
            
            let cmd_name = command.data.name.clone();
            let user_id = command.user.id.get();
            let username = command.user.name.clone();
            
            let http = ctx.http.clone();
            let interaction_id = command.id;
            let interaction_token = command.token.clone();

            {
                let lua = data.lua.lock().unwrap();

                let callback: Option<Function> = (|| {
                    let globals = lua.globals();
                    let navi: Table = globals.get("navi").ok()?;
                    let slash: Table = navi.get("slash_commands").ok()?;
                    let cmd_data: Table = slash.get(cmd_name.as_str()).ok()?;
                    cmd_data.get("callback").ok()
                })();

                if let Some(func) = callback {
                    if let Ok(ctx_table) = lua.create_table() {
                        let _ = ctx_table.set("user_id", user_id);
                        let _ = ctx_table.set("username", username);

                        let reply_fn = lua.create_function(move |_, msg: String| {
                            let http = http.clone();
                            let token = interaction_token.clone();
                            let id = interaction_id;

                            tokio::spawn(async move {
                                let response = serenity::CreateInteractionResponse::Message(
                                    serenity::CreateInteractionResponseMessage::new().content(msg)
                                );
                                if let Err(e) = http.create_interaction_response(id, &token, &response, vec![]).await {
                                    println!("Error replying: {}", e);
                                }
                            });
                            Ok(())
                        });

                        if let Ok(reply) = reply_fn {
                            let _ = ctx_table.set("reply", reply);
                            if let Err(e) = func.call::<_, ()>(ctx_table) {
                                println!("❌ Lua Slash Error: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}