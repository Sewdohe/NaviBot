mod autocomplete;
mod component;
mod guild_cache;
mod member;
mod message;
mod modal;
mod reaction;
mod slash;
mod voice;

use crate::types::{Data, Error};
use poise::serenity_prelude as serenity;

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Message { new_message } if !new_message.author.bot => {
            message::handle(ctx, new_message, data).await?;
        }
        serenity::FullEvent::InteractionCreate { interaction } => match interaction {
            serenity::Interaction::Command(cmd) => slash::handle(ctx, cmd, data).await?,
            serenity::Interaction::Autocomplete(ac) => autocomplete::handle(ctx, ac, data).await?,
            serenity::Interaction::Component(comp) => component::handle(ctx, comp, data).await?,
            serenity::Interaction::Modal(modal_submit) => modal::handle(ctx, modal_submit, data).await?,
            _ => {}
        },
        serenity::FullEvent::ReactionAdd { add_reaction } => {
            reaction::handle_add(add_reaction, data).await?;
        }
        serenity::FullEvent::ReactionRemove { removed_reaction } => {
            reaction::handle_remove(removed_reaction, data).await?;
        }
        serenity::FullEvent::GuildMemberAddition { new_member } => {
            member::handle(new_member, data).await?;
        }
        serenity::FullEvent::GuildMemberRemoval { guild_id, user, .. } => {
            member::handle_leave(user, *guild_id, data).await?;
        }
        serenity::FullEvent::MessageUpdate { event, .. } => {
            message::handle_edit(event, data).await?;
        }
        serenity::FullEvent::MessageDelete { channel_id, deleted_message_id, guild_id } => {
            message::handle_delete(*channel_id, *deleted_message_id, *guild_id, data).await?;
        }
        serenity::FullEvent::VoiceStateUpdate { new, .. } => {
            voice::handle(new, data).await?;
        }
        serenity::FullEvent::GuildCreate { guild, is_new: _ } => {
            guild_cache::handle(guild, data).await?;
        }
        serenity::FullEvent::GuildUpdate { new_data, .. } => {
            data.discord_state
                .lock()
                .unwrap()
                .guild_owners
                .insert(new_data.id.to_string(), new_data.owner_id.to_string());
        }
        _ => {}
    }
    Ok(())
}
