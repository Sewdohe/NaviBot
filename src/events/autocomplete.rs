use crate::types::{Data, Error, LogLevel};
use poise::serenity_prelude as serenity;

pub async fn handle(
    ctx: &serenity::Context,
    interaction: &serenity::CommandInteraction,
    data: &Data,
) -> Result<(), Error> {
    // Mirror the slash.rs subcommand-vs-flat logic to find the command name and focused option.
    let (cmd_name, effective_options): (String, Vec<serenity::CommandDataOption>) =
        if let Some(first) = interaction.data.options.first() {
            if let serenity::CommandDataOptionValue::SubCommand(sub_opts) = &first.value {
                (first.name.clone(), sub_opts.clone())
            } else {
                (interaction.data.name.clone(), interaction.data.options.clone())
            }
        } else {
            return Ok(());
        };

    // Find the focused option — it carries Autocomplete { value } instead of a resolved value.
    let (focused_name, current_value) = match effective_options.iter().find_map(|opt| {
        if let serenity::CommandDataOptionValue::Autocomplete { value, .. } = &opt.value {
            Some((opt.name.clone(), value.clone()))
        } else {
            None
        }
    }) {
        Some(pair) => pair,
        None => return Ok(()),
    };

    // Call the Lua handler and collect choices.
    let choices: Vec<(String, String)> = {
        let lua = data.lua.lock().unwrap();

        let handler: Option<mlua::Function> = (|| {
            let navi: mlua::Table = lua.globals().get("navi").ok()?;
            let ac: mlua::Table = navi.get("autocomplete_handlers").ok()?;
            let cmd_handlers: mlua::Table = ac.get(cmd_name.as_str()).ok()?;
            cmd_handlers.get(focused_name.as_str()).ok()
        })();

        match handler {
            None => return Ok(()),
            Some(func) => {
                let ctx_table = lua.create_table()?;
                ctx_table.set("current_value", current_value)?;
                ctx_table.set("user_id", interaction.user.id.get().to_string())?;
                ctx_table.set("guild_id", interaction.guild_id.map(|g| g.get().to_string()))?;

                match func.call::<_, mlua::Value>(ctx_table) {
                    Err(e) => {
                        let _ = data.tui_tx.send(crate::types::BotEvent::Log(
                            LogLevel::Error,
                            format!("Lua autocomplete error: {}", e),
                        ));
                        return Ok(());
                    }
                    Ok(mlua::Value::Table(results)) => {
                        let mut out = Vec::new();
                        for pair in results.pairs::<mlua::Integer, mlua::Table>().flatten() {
                            let row = pair.1;
                            let name: String = row.get("name").unwrap_or_default();
                            let value: String = row
                                .get::<_, String>("value")
                                .unwrap_or_else(|_| name.clone());
                            out.push((name, value));
                        }
                        out
                    }
                    Ok(_) => return Ok(()),
                }
            }
        }
    };

    // Respond to Discord with the choices (max 25).
    let mut ac_response = serenity::CreateAutocompleteResponse::new();
    for (name, value) in choices.into_iter().take(25) {
        ac_response = ac_response.add_string_choice(name, value);
    }
    let response = serenity::CreateInteractionResponse::Autocomplete(ac_response);
    if let Err(e) = ctx
        .http
        .create_interaction_response(interaction.id, &interaction.token, &response, vec![])
        .await
    {
        let _ = data.tui_tx.send(crate::types::BotEvent::Log(
            LogLevel::Error,
            format!("Failed to send autocomplete response: {}", e),
        ));
    }

    Ok(())
}
