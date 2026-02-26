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

                let reply_fn = {
                    let http = http.clone();
                    let token = interaction_token.clone();
                    let tx = tx_reply.clone();
                    lua.create_function(move |_, (msg, ephemeral): (String, Option<bool>)| {
                        let http = http.clone();
                        let token = token.clone();
                        let id = interaction_id;
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            let msg = serenity::CreateInteractionResponseMessage::new()
                                .content(msg)
                                .ephemeral(ephemeral.unwrap_or(false));
                            let response = serenity::CreateInteractionResponse::Message(msg);
                            if let Err(e) = http.create_interaction_response(id, &token, &response, vec![]).await {
                                let _ = tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Error replying: {}", e)));
                            }
                        });
                        Ok(())
                    })
                };

                let reply_embed_fn = {
                    let http = http.clone();
                    let token = interaction_token.clone();
                    let tx = tx_reply.clone();
                    lua.create_function(move |_, (data, ephemeral): (mlua::Table, Option<bool>)| {
                        let http = http.clone();
                        let token = token.clone();
                        let id = interaction_id;
                        let tx = tx.clone();

                        let title: Option<String> = data.get("title").ok();
                        let description: Option<String> = data.get("description").ok();
                        let color: Option<u32> = data.get("color").ok();
                        let image_url: Option<String> = data.get::<_, String>("image").ok().filter(|s| !s.is_empty());
                        let mut fields = Vec::new();
                        if let Ok(lua_fields) = data.get::<_, Vec<mlua::Table>>("fields") {
                            for f in lua_fields {
                                let name: String = f.get("name").unwrap_or_default();
                                let value: String = f.get("value").unwrap_or_default();
                                let inline: bool = f.get("inline").unwrap_or(false);
                                fields.push((name, value, inline));
                            }
                        }

                        let mut action_rows: Vec<serenity::CreateActionRow> = Vec::new();
                        let mut current_buttons: Vec<serenity::CreateButton> = Vec::new();
                        if let Ok(comps) = data.get::<_, Vec<mlua::Table>>("components") {
                            for c in comps {
                                let c_type: String = c.get("type").unwrap_or("button".into());
                                if c_type == "button" {
                                    let label: String = c.get("label").unwrap_or("Button".into());
                                    let style_str: String = c.get("style").unwrap_or("primary".into());
                                    if style_str == "link" || style_str == "url" {
                                        let url: String = c.get("url").unwrap_or_default();
                                        current_buttons.push(serenity::CreateButton::new_link(url).label(label));
                                    } else {
                                        let custom_id: String = c.get("id").unwrap_or("unknown".into());
                                        let style = match style_str.as_str() {
                                            "secondary" | "gray" => serenity::ButtonStyle::Secondary,
                                            "success" | "green" => serenity::ButtonStyle::Success,
                                            "danger" | "red" => serenity::ButtonStyle::Danger,
                                            _ => serenity::ButtonStyle::Primary,
                                        };
                                        current_buttons.push(serenity::CreateButton::new(custom_id).style(style).label(label));
                                    }
                                } else if c_type == "select" {
                                    if !current_buttons.is_empty() {
                                        action_rows.push(serenity::CreateActionRow::Buttons(current_buttons.clone()));
                                        current_buttons.clear();
                                    }
                                    let custom_id: String = c.get("id").unwrap_or("select_menu".into());
                                    let placeholder: String = c.get("placeholder").unwrap_or("Select an option...".into());
                                    let mut options = Vec::new();
                                    if let Ok(lua_opts) = c.get::<_, Vec<mlua::Table>>("options") {
                                        for opt in lua_opts {
                                            let lbl: String = opt.get("label").unwrap_or("Option".into());
                                            let val: String = opt.get("value").unwrap_or(lbl.clone());
                                            let mut builder = serenity::CreateSelectMenuOption::new(lbl, val);
                                            if let Ok(desc) = opt.get::<_, String>("description") { builder = builder.description(desc); }
                                            options.push(builder);
                                        }
                                    }
                                    let menu = serenity::CreateSelectMenu::new(custom_id, serenity::CreateSelectMenuKind::String { options }).placeholder(placeholder);
                                    action_rows.push(serenity::CreateActionRow::SelectMenu(menu));
                                }
                            }
                        }
                        if !current_buttons.is_empty() {
                            action_rows.push(serenity::CreateActionRow::Buttons(current_buttons));
                        }

                        tokio::spawn(async move {
                            let mut embed = serenity::CreateEmbed::new();
                            if let Some(t) = title { embed = embed.title(t); }
                            if let Some(d) = description { embed = embed.description(d); }
                            if let Some(c) = color { embed = embed.color(serenity::Color::new(c)); }
                            for (n, v, i) in fields { embed = embed.field(n, v, i); }
                            if let Some(img) = image_url { embed = embed.image(img); }

                            let mut msg = serenity::CreateInteractionResponseMessage::new()
                                .embed(embed)
                                .ephemeral(ephemeral.unwrap_or(false));
                            if !action_rows.is_empty() { msg = msg.components(action_rows); }

                            let response = serenity::CreateInteractionResponse::Message(msg);
                            if let Err(e) = http.create_interaction_response(id, &token, &response, vec![]).await {
                                let _ = tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Error sending embed reply: {}", e)));
                            }
                        });
                        Ok(())
                    })
                };

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
                    if let Ok(re) = reply_embed_fn {
                        let _ = ctx_table.set("reply_embed", re);
                    }
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
