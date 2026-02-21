use crate::types::{Data, Error};
use mlua::{Function, Table};
use poise::serenity_prelude as serenity;

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    // 1. MESSAGE HANDLER
    if let serenity::FullEvent::Message { new_message } = event {
        if new_message.author.bot {
            return Ok(());
        }

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
                    let _ = data.tui_tx.send(crate::types::BotEvent::Log(format!("❌ Lua Error: {}", e)));
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
            let channel_id = command.channel_id.get().to_string();
            let guild_id = command.guild_id.map(|g| g.get().to_string());

            // --- STEP A: PREPARE DATA ---

            // 1. Define a temporary container for our types
            enum ArgValue {
                String(String),
                Integer(i64),
                Boolean(bool),
                Number(f64),
            }

            // 2. Parse Discord Options into our Container
            let mut args_map: Vec<(String, ArgValue)> = Vec::new();

            for option in &command.data.options {
                let name = option.name.clone();

                let value = match &option.value {
                    serenity::CommandDataOptionValue::String(s) => ArgValue::String(s.clone()),
                    serenity::CommandDataOptionValue::Integer(i) => ArgValue::Integer(*i),
                    serenity::CommandDataOptionValue::Boolean(b) => ArgValue::Boolean(*b),
                    serenity::CommandDataOptionValue::Number(n) => ArgValue::Number(*n),

                    serenity::CommandDataOptionValue::User(u) => ArgValue::String(u.to_string()),
                    serenity::CommandDataOptionValue::Channel(c) => ArgValue::String(c.to_string()),
                    serenity::CommandDataOptionValue::Role(r) => ArgValue::String(r.to_string()),

                    _ => ArgValue::String("unknown".to_string()),
                };

                args_map.push((name, value));
            }

            let http = ctx.http.clone();
            let interaction_id = command.id;
            let interaction_token = command.token.clone();
            
            // CLONE THE TX FOR THE ASYNC REPLY BLOCK
            let tx_reply = data.tui_tx.clone();

            // --- STEP B: LOCK LUA ---
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
                        let _ = ctx_table.set("channel_id", channel_id);
                        let _ = ctx_table.set("guild_id", guild_id);

                        // 3. INJECT ARGUMENTS WITH TYPES
                        let args_table = lua.create_table()?;
                        for (k, v) in args_map {
                            match v {
                                ArgValue::String(s) => args_table.set(k, s)?,
                                ArgValue::Integer(i) => args_table.set(k, i)?,
                                ArgValue::Boolean(b) => args_table.set(k, b)?,
                                ArgValue::Number(n) => args_table.set(k, n)?,
                            }
                        }
                        ctx_table.set("args", args_table)?;

                        let reply_fn = lua.create_function(move |_, msg: String| {
                            let http = http.clone();
                            let token = interaction_token.clone();
                            let id = interaction_id;
                            let tx = tx_reply.clone();
                            tokio::spawn(async move {
                                let response = serenity::CreateInteractionResponse::Message(
                                    serenity::CreateInteractionResponseMessage::new().content(msg),
                                );
                                if let Err(e) = http
                                    .create_interaction_response(id, &token, &response, vec![])
                                    .await
                                {
                                    let _ = tx.send(crate::types::BotEvent::Log(format!("Error replying: {}", e)));
                                }
                            });
                            Ok(())
                        });

                        if let Ok(reply) = reply_fn {
                            let _ = ctx_table.set("reply", reply);
                            if let Err(e) = func.call::<_, ()>(ctx_table) {
                                let _ = data.tui_tx.send(crate::types::BotEvent::Log(format!("❌ Lua Slash Error: {}", e)));
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. REACTION ADD HANDLER
    if let serenity::FullEvent::ReactionAdd { add_reaction } = event {
        { 
            let ctx_lua = data.lua.lock().unwrap();
            
            if let Ok(callback) = ctx_lua.globals().get::<_, mlua::Function>("on_reaction_add") {
                if let Ok(table) = ctx_lua.create_table() {
                    let _ = table.set("user_id", add_reaction.user_id.map(|u| u.get().to_string()));
                    let _ = table.set("channel_id", add_reaction.channel_id.get().to_string());
                    let _ = table.set("message_id", add_reaction.message_id.get().to_string());
                    let _ = table.set("guild_id", add_reaction.guild_id.map(|g| g.get().to_string()));
                    let _ = table.set("emoji", add_reaction.emoji.to_string());
                    
                    if let Err(e) = callback.call::<_, ()>(table) {
                        let _ = data.tui_tx.send(crate::types::BotEvent::Log(format!("❌ Lua Reaction Error: {}", e)));
                    }
                }
            };
        } 
    }

    // 4. REACTION REMOVE HANDLER
    if let serenity::FullEvent::ReactionRemove { removed_reaction } = event {
        {
            let ctx_lua = data.lua.lock().unwrap();
            
            if let Ok(callback) = ctx_lua.globals().get::<_, mlua::Function>("on_reaction_remove") {
                if let Ok(table) = ctx_lua.create_table() {
                    let _ = table.set("user_id", removed_reaction.user_id.map(|u| u.get().to_string()));
                    let _ = table.set("channel_id", removed_reaction.channel_id.get().to_string());
                    let _ = table.set("message_id", removed_reaction.message_id.get().to_string());
                    let _ = table.set("guild_id", removed_reaction.guild_id.map(|g| g.get().to_string()));
                    let _ = table.set("emoji", removed_reaction.emoji.to_string());

                    if let Err(e) = callback.call::<_, ()>(table) {
                        let _ = data.tui_tx.send(crate::types::BotEvent::Log(format!("❌ Lua Reaction Error: {}", e)));
                    }
                }
            };
        }
    }

    // Member join events
    if let serenity::FullEvent::GuildMemberAddition { new_member } = event {
        let _ = data.tui_tx.send(crate::types::BotEvent::UserJoined(new_member.user.name.clone()));
        
        let lua = data.lua.lock().unwrap();
        if let Ok(func) = lua.globals().get::<_, mlua::Function>("on_member_join") {
            let _ = func.call::<_, ()>((
                new_member.user.id.get().to_string(), 
                new_member.user.name.clone()
            ));
        };
    }

    // --- CACHE DISCORD DATA ---
    if let serenity::FullEvent::GuildCreate { guild, is_new: _ } = event {
        let mut state = data.discord_state.lock().unwrap();
        state.channels.clear();
        state.categories.clear();
        state.roles.clear();
        
        for (id, channel) in &guild.channels {
            if channel.kind == serenity::ChannelType::Text {
                state.channels.push((id.get().to_string(), channel.name.clone()));
            } else if channel.kind == serenity::ChannelType::Category {
                state.categories.push((id.get().to_string(), channel.name.clone()));
            }
        }
        
        for (id, role) in &guild.roles {
            let (r, g, b) = role.colour.tuple();
            state.roles.push(crate::types::DiscordRole {
                id: id.get().to_string(),
                name: role.name.clone(),
                color: (r, g, b),
            });
        }
        
        state.channels.sort_by(|a, b| a.1.cmp(&b.1));
        state.categories.sort_by(|a, b| a.1.cmp(&b.1));
        state.roles.sort_by(|a, b| a.name.cmp(&b.name));
        
        let _ = data.tui_tx.send(crate::types::BotEvent::Log(
            format!("📡 Cached {} channels, {} categories, {} roles.", 
                state.channels.len(), state.categories.len(), state.roles.len())
        ));
    }

    // 5. COMPONENT INTERACTION (Buttons/Select Menus)
    if let serenity::FullEvent::InteractionCreate { interaction } = event {
        if let serenity::Interaction::Component(comp) = interaction {
            let user_id = comp.user.id.get().to_string();
            let custom_id = comp.data.custom_id.clone();
            let channel_id = comp.channel_id.get().to_string();
            let guild_id = comp.guild_id.map(|g| g.get().to_string());

            let values: Vec<String> = match &comp.data.kind {
                serenity::ComponentInteractionDataKind::StringSelect { values } => values.clone(),
                _ => Vec::new(),
            };

            { 
                let ctx_lua = data.lua.lock().unwrap();
                
                if let Ok(callback) = ctx_lua.globals().get::<_, mlua::Function>("on_component") {
                    if let Ok(table) = ctx_lua.create_table() {
                        let _ = table.set("custom_id", custom_id);
                        let _ = table.set("user_id", user_id);
                        let _ = table.set("channel_id", channel_id);
                        let _ = table.set("values", values);
                        let _ = table.set("guild_id", guild_id);

                        let http = ctx.http.clone();
                        let interaction_id = comp.id;
                        let token = comp.token.clone();
                        
                        // CLONE THE TX FOR THE ASYNC REPLY BLOCK
                        let tx_reply = data.tui_tx.clone();
                        
                        table.set("reply", ctx_lua.create_function(move |_, (msg, ephemeral): (String, bool)| {
                            let h = http.clone();
                            let t = token.clone();
                            let tx = tx_reply.clone();
                            tokio::spawn(async move {
                                let data = serenity::CreateInteractionResponseMessage::new()
                                    .content(msg)
                                    .ephemeral(ephemeral);
                                let resp = serenity::CreateInteractionResponse::Message(data);
                                
                                if let Err(e) = h.create_interaction_response(interaction_id, &t, &resp, vec![]).await {
                                    let _ = tx.send(crate::types::BotEvent::Log(format!("Error replying to component: {}", e)));
                                }
                            });
                            Ok(())
                        })?)?;

                        if let Err(e) = callback.call::<_, ()>(table) {
                            let _ = data.tui_tx.send(crate::types::BotEvent::Log(format!("❌ Lua Component Error: {}", e)));
                        }
                    }
                };
            }
        }
    }

    Ok(())
}