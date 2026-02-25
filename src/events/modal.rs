use crate::types::{Data, Error, LogLevel};
use poise::serenity_prelude as serenity;

pub async fn handle(ctx: &serenity::Context, modal_submit: &serenity::ModalInteraction, data: &Data) -> Result<(), Error> {
    let user_id = modal_submit.user.id.get().to_string();
    let username = modal_submit.user.name.clone();
    let custom_id = modal_submit.data.custom_id.clone();
    let channel_id = modal_submit.channel_id.get().to_string();
    let guild_id = modal_submit.guild_id.map(|g| g.get().to_string());
    let member_roles: Vec<String> = modal_submit
        .member
        .as_ref()
        .map(|m| m.roles.iter().map(|r| r.get().to_string()).collect())
        .unwrap_or_default();

    let mut field_values: Vec<(String, String)> = Vec::new();
    for row in &modal_submit.data.components {
        for comp in &row.components {
            if let serenity::ActionRowComponent::InputText(input) = comp {
                field_values.push((
                    input.custom_id.clone(),
                    input.value.clone().unwrap_or_default(),
                ));
            }
        }
    }

    let http = ctx.http.clone();
    let interaction_id = modal_submit.id;
    let token = modal_submit.token.clone();
    let tx_reply = data.tui_tx.clone();

    {
        let lua = data.lua.lock().unwrap();
        if let Ok(callback) = lua.globals().get::<_, mlua::Function>("on_modal_submit") {
            let table = lua.create_table()?;
            let _ = table.set("custom_id", custom_id);
            let _ = table.set("user_id", user_id);
            let _ = table.set("username", username);
            let _ = table.set("channel_id", channel_id);
            let _ = table.set("guild_id", guild_id);

            let roles_table = lua.create_table()?;
            for (i, role_id) in member_roles.iter().enumerate() {
                roles_table.set(i + 1, role_id.clone())?;
            }
            let _ = table.set("member_roles", roles_table);

            let values_table = lua.create_table()?;
            for (k, v) in field_values {
                let _ = values_table.set(k, v);
            }
            let _ = table.set("values", values_table);

            let reply_http = http.clone();
            let reply_token = token.clone();
            let reply_tx = tx_reply.clone();
            table.set("reply", lua.create_function(move |_, (msg, ephemeral): (String, bool)| {
                let h = reply_http.clone();
                let t = reply_token.clone();
                let tx = reply_tx.clone();
                tokio::spawn(async move {
                    let data = serenity::CreateInteractionResponseMessage::new()
                        .content(msg)
                        .ephemeral(ephemeral);
                    let resp = serenity::CreateInteractionResponse::Message(data);
                    if let Err(e) = h.create_interaction_response(interaction_id, &t, &resp, vec![]).await {
                        let _ = tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Modal reply error: {}", e)));
                    }
                });
                Ok(())
            })?)?;

            if let Err(e) = callback.call::<_, ()>(table) {
                let _ = data.tui_tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Lua Modal Error: {}", e)));
            }
        };
    }

    Ok(())
}
