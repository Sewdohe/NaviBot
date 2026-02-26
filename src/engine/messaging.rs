use mlua::prelude::*;
use poise::serenity_prelude as serenity;
use tokio::sync::mpsc::UnboundedSender;
use crate::types::{BotEvent, LogLevel};
use reqwest::Client as ReqwestClient;

fn parse_color(c: Option<u32>) -> serenity::Color {
    match c {
        Some(val) => serenity::Color::new(val),
        None => serenity::Color::new(0x000000),
    }
}

pub fn register(
    lua: &Lua,
    navi: &LuaTable,
    ctx: &serenity::Context,
    tui_tx: UnboundedSender<BotEvent>,
) -> LuaResult<()> {
    // --- HTTP ---
    let http_client = ReqwestClient::new();
    let http_table = lua.create_table()?;

    let c = http_client.clone();
    http_table.set("get", lua.create_function(move |lua, (url, headers): (String, Option<mlua::Table>)| {
        let mut req = c.get(&url);
        if let Some(h) = headers {
            for pair in h.pairs::<String, String>().flatten() {
                req = req.header(pair.0.as_str(), pair.1.as_str());
            }
        }
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move { req.send().await?.text().await })
        });
        match result {
            Ok(body) => Ok(mlua::Value::String(lua.create_string(&body)?)),
            Err(e)   => { eprintln!("[navi.http.get] {e}"); Ok(mlua::Value::Nil) }
        }
    })?)?;

    let c = http_client.clone();
    http_table.set("post", lua.create_function(move |lua, (url, body, headers): (String, String, Option<mlua::Table>)| {
        let mut req = c.post(&url).body(body);
        if let Some(h) = headers {
            for pair in h.pairs::<String, String>().flatten() {
                req = req.header(pair.0.as_str(), pair.1.as_str());
            }
        }
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move { req.send().await?.text().await })
        });
        match result {
            Ok(body) => Ok(mlua::Value::String(lua.create_string(&body)?)),
            Err(e)   => { eprintln!("[navi.http.post] {e}"); Ok(mlua::Value::Nil) }
        }
    })?)?;

    navi.set("http", http_table)?;

    // --- SAY ---
    let http_say = ctx.http.clone();
    let tx_say = tui_tx.clone();
    navi.set("say", lua.create_function(move |_, (channel_id, text): (u64, String)| {
        let http = http_say.clone();
        let tx = tx_say.clone();
        tokio::spawn(async move {
            let channel = serenity::ChannelId::new(channel_id);
            if let Err(e) = channel.say(&http, text).await {
                let _ = tx.send(BotEvent::Log(LogLevel::Error, format!("Error sending message: {}", e)));
            }
        });
        Ok(())
    })?)?;

    // --- SAY SYNC (returns message ID) ---
    let http_say_sync = ctx.http.clone();
    navi.set("say_sync", lua.create_function(move |lua, (channel_id, text): (String, String)| {
        let http = http_say_sync.clone();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let c_id = serenity::ChannelId::new(channel_id.parse().unwrap_or(0));
                c_id.say(&http, text).await
            })
        });
        match result {
            Ok(msg) => Ok(mlua::Value::String(lua.create_string(&msg.id.get().to_string())?)),
            Err(_) => Ok(mlua::Value::Nil),
        }
    })?)?;

    // --- BOT PRESENCE / STATUS ---
    let ctx_status = ctx.clone();
    navi.set(
        "set_status",
        lua.create_function(move |_, (activity_type, text): (String, String)| {
            let ctx = ctx_status.clone();
            let activity = match activity_type.to_lowercase().as_str() {
                "playing" => Some(serenity::ActivityData::playing(text)),
                "listening" => Some(serenity::ActivityData::listening(text)),
                "watching" => Some(serenity::ActivityData::watching(text)),
                "competing" => Some(serenity::ActivityData::competing(text)),
                "custom" => Some(serenity::ActivityData::custom(text)),
                _ => None,
            };
            ctx.set_activity(activity);
            Ok(())
        })?,
    )?;

    // --- REACT ---
    let http_react = ctx.http.clone();
    let tx_react = tui_tx.clone();
    navi.set(
        "react",
        lua.create_function(
            move |_, (channel_id, message_id, emoji): (String, String, String)| {
                let http = http_react.clone();
                let tx = tx_react.clone();
                tokio::spawn(async move {
                    let c_id = serenity::ChannelId::new(channel_id.parse().unwrap_or(0));
                    let m_id = serenity::MessageId::new(message_id.parse().unwrap_or(0));
                    let reaction_type = serenity::ReactionType::try_from(emoji.as_str())
                        .unwrap_or(serenity::ReactionType::Unicode(emoji));
                    if let Err(e) = c_id.create_reaction(&http, m_id, reaction_type).await {
                        let _ = tx.send(BotEvent::Log(LogLevel::Error, format!("Failed to react: {}", e)));
                    }
                });
                Ok(())
            },
        )?,
    )?;

    // --- EDIT MESSAGE ---
    let http_edit_msg = ctx.http.clone();
    navi.set("edit_message", lua.create_function(move |_, (channel_id, message_id, content): (String, String, String)| {
        let http = http_edit_msg.clone();
        tokio::spawn(async move {
            let c_id = serenity::ChannelId::new(channel_id.parse().unwrap_or(0));
            let m_id = serenity::MessageId::new(message_id.parse().unwrap_or(0));
            let _ = c_id.edit_message(&http, m_id, serenity::EditMessage::new().content(content)).await;
        });
        Ok(())
    })?)?;

    // --- DELETE MESSAGE ---
    let http_del_msg = ctx.http.clone();
    navi.set("delete_message", lua.create_function(move |_, (channel_id, message_id): (String, String)| {
        let http = http_del_msg.clone();
        tokio::spawn(async move {
            let c_id = serenity::ChannelId::new(channel_id.parse().unwrap_or(0));
            let m_id = serenity::MessageId::new(message_id.parse().unwrap_or(0));
            let _ = c_id.delete_message(&http, m_id).await;
        });
        Ok(())
    })?)?;

    // --- SEND MESSAGE / EMBED / COMPONENTS ---
    let http_msg = ctx.http.clone();
    let tx_send_msg = tui_tx.clone();
    navi.set(
        "send_message",
        lua.create_function(move |_, (channel_id, data): (String, LuaTable)| {
            let http = http_msg.clone();
            let tx = tx_send_msg.clone();

            let title: Option<String> = data.get("title").ok();
            let description: Option<String> = data.get("description").ok();
            let color: Option<u32> = data.get("color").ok();
            let image_url: Option<String> = data.get::<_, String>("image").ok().filter(|s| !s.is_empty());
            let mut fields = Vec::new();
            if let Ok(lua_fields) = data.get::<_, Vec<LuaTable>>("fields") {
                for f in lua_fields {
                    let name: String = f.get("name").unwrap_or_default();
                    let value: String = f.get("value").unwrap_or_default();
                    let inline: bool = f.get("inline").unwrap_or(false);
                    fields.push((name, value, inline));
                }
            }

            let mut action_rows = Vec::new();
            let mut current_buttons = Vec::new();

            if let Ok(comps) = data.get::<_, Vec<LuaTable>>("components") {
                for c in comps {
                    let c_type: String = c.get("type").unwrap_or("button".into());

                    if c_type == "button" {
                        let label: String = c.get("label").unwrap_or("Button".into());
                        let style_str: String = c.get("style").unwrap_or("primary".into());

                        if style_str == "link" || style_str == "url" {
                            let url: String = c.get("url").unwrap_or("https://discord.com".into());
                            current_buttons
                                .push(serenity::CreateButton::new_link(url).label(label));
                        } else {
                            let custom_id: String = c.get("id").unwrap_or("unknown".into());
                            let style = match style_str.as_str() {
                                "secondary" | "gray" => serenity::ButtonStyle::Secondary,
                                "success" | "green" => serenity::ButtonStyle::Success,
                                "danger" | "red" => serenity::ButtonStyle::Danger,
                                _ => serenity::ButtonStyle::Primary,
                            };
                            current_buttons.push(
                                serenity::CreateButton::new(custom_id)
                                    .style(style)
                                    .label(label),
                            );
                        }
                    } else if c_type == "select" {
                        if !current_buttons.is_empty() {
                            action_rows
                                .push(serenity::CreateActionRow::Buttons(current_buttons.clone()));
                            current_buttons.clear();
                        }

                        let custom_id: String = c.get("id").unwrap_or("select_menu".into());
                        let placeholder: String =
                            c.get("placeholder").unwrap_or("Select an option...".into());

                        let mut options = Vec::new();
                        if let Ok(lua_opts) = c.get::<_, Vec<LuaTable>>("options") {
                            for opt in lua_opts {
                                let label: String = opt.get("label").unwrap_or("Option".into());
                                let value: String = opt.get("value").unwrap_or(label.clone());
                                let desc: Option<String> = opt.get("description").ok();
                                let emoji: Option<String> = opt.get("emoji").ok();

                                let mut builder =
                                    serenity::CreateSelectMenuOption::new(label, value);
                                if let Some(d) = desc {
                                    builder = builder.description(d);
                                }
                                if let Some(e) = emoji {
                                    builder = builder.emoji(serenity::ReactionType::Unicode(e));
                                }

                                options.push(builder);
                            }
                        }

                        let menu = serenity::CreateSelectMenu::new(
                            custom_id,
                            serenity::CreateSelectMenuKind::String { options },
                        )
                        .placeholder(placeholder);

                        action_rows.push(serenity::CreateActionRow::SelectMenu(menu));
                    }
                }
            }

            if !current_buttons.is_empty() {
                action_rows.push(serenity::CreateActionRow::Buttons(current_buttons));
            }

            tokio::spawn(async move {
                let mut msg = serenity::CreateMessage::new();

                if title.is_some() || description.is_some() {
                    let mut embed = serenity::CreateEmbed::new();
                    if let Some(t) = title {
                        embed = embed.title(t);
                    }
                    if let Some(d) = description {
                        embed = embed.description(d);
                    }
                    embed = embed.color(parse_color(color));
                    for (n, v, i) in fields {
                        embed = embed.field(n, v, i);
                    }
                    if let Some(img) = image_url {
                        embed = embed.image(img);
                    }
                    msg = msg.embed(embed);
                }

                if !action_rows.is_empty() {
                    msg = msg.components(action_rows);
                }

                let c_id = serenity::ChannelId::new(channel_id.parse().unwrap_or(0));
                if let Err(e) = c_id.send_message(&http, msg).await {
                    let _ = tx.send(BotEvent::Log(LogLevel::Error, format!("Error sending message: {}", e)));
                }
            });
            Ok(())
        })?,
    )?;

    // --- DM ---
    let http_dm = ctx.http.clone();
    let tx_dm = tui_tx.clone();
    navi.set("dm", lua.create_function(move |_, (user_id, text): (String, String)| {
        let http = http_dm.clone();
        let tx = tx_dm.clone();
        tokio::spawn(async move {
            let u_id = serenity::UserId::new(user_id.parse().unwrap_or(0));
            match u_id.create_dm_channel(&http).await {
                Ok(dm) => {
                    if let Err(e) = dm.say(&http, text).await {
                        let _ = tx.send(BotEvent::Log(LogLevel::Error, format!("Failed to send DM: {}", e)));
                    }
                }
                Err(e) => {
                    let _ = tx.send(BotEvent::Log(LogLevel::Error, format!("Failed to open DM channel: {}", e)));
                }
            }
        });
        Ok(())
    })?)?;

    // --- FETCH MESSAGE ---
    let http_fetch = ctx.http.clone();
    navi.set("fetch_message", lua.create_function(move |lua, (channel_id, message_id): (String, String)| {
        let http = http_fetch.clone();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let c_id = serenity::ChannelId::new(channel_id.parse().unwrap_or(0));
                let m_id = serenity::MessageId::new(message_id.parse().unwrap_or(0));
                c_id.message(&http, m_id).await
            })
        });
        match result {
            Ok(msg) => {
                let t = lua.create_table()?;
                t.set("message_id", msg.id.get().to_string())?;
                t.set("channel_id", msg.channel_id.get().to_string())?;
                t.set("guild_id", msg.guild_id.map(|g| g.get().to_string()))?;
                t.set("content", msg.content.clone())?;
                t.set("author_id", msg.author.id.get().to_string())?;
                t.set("author", msg.author.name.clone())?;
                let attachments = lua.create_table()?;
                for (i, a) in msg.attachments.iter().enumerate() {
                    attachments.set(i + 1, a.url.clone())?;
                }
                t.set("attachments", attachments)?;
                Ok(mlua::Value::Table(t))
            }
            Err(_) => Ok(mlua::Value::Nil),
        }
    })?)?;

    Ok(())
}
