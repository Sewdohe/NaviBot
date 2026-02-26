use super::embed::build_reply_embed;
use crate::types::{Data, Error, LogLevel};
use poise::serenity_prelude as serenity;

pub async fn handle(ctx: &serenity::Context, comp: &serenity::ComponentInteraction, data: &Data) -> Result<(), Error> {
    let user_id = comp.user.id.get().to_string();
    let username = comp.user.name.clone();
    let custom_id = comp.data.custom_id.clone();
    let channel_id = comp.channel_id.get().to_string();
    let guild_id = comp.guild_id.map(|g| g.get().to_string());
    let member_roles: Vec<String> = comp
        .member
        .as_ref()
        .map(|m| m.roles.iter().map(|r| r.get().to_string()).collect())
        .unwrap_or_default();

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
                let _ = table.set("username", username);
                let _ = table.set("channel_id", channel_id);
                let _ = table.set("values", values);
                let _ = table.set("guild_id", guild_id);

                let roles_table = ctx_lua.create_table()?;
                for (i, role_id) in member_roles.iter().enumerate() {
                    roles_table.set(i + 1, role_id.clone())?;
                }
                let _ = table.set("member_roles", roles_table);

                let http = ctx.http.clone();
                let interaction_id = comp.id;
                let token = comp.token.clone();

                let reply_embed_http = ctx.http.clone();
                let reply_embed_token = token.clone();
                let modal_http = http.clone();
                let modal_token = token.clone();
                let modal_id = interaction_id;
                let modal_tx = data.tui_tx.clone();

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
                            let _ = tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Error replying to component: {}", e)));
                        }
                    });
                    Ok(())
                })?)?;

                let reply_embed_tx = data.tui_tx.clone();
                if let Ok(reply_embed_fn) = ctx_lua.create_function(move |_, (data, ephemeral): (mlua::Table, Option<bool>)| {
                    let msg = build_reply_embed(&data, ephemeral.unwrap_or(false))?;
                    let http = reply_embed_http.clone();
                    let tok = reply_embed_token.clone();
                    let tx = reply_embed_tx.clone();
                    tokio::spawn(async move {
                        let response = serenity::CreateInteractionResponse::Message(msg);
                        if let Err(e) = http.create_interaction_response(interaction_id, &tok, &response, vec![]).await {
                            let _ = tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("component reply_embed error: {}", e)));
                        }
                    });
                    Ok(())
                }) {
                    let _ = table.set("reply_embed", reply_embed_fn);
                }

                if let Ok(modal_fn) = ctx_lua.create_function(move |_, (custom_id, title, fields): (String, String, mlua::Table)| {
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
                }) {
                    let _ = table.set("modal", modal_fn);
                }

                if let Err(e) = callback.call::<_, ()>(table) {
                    let _ = data.tui_tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Lua Component Error: {}", e)));
                }
            }
        };
    }

    Ok(())
}
