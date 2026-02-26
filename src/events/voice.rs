use crate::types::{Data, Error, LogLevel};
use poise::serenity_prelude as serenity;

pub async fn handle(new: &serenity::VoiceState, data: &Data) -> Result<(), Error> {
    let lua = data.lua.lock().unwrap();
    if let Ok(func) = lua.globals().get::<_, mlua::Function>("on_voice_state_update") {
        let t = lua.create_table()?;
        t.set("user_id", new.user_id.get().to_string())?;
        t.set("guild_id", new.guild_id.map(|g| g.get().to_string()))?;
        t.set("channel_id", new.channel_id.map(|c| c.get().to_string()))?;
        t.set("self_mute", new.self_mute)?;
        t.set("self_deaf", new.self_deaf)?;
        t.set("self_stream", new.self_stream.unwrap_or(false))?;
        t.set("self_video", new.self_video)?;
        if let Err(e) = func.call::<_, ()>(t) {
            let _ = data.tui_tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Lua on_voice_state_update error: {}", e)));
        }
    }
    Ok(())
}
