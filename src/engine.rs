use crate::types::{BotEvent, Data, Error, LogLevel};
use crate::types::{ConfigField, ConfigRegistry, ConfigType, PluginSchema};
use mlua::prelude::*;
use mlua::Function;
use mlua::{LuaOptions, StdLib};
use poise::serenity_prelude as serenity;
use rusqlite::{Connection, OptionalExtension};
use serenity::{CreateCommand, CreateCommandOption};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

// --- HELPER: Load Plugins ---
pub fn load_plugins(lua: &Lua) -> (LogLevel, String) {
    // 1. Reset Event Bus
    let result: LuaResult<()> = (|| {
        let navi: LuaTable = lua.globals().get("navi")?;
        navi.set("listeners", lua.create_table()?)?;
        Ok(())
    })();

    if let Err(e) = result {
        return (LogLevel::Error, format!("Failed to reset listeners: {}", e));
    }

    // 2. Read Files
    let mut count = 0;
    let mut details = String::new();
    let mut error_msg = None;

    if !Path::new("plugins").exists() {
        return (LogLevel::Warn, "'plugins/' folder not found!".to_string());
    }

    // --- BAKE IN THE CORE ENGINE API ---
    let core_events = include_str!("../core/events.lua");
    let core_components = include_str!("../core/components.lua");
    // let core_dispatcher = include_str!("../core/dispatcher.lua");

    // Execute them in exact order
    let _ = lua.load(core_events).set_name("core:events").exec();
    let _ = lua.load(core_components).set_name("core:components").exec();
    // let _ = lua.load(core_dispatcher).set_name("core:dispatcher").exec();

    if let Ok(entries) = std::fs::read_dir("plugins") {
        // 1. Collect all valid .lua files into a Vector first
        let mut lua_files = Vec::new();
        for entry in entries {
            if let Ok(file) = entry {
                let p = file.path();
                if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("lua") {
                    lua_files.push(p);
                }
            }
        }

        // 2. Sort plugin loading alphabetically
        lua_files.sort();

        // 3. Now execute them in guaranteed order
        for p in lua_files {
            if let Ok(code) = std::fs::read_to_string(&p) {
                let chunk = lua.load(&code).set_name(p.to_string_lossy());
                if let Err(e) = chunk.exec() {
                    error_msg = Some(format!("Error in {:?}: \n{}", p, e));
                    break;
                }
                count += 1;
                details.push_str(&format!("Loaded: {:?}\n", p.file_name().unwrap()));
            }
        }
    }

    if let Some(err) = error_msg {
        (LogLevel::Error, err)
    } else {
        (LogLevel::Info, format!("Loaded {} plugins.", count))
    }
}

// --- HELPER: Read Slash Commands (Sync) ---
pub fn read_slash_commands(lua: &Lua) -> Result<Vec<CreateCommand>, Error> {
    let navi: LuaTable = lua.globals().get("navi")?;
    let slash_cmds: LuaTable = match navi.get("slash_commands") {
        Ok(t) => t,
        Err(_) => return Ok(Vec::new()),
    };

    let mut commands = Vec::new();

    for pair in slash_cmds.pairs::<String, LuaTable>() {
        let (name, data) = pair?;
        let desc: String = data.get("description")?;

        let mut command = CreateCommand::new(name).description(desc);

        if let Ok(options) = data.get::<_, Vec<LuaTable>>("options") {
            for opt in options {
                let name: String = opt.get("name")?;
                let desc: String = opt.get("description")?;
                let type_str: String = opt.get("type")?;
                let required: bool = opt.get("required").unwrap_or(false);

                let kind = match type_str.as_str() {
                    "string" => serenity::CommandOptionType::String,
                    "integer" => serenity::CommandOptionType::Integer,
                    "boolean" => serenity::CommandOptionType::Boolean,
                    "user" => serenity::CommandOptionType::User,
                    "channel" => serenity::CommandOptionType::Channel,
                    "role" => serenity::CommandOptionType::Role,
                    "number" => serenity::CommandOptionType::Number,
                    _ => serenity::CommandOptionType::String,
                };

                let option = CreateCommandOption::new(kind, name, desc).required(required);
                command = command.add_option(option);
            }
        }
        commands.push(command);
    }

    Ok(commands)
}

// --- HELPER: Upload Slash Commands (Async) ---
pub async fn upload_slash_commands(
    http: &serenity::Http,
    commands: Vec<CreateCommand>,
) -> Result<String, Error> {
    let guilds = http.get_guilds(None, None).await?;
    let count = guilds.len();

    for guild in guilds {
        guild.id.set_commands(http, commands.clone()).await?;
    }

    Ok(format!("✅ Synced slash commands to {} guilds.", count))
}

