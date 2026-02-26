use crate::types::{Data, Error, LogLevel};
use mlua::{Function, Table};
use poise::serenity_prelude as serenity;

enum ArgValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Number(f64),
}

pub async fn handle(ctx: &serenity::Context, command: &serenity::CommandInteraction, data: &Data) -> Result<(), Error> {
    // Detect whether this is a subcommand interaction (grouped plugin) or a flat command.
    // For grouped commands Discord sends: top-level name = plugin, options[0] = SubCommand.
    let (cmd_name, effective_options): (String, Vec<serenity::CommandDataOption>) =
        if let Some(first) = command.data.options.first() {
            if let serenity::CommandDataOptionValue::SubCommand(sub_opts) = &first.value {
                (first.name.clone(), sub_opts.clone())
            } else {
                (command.data.name.clone(), command.data.options.clone())
            }
        } else {
            (command.data.name.clone(), command.data.options.clone())
        };

    let user_id = command.user.id.get();
    let username = command.user.name.clone();
    let channel_id = command.channel_id.get().to_string();
    let guild_id = command.guild_id.map(|g| g.get().to_string());
    let member_roles: Vec<String> = command
        .member
        .as_ref()
        .map(|m| m.roles.iter().map(|r| r.get().to_string()).collect())
        .unwrap_or_default();

    let mut args_map: Vec<(String, ArgValue)> = Vec::new();

    for option in &effective_options {
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

    let tx_reply = data.tui_tx.clone();

    let modal_http = http.clone();
    let modal_token = interaction_token.clone();
    let modal_id = interaction_id;
    let modal_tx = tx_reply.clone();

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

                let roles_table = lua.create_table()?;
                for (i, role_id) in member_roles.iter().enumerate() {
                    roles_table.set(i + 1, role_id.clone())?;
                }
                let _ = ctx_table.set("member_roles", roles_table);

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
                            let _ = tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Error replying: {}", e)));
                        }
                    });
                    Ok(())
                });

                let modal_fn = lua.create_function(move |_, (custom_id, title, fields): (String, String, mlua::Table)| {
                    let mut rows = Vec::new();
                    for pair in fields.pairs::<mlua::Integer, mlua::Table>().flatten() {
                        let f = pair.1;
                        let field_id: String = f.get("id").unwrap_or_default();
                        let label: String = f.get("label").unwrap_or_default();
                        let placeholder: String = f.get("placeholder").unwrap_or_default();
                        let required: bool = f.get("required").unwrap_or(true);
                        let style_str: String = f.get("style").unwrap_or_else(|_| "short".into());
                        let style = if style_str == "paragraph" {
                            serenity::InputTextStyle::Paragraph
                        } else {
                            serenity::InputTextStyle::Short
                        };
                        rows.push(serenity::CreateActionRow::InputText(
                            serenity::CreateInputText::new(style, label, field_id)
                                .placeholder(placeholder)
                                .required(required)
                        ));
                    }
                    let modal = serenity::CreateModal::new(custom_id, title).components(rows);
                    let response = serenity::CreateInteractionResponse::Modal(modal);
                    let http = modal_http.clone();
                    let token = modal_token.clone();
                    let tx = modal_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = http.create_interaction_response(modal_id, &token, &response, vec![]).await {
                            let _ = tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Modal open error: {}", e)));
                        }
                    });
                    Ok(())
                });

                if let Ok(reply) = reply_fn {
                    let _ = ctx_table.set("reply", reply);
                    if let Ok(modal) = modal_fn {
                        let _ = ctx_table.set("modal", modal);
                    }
                    if let Err(e) = func.call::<_, ()>(ctx_table) {
                        let _ = data.tui_tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Lua Slash Error: {}", e)));
                    }
                }
            }
        }
    }

    Ok(())
}
