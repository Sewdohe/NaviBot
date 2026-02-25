use crate::types::{Data, Error, LogLevel};
use poise::serenity_prelude as serenity;

pub async fn handle(guild: &serenity::Guild, data: &Data) -> Result<(), Error> {
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
        LogLevel::Info,
        format!(
            "Cached {} channels, {} categories, {} roles.",
            state.channels.len(),
            state.categories.len(),
            state.roles.len()
        ),
    ));

    Ok(())
}