// --- ENGINE INITIALIZATION ---
pub async fn init(
    ctx: &serenity::Context,
    tui_tx: UnboundedSender<BotEvent>,
    config_registry: ConfigRegistry,
    discord_state: crate::types::SharedDiscordState,
) -> Result<Data, Error> {
    let _ = tui_tx.send(BotEvent::Log(LogLevel::Info, "--- Engine Initialization ---".into()));

    let libs = StdLib::ALL_SAFE | StdLib::DEBUG;
    let lua = unsafe { Lua::unsafe_new_with(libs, LuaOptions::default()) };

    // 1. DATABASE
    let conn = Connection::open("navi.db").expect("Failed to open DB");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT)",
        (),
    )
    .expect("Failed to create DB table");
    let db = Arc::new(Mutex::new(conn));

    // 2. CREATE 'navi' TABLE
    let navi = lua.create_table()?;
    lua.globals().set("navi", navi.clone())?; // <--- INJECT IT IMMEDIATELY

    // --- LOGGING ---
    let navi_log = lua.create_table()?;

    let tx_info = tui_tx.clone();
    navi_log.set("info", lua.create_function(move |_, msg: String| {
        let _ = tx_info.send(BotEvent::Log(LogLevel::Info, msg));
        Ok(())
    })?)?;

    let tx_warn = tui_tx.clone();
    navi_log.set("warn", lua.create_function(move |_, msg: String| {
        let _ = tx_warn.send(BotEvent::Log(LogLevel::Warn, msg));
        Ok(())
    })?)?;

    let tx_err = tui_tx.clone();
    navi_log.set("error", lua.create_function(move |_, msg: String| {
        let _ = tx_err.send(BotEvent::Log(LogLevel::Error, msg));
        Ok(())
    })?)?;

    navi.set("log", navi_log)?;

    lua.load(
        r#"
        local old_print = print
        _G.print = function(...)
            local args = {...}
            local parts = {}
            for i, v in ipairs(args) do
                table.insert(parts, tostring(v))
            end
            local msg = table.concat(parts, "\t")
            local info = debug.getinfo(2, "S")
            local source = info and info.short_src or "Unknown"
            source = source:gsub("plugins[/\\]", "")
            navi.log.info(string.format("[%s] %s", source, msg))
        end
    "#,
    )
    .exec()?;

    // DATABASE QUERYING
    let db_conn_query = db.clone();
    navi.set(
        "_db_query_raw",
        lua.create_function(move |lua, sql: String| {
            let conn = db_conn_query.lock().unwrap();

            let mut stmt = conn.prepare(&sql).map_err(mlua::Error::external)?;

            // We map the first two columns (index 0 and 1) into strings
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0).unwrap_or_default(),
                        row.get::<_, String>(1).unwrap_or_default(),
                    ))
                })
                .map_err(mlua::Error::external)?;

            // Build a Lua array of {key = "...", value = "..."} tables
            let result_table = lua.create_table()?;
            for (i, row_result) in rows.enumerate() {
                if let Ok((k, v)) = row_result {
                    let row_table = lua.create_table()?;
                    row_table.set("key", k)?;
                    row_table.set("value", v)?;
                    result_table.set(i + 1, row_table)?;
                }
            }

            Ok(result_table)
        })?,
    )?;

    // --- MESSAGING ---
    let http_client = ctx.http.clone();
    let tx_say = tui_tx.clone();
    let say_fn = lua.create_function(move |_, (channel_id, text): (u64, String)| {
        let http = http_client.clone();
        let tx = tx_say.clone();
        tokio::spawn(async move {
            let channel = serenity::ChannelId::new(channel_id);
            if let Err(e) = channel.say(&http, text).await {
                let _ = tx.send(BotEvent::Log(LogLevel::Error, format!("Error sending message: {}", e)));
            }
        });
        Ok(())
    })?;
    navi.set("say", say_fn)?;

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
                _ => None, // Clears the status if they type something invalid or "none"
            };

            ctx.set_activity(activity);
            Ok(())
        })?,
    )?;

    // CREATE CHANNEL
    let ctx_channel = ctx.clone();
    navi.set(
        "create_channel",
        lua.create_function(
            move |_, (guild_id, name, options): (String, String, mlua::Table)| {
                let http = ctx_channel.http.clone();

                // 1. Extract optional parameters from the Lua table
                let category_id: Option<String> = options.get("category_id").unwrap_or(None);
                let user_id: Option<String> = options.get("user_id").unwrap_or(None);
                let role_id: Option<String> = options.get("role_id").unwrap_or(None);
                let welcome_message: Option<String> =
                    options.get("welcome_message").unwrap_or(None);
                let close_button: Option<bool> = options.get("close_button").unwrap_or(None);

                tokio::spawn(async move {
                    use poise::serenity_prelude as serenity;
                    let gid = serenity::GuildId::new(guild_id.parse().unwrap_or(0));

                    // 2. Start building the generic channel
                    let mut builder =
                        serenity::CreateChannel::new(&name).kind(serenity::ChannelType::Text);

                    // 3. Apply Category if provided
                    if let Some(cid_str) = category_id {
                        if let Ok(cid) = cid_str.parse::<u64>() {
                            builder = builder.category(serenity::ChannelId::new(cid));
                        }
                    }

                    // 4. Apply Private Permissions if a user or role is provided
                    if user_id.is_some() || role_id.is_some() {
                        let everyone_id = serenity::RoleId::new(gid.get());
                        let mut perms = vec![serenity::PermissionOverwrite {
                            allow: serenity::Permissions::empty(),
                            deny: serenity::Permissions::VIEW_CHANNEL, // Deny @everyone
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

                    // 5. Create it and send the optional welcome message!
                    if let Ok(channel) = gid.create_channel(&http, builder).await {
                        if let Some(msg_text) = welcome_message {
                            let mut msg = serenity::CreateMessage::new().content(msg_text);

                            // --- ATTACH THE CLOSE BUTTON IF REQUESTED ---
                            if close_button.unwrap_or(false) {
                                let action_row = serenity::CreateActionRow::Buttons(vec![
                                    serenity::CreateButton::new("btn_close_ticket")
                                        .label("🔒 Close Ticket")
                                        .style(serenity::ButtonStyle::Danger), // Makes the button Red!
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

    // Add Role
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

    // Remove Role
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

    // React to Message
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

    // Send Message / Embed / Components
    let http_msg = ctx.http.clone();
    let tx_send_msg = tui_tx.clone();

    fn parse_color(c: Option<u32>) -> serenity::Color {
        match c {
            Some(val) => serenity::Color::new(val),
            None => serenity::Color::new(0x000000),
        }
    }

    navi.set(
        "send_message",
        lua.create_function(move |_, (channel_id, data): (String, LuaTable)| {
            let http = http_msg.clone();
            let tx = tx_send_msg.clone();

            let title: Option<String> = data.get("title").ok();
            let description: Option<String> = data.get("description").ok();
            let color: Option<u32> = data.get("color").ok();
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

    // --- CONFIG REGISTRY ---
    let registry_for_lua = config_registry.clone();
    let db_for_config = db.clone();
    let tx_config = tui_tx.clone();

    navi.set(
        "register_config",
        lua.create_function(move |_, (plugin_name, schema): (String, mlua::Table)| {
            let mut fields = Vec::new();

            for pair in schema.pairs::<mlua::Integer, mlua::Table>() {
                let (_, field_table) = pair?;

                let key: String = field_table.get("key")?;
                let name: String = field_table.get("name").unwrap_or_else(|_| key.clone());
                let description: String = field_table.get("description").unwrap_or_default();
                let type_str: String = field_table
                    .get("type")
                    .unwrap_or_else(|_| "string".to_string());

                let default_value: String = match field_table.get::<_, mlua::Value>("default") {
                    Ok(mlua::Value::String(s)) => s.to_str()?.to_string(),
                    Ok(mlua::Value::Integer(i)) => i.to_string(),
                    Ok(mlua::Value::Number(n)) => n.to_string(),
                    Ok(mlua::Value::Boolean(b)) => b.to_string(),
                    _ => "".to_string(),
                };

                let field_type = match type_str.as_str() {
                    "number" => ConfigType::Number,
                    "boolean" => ConfigType::Boolean,
                    "channel" => ConfigType::Channel,
                    "role" => ConfigType::Role,
                    "category" => ConfigType::Category,
                    _ => ConfigType::String,
                };

                // DIRECT SQLITE ACCESS - Safely read from the database to see if a value exists
                let db_key = format!("config:{}:{}", plugin_name, key);
                let mut actual_value: Option<String> = None;

                if let Ok(conn) = db_for_config.lock() {
                    if let Ok(mut stmt) = conn.prepare("SELECT value FROM kv_store WHERE key = ?1")
                    {
                        actual_value = stmt
                            .query_row([&db_key], |row| row.get(0))
                            .optional()
                            .unwrap_or(None);
                    }
                }

                // Use the actual DB value, or fallback to default (and save the default!)
                let final_value = if let Some(val) = actual_value {
                    val
                } else {
                    if let Ok(conn) = db_for_config.lock() {
                        let _ = conn.execute(
                            "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                            (&db_key, &default_value),
                        );
                    }
                    default_value.clone()
                };

                fields.push(ConfigField {
                    key: key.clone(),
                    name,
                    description,
                    field_type,
                    default_value: final_value,
                });
            }

            let plugin_schema = PluginSchema {
                plugin_name: plugin_name.clone(),
                fields,
            };

            {
                let mut registry = registry_for_lua.lock().unwrap();
                registry.insert(plugin_name.clone(), plugin_schema);
            }

            let _ = tx_config.send(BotEvent::Log(LogLevel::Info, format!(
                "Registered config schema for plugin: {}",
                plugin_name
            )));

            Ok(())
        })?,
    )?;

    // --- DB API ---
    let db_conn_set = db.clone();
    navi.set(
        "_db_set_raw",
        lua.create_function(move |_, (key, value): (String, String)| {
            let conn = db_conn_set.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                (key, value),
            )
            .map_err(mlua::Error::external)?;
            Ok(())
        })?,
    )?;

    let db_conn_get = db.clone();
    navi.set(
        "_db_get_raw",
        lua.create_function(move |lua, key: String| {
            let conn = db_conn_get.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT value FROM kv_store WHERE key = ?1")
                .map_err(mlua::Error::external)?;
            let result: Option<String> = stmt
                .query_row([key], |row| row.get(0))
                .optional()
                .map_err(mlua::Error::external)?;

            match result {
                Some(val) => Ok(mlua::Value::String(lua.create_string(&val)?)),
                None => Ok(mlua::Value::Nil),
            }
        })?,
    )?;

    // 2. Build the Smart Lua Wrapper
    lua.load(
        r#"
        navi.db = {}
        
        function navi.db.set(key, value)
            if not string.find(key, ":") then
                local info = debug.getinfo(2, "S")
                local source = info and info.short_src or "unknown"
                local plugin = string.match(source, "([^/\\]+)%.lua$") or "global"
                key = plugin .. ":" .. key
            end
            navi._db_set_raw(key, tostring(value))
        end

        function navi.db.get(key)
            if not string.find(key, ":") then
                local info = debug.getinfo(2, "S")
                local source = info and info.short_src or "unknown"
                local plugin = string.match(source, "([^/\\]+)%.lua$") or "global"
                key = plugin .. ":" .. key
            end
            return navi._db_get_raw(key)
        end

        -- NEW: Pass the SQL string directly to the raw pipe
        function navi.db.query(sql)
            return navi._db_query_raw(sql)
        end
    "#,
    )
    .exec()?;

    // --- REGISTRIES ---
    let listeners = lua.create_table()?;
    navi.set("listeners", listeners)?;

    navi.set(
        "register",
        lua.create_function(|lua, func: Function| {
            let navi: LuaTable = lua.globals().get("navi")?;
            let listeners: LuaTable = navi.get("listeners")?;
            listeners.set(listeners.len()? + 1, func)?;
            Ok(())
        })?,
    )?;

    let slash_cmds = lua.create_table()?;
    navi.set("slash_commands", slash_cmds)?;

    navi.set(
        "create_slash",
        lua.create_function(
            |lua, (name, desc, options, func): (String, String, LuaValue, Function)| {
                let navi: LuaTable = lua.globals().get("navi")?;
                let slash_cmds: LuaTable = navi.get("slash_commands")?;

                let cmd_data = lua.create_table()?;
                cmd_data.set("description", desc)?;
                cmd_data.set("options", options)?;
                cmd_data.set("callback", func)?;

                slash_cmds.set(name, cmd_data)?;
                Ok(())
            },
        )?,
    )?;

    // we do this higher up now to make the table
    // available to the core API files that are loaded before the plugins, so they can also register commands and listeners
    // lua.globals().set("navi", navi)?;

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

    // 4. LOAD CONDUCTOR
    lua.load(
        r#"
        function on_message(msg)
            if navi.listeners then
                for i, listener in ipairs(navi.listeners) do
                    pcall(listener, msg)
                end
            end
        end
    "#,
    )
    .exec()?;

    // 5. LOAD PLUGINS
    let (load_level, load_report) = load_plugins(&lua);
    let _ = tui_tx.send(BotEvent::Log(load_level, load_report));

    drop(navi); // <-- VERY IMPORTANT to drop this before we create the Data struct, otherwise we get a deadlock when trying to access it from the TUI!

    Ok(Data {
        lua: Arc::new(Mutex::new(lua)),
        db,
        tui_tx,
        discord_state,
    })
}

