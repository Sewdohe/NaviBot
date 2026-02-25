use crate::types::{Data, Error, LogLevel};
use mlua::Function;
use poise::serenity_prelude as serenity;

pub async fn handle(_ctx: &serenity::Context, msg: &serenity::Message, data: &Data) -> Result<(), Error> {
    let content = msg.content.clone();
    let author = msg.author.name.clone();
    let channel_id = msg.channel_id.get();
    let guild_id = msg.guild_id.map(|g| g.get().to_string());

    let lua = data.lua.lock().unwrap();

    let func: Option<Function> = match lua.globals().get("on_message") {
        Ok(f) => Some(f),
        Err(_) => None,
    };

    if let Some(f) = func {
        let msg_table = lua.create_table()?;
        msg_table.set("content", content)?;
        msg_table.set("channel_id", channel_id)?;
        msg_table.set("message_id", msg.id.get())?;
        msg_table.set("author", author)?;
        msg_table.set("author_id", msg.author.id.get())?;
        msg_table.set("author_avatar", msg.author.face())?;
        msg_table.set("guild_id", guild_id)?;

        let mentions = lua.create_table()?;
        for (i, user) in msg.mentions.iter().enumerate() {
            let u = lua.create_table()?;
            u.set("name", user.name.clone())?;
            u.set("id", user.id.get())?;
            u.set("avatar", user.face())?;
            mentions.set(i + 1, u)?;
        }
        msg_table.set("mentions", mentions)?;

        let attachments = lua.create_table()?;
        for (i, attachment) in msg.attachments.iter().enumerate() {
            attachments.set(i + 1, attachment.url.clone())?;
        }
        msg_table.set("attachments", attachments)?;

        if let Err(e) = f.call::<_, ()>(msg_table) {
            let _ = data.tui_tx.send(crate::types::BotEvent::Log(LogLevel::Error, format!("Lua Error: {}", e)));
        }
    }

    Ok(())
}
