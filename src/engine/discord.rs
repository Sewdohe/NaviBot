use mlua::prelude::*;
use poise::serenity_prelude as serenity;
use tokio::sync::mpsc::UnboundedSender;
use crate::types::{BotEvent, LogLevel, SharedDiscordState};

pub fn register(
    lua: &Lua,
    navi: &LuaTable,
    ctx: &serenity::Context,
    tui_tx: UnboundedSender<BotEvent>,
    discord_state: SharedDiscordState,
) -> LuaResult<()> {
    // --- CREATE CHANNEL ---
    let ctx_channel = ctx.clone();
    navi.set(
        "create_channel",
        lua.create_function(
            move |_, (guild_id, name, options): (String, String, mlua::Table)| {
                let http = ctx_channel.http.clone();

                let category_id: Option<String> = options.get("category_id").unwrap_or(None);
                let user_id: Option<String> = options.get("user_id").unwrap_or(None);
                let role_id: Option<String> = options.get("role_id").unwrap_or(None);
                let welcome_message: Option<String> =
                    options.get("welcome_message").unwrap_or(None);
                let close_button: Option<bool> = options.get("close_button").unwrap_or(None);

                tokio::spawn(async move {
                    use poise::serenity_prelude as serenity;
                    let gid = serenity::GuildId::new(guild_id.parse().unwrap_or(0));

                    let mut builder =
                        serenity::CreateChannel::new(&name).kind(serenity::ChannelType::Text);

                    if let Some(cid_str) = category_id {
                        if let Ok(cid) = cid_str.parse::<u64>() {
                            builder = builder.category(serenity::ChannelId::new(cid));
                        }
                    }

                    if user_id.is_some() || role_id.is_some() {
                        let everyone_id = serenity::RoleId::new(gid.get());
                        let mut perms = vec![serenity::PermissionOverwrite {
                            allow: serenity::Permissions::empty(),
                            deny: serenity::Permissions::VIEW_CHANNEL,
                            kind: serenity::PermissionOverwriteType::Role(everyone_id),
                        }];

                        if let Some(uid_str) = user_id {
                            if let Ok(uid) = uid_str.parse::<u64>() {
                                perms.push(serenity::PermissionOverwrite {
                                    allow: serenity::Permissions::VIEW_CHANNEL
                                        | serenity::Permissions::SEND_MESSAGES,
                                    deny: serenity::Permissions::empty(),
                                    kind: serenity::PermissionOverwriteType::Member(
                                        serenity::UserId::new(uid),
                                    ),
                                });
                            }
                        }

                        if let Some(rid_str) = role_id {
                            if let Ok(rid) = rid_str.parse::<u64>() {
                                perms.push(serenity::PermissionOverwrite {
                                    allow: serenity::Permissions::VIEW_CHANNEL
                                        | serenity::Permissions::SEND_MESSAGES,
                                    deny: serenity::Permissions::empty(),
                                    kind: serenity::PermissionOverwriteType::Role(
                                        serenity::RoleId::new(rid),
                                    ),
                                });
                            }
                        }

                        builder = builder.permissions(perms);
                    }

                    if let Ok(channel) = gid.create_channel(&http, builder).await {
                        if let Some(msg_text) = welcome_message {
                            let mut msg = serenity::CreateMessage::new().content(msg_text);

                            if close_button.unwrap_or(false) {
                                let action_row = serenity::CreateActionRow::Buttons(vec![
                                    serenity::CreateButton::new("btn_close_ticket")
                                        .label("🔒 Close Ticket")
                                        .style(serenity::ButtonStyle::Danger),
                                ]);
                                msg = msg.components(vec![action_row]);
                            }

                            let _ = channel.id.send_message(&http, msg).await;
                        }
                    }
                });
                Ok(())
            },
        )?,
    )?;

    // --- DELETE CHANNEL ---
    let ctx_del = ctx.clone();
    navi.set(
        "delete_channel",
        lua.create_function(move |_, channel_id: String| {
            let http = ctx_del.http.clone();
            tokio::spawn(async move {
                use poise::serenity_prelude as serenity;
                if let Ok(cid) = channel_id.parse::<u64>() {
                    let _ = serenity::ChannelId::new(cid).delete(&http).await;
                }
            });
            Ok(())
        })?,
    )?;

    // --- ADD ROLE ---
    let http_add_role = ctx.http.clone();
    let tx_add_role = tui_tx.clone();
    navi.set(
        "add_role",
        lua.create_function(
            move |_, (guild_id, user_id, role_id): (String, String, String)| {
                let http = http_add_role.clone();
                let tx = tx_add_role.clone();
                tokio::spawn(async move {
                    let g_id = serenity::GuildId::new(guild_id.parse().unwrap_or(0));
                    let u_id = serenity::UserId::new(user_id.parse().unwrap_or(0));
                    let r_id = serenity::RoleId::new(role_id.parse().unwrap_or(0));

                    if let Err(e) = http.add_member_role(g_id, u_id, r_id, None).await {
                        let _ = tx.send(BotEvent::Log(LogLevel::Error, format!("Failed to add role: {}", e)));
                    }
                });
                Ok(())
            },
        )?,
    )?;

    // --- REMOVE ROLE ---
    let http_remove_role = ctx.http.clone();
    let tx_remove_role = tui_tx.clone();
    navi.set(
        "remove_role",
        lua.create_function(
            move |_, (guild_id, user_id, role_id): (String, String, String)| {
                let http = http_remove_role.clone();
                let tx = tx_remove_role.clone();
                tokio::spawn(async move {
                    let g_id = serenity::GuildId::new(guild_id.parse().unwrap_or(0));
                    let u_id = serenity::UserId::new(user_id.parse().unwrap_or(0));
                    let r_id = serenity::RoleId::new(role_id.parse().unwrap_or(0));

                    if let Err(e) = http.remove_member_role(g_id, u_id, r_id, None).await {
                        let _ = tx.send(BotEvent::Log(LogLevel::Error, format!("Failed to remove role: {}", e)));
                    }
                });
                Ok(())
            },
        )?,
    )?;

    // --- KICK ---
    let http_kick = ctx.http.clone();
    navi.set("kick", lua.create_function(move |_, (guild_id, user_id, reason): (String, String, Option<String>)| {
        let http = http_kick.clone();
        tokio::spawn(async move {
            let g_id = serenity::GuildId::new(guild_id.parse().unwrap_or(0));
            let u_id = serenity::UserId::new(user_id.parse().unwrap_or(0));
            let _ = http.kick_member(g_id, u_id, reason.as_deref()).await;
        });
        Ok(())
    })?)?;

    // --- BAN ---
    let http_ban = ctx.http.clone();
    navi.set("ban", lua.create_function(move |_, (guild_id, user_id, dmd, reason): (String, String, u8, Option<String>)| {
        let http = http_ban.clone();
        tokio::spawn(async move {
            let g_id = serenity::GuildId::new(guild_id.parse().unwrap_or(0));
            let u_id = serenity::UserId::new(user_id.parse().unwrap_or(0));
            let _ = http.ban_user(g_id, u_id, dmd, reason.as_deref()).await;
        });
        Ok(())
    })?)?;

    // --- UNBAN ---
    let http_unban = ctx.http.clone();
    navi.set("unban", lua.create_function(move |_, (guild_id, user_id): (String, String)| {
        let http = http_unban.clone();
        tokio::spawn(async move {
            let g_id = serenity::GuildId::new(guild_id.parse().unwrap_or(0));
            let u_id = serenity::UserId::new(user_id.parse().unwrap_or(0));
            let _ = http.remove_ban(g_id, u_id, None).await;
        });
        Ok(())
    })?)?;

    // --- TIMEOUT ---
    let http_timeout = ctx.http.clone();
    navi.set("timeout", lua.create_function(move |_, (guild_id, user_id, secs): (String, String, u64)| {
        let http = http_timeout.clone();
        tokio::spawn(async move {
            let g_id = serenity::GuildId::new(guild_id.parse().unwrap_or(0));
            let u_id = serenity::UserId::new(user_id.parse().unwrap_or(0));
            let edit = if secs == 0 {
                serenity::EditMember::new().enable_communication()
            } else {
                let until_ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + secs;
                let ts = serenity::Timestamp::from_unix_timestamp(until_ts as i64)
                    .unwrap_or(serenity::Timestamp::now());
                serenity::EditMember::new().disable_communication_until(ts.to_string())
            };
            let _ = g_id.edit_member(&http, u_id, edit).await;
        });
        Ok(())
    })?)?;

    // --- DISCORD CACHE BINDINGS ---
    let state_roles = discord_state.clone();
    navi.set(
        "get_roles",
        lua.create_function(move |lua, _guild_id: mlua::Value| {
            let state = state_roles.lock().unwrap();
            let result = lua.create_table()?;
            for (i, role) in state.roles.iter().enumerate() {
                let t = lua.create_table()?;
                t.set("id", role.id.clone())?;
                t.set("name", role.name.clone())?;
                let color_t = lua.create_table()?;
                color_t.set(1, role.color.0)?;
                color_t.set(2, role.color.1)?;
                color_t.set(3, role.color.2)?;
                t.set("color", color_t)?;
                result.set(i + 1, t)?;
            }
            Ok(result)
        })?,
    )?;

    let state_channels = discord_state.clone();
    navi.set(
        "get_channels",
        lua.create_function(move |lua, _guild_id: mlua::Value| {
            let state = state_channels.lock().unwrap();
            let result = lua.create_table()?;
            for (i, (id, name)) in state.channels.iter().enumerate() {
                let t = lua.create_table()?;
                t.set("id", id.clone())?;
                t.set("name", name.clone())?;
                result.set(i + 1, t)?;
            }
            Ok(result)
        })?,
    )?;

    Ok(())
}
