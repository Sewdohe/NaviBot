use crate::types::{Data, Error, LogLevel};
use poise::serenity_prelude as serenity;

pub async fn handle(member: &serenity::Member, data: &Data) -> Result<(), Error> {
    let _ = data.tui_tx.send(crate::types::BotEvent::Log(
        LogLevel::Info,
        format!("👋 User Joined: {}", member.user.name),
    ));

    let lua = data.lua.lock().unwrap();
    if let Ok(func) = lua.globals().get::<_, mlua::Function>("on_member_join") {
        if let Err(e) = func.call::<_, ()>((
            member.user.id.get().to_string(),
            member.user.name.clone(),
            member.guild_id.get().to_string(),
        )) {
            let _ = data.tui_tx.send(crate::types::BotEvent::Log(
                LogLevel::Error,
                format!("Greeter Crash: {}", e),
            ));
        }
    };

    Ok(())
}
