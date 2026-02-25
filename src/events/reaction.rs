use crate::types::{Data, Error, LogLevel};
use poise::serenity_prelude as serenity;

pub async fn handle_add(reaction: &serenity::Reaction, data: &Data) -> Result<(), Error> {
    let ctx_lua = data.lua.lock().unwrap();

    if let Ok(callback) = ctx_lua.globals().get::<_, mlua::Function>("on_reaction_add") {
        if let Ok(table) = ctx_lua.create_table() {
            let _ = table.set("user_id", reaction.user_id.map(|u| u.get().to_string()));
            let _ = table.set("channel_id", reaction.channel_id.get().to_string());
            let _ = table.set("message_id", reaction.message_id.get().to_string());
            let _ = table.set("guild_id", reaction.guild_id.map(|g| g.get().to_string()));
            let _ = table.set("emoji", reaction.emoji.to_string());

            if let Err(e) = callback.call::<_, ()>(table) {
                let _ = data.tui_tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Lua Reaction Error: {}", e)));
            }
        }
    };

    Ok(())
}

pub async fn handle_remove(reaction: &serenity::Reaction, data: &Data) -> Result<(), Error> {
    let ctx_lua = data.lua.lock().unwrap();

    if let Ok(callback) = ctx_lua.globals().get::<_, mlua::Function>("on_reaction_remove") {
        if let Ok(table) = ctx_lua.create_table() {
            let _ = table.set("user_id", reaction.user_id.map(|u| u.get().to_string()));
            let _ = table.set("channel_id", reaction.channel_id.get().to_string());
            let _ = table.set("message_id", reaction.message_id.get().to_string());
            let _ = table.set("guild_id", reaction.guild_id.map(|g| g.get().to_string()));
            let _ = table.set("emoji", reaction.emoji.to_string());

            if let Err(e) = callback.call::<_, ()>(table) {
                let _ = data.tui_tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Lua Reaction Error: {}", e)));
            }
        }
    };

    Ok(())
}
