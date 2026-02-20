use crate::types::{BotEvent, Data, Error};
use mlua::prelude::*;
use mlua::Function;
use mlua::{StdLib, LuaOptions};
use poise::serenity_prelude as serenity;
use rusqlite::{Connection, OptionalExtension};
use serenity::{CreateCommand, CreateCommandOption};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;
use crate::types::{ConfigRegistry, ConfigField, ConfigType, PluginSchema};

// --- HELPER: Load Plugins ---
pub fn load_plugins(lua: &Lua) -> String {
    // 1. Reset Event Bus
    let result: LuaResult<()> = (|| {
        let navi: LuaTable = lua.globals().get("navi")?;
        navi.set("listeners", lua.create_table()?)?;
        Ok(())
    })();

    if let Err(e) = result {
        return format!("❌ Failed to reset listeners: {}", e);
    }

    // 2. Read Files
    let mut count = 0;
    let mut details = String::new();
    let mut error_msg = None;

    if !Path::new("plugins").exists() {
        return "⚠️ 'plugins/' folder not found!".to_string();
    }

    if let Ok(paths) = std::fs::read_dir("plugins") {
        for path in paths {
            if let Ok(path) = path {
                let p = path.path();
                if p.extension().and_then(|s| s.to_str()) == Some("lua") {
                    if let Ok(code) = std::fs::read_to_string(&p) {
                        let chunk = lua.load(&code).set_name(p.to_string_lossy());
                        if let Err(e) = chunk.exec() {
                            error_msg = Some(format!("❌ Error in {:?}: \n{}", p, e));
                            break;
                        }
                        count += 1;
                        details.push_str(&format!("Loaded: {:?}\n", p.file_name().unwrap()));
                    }
                }
            }
        }
    }

    if let Some(err) = error_msg {
        err
    } else {
        format!("✅ Loaded {} plugins.", count)
    }
}

// --- HELPER: Read Slash Commands (Sync) ---
pub fn read_slash_commands(lua: &Lua) -> Result<Vec<CreateCommand>, Error> {
    let navi: LuaTable = lua.globals().get("navi")?;
    // Gracefully handle if slash_commands table doesn't exist yet
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
) -> Result<Data, Error> {
    let _ = tui_tx.send(BotEvent::Log("--- Engine Initialization ---".into()));

    let libs = StdLib::ALL_SAFE | StdLib::DEBUG;
    let lua = unsafe { 
        Lua::unsafe_new_with(libs, LuaOptions::default())
    };

    

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

    // --- LOGGING (Intercept print) ---
    let tx_log = tui_tx.clone();
    navi.set("log", lua.create_function(move |_, msg: String| {
        let _ = tx_log.send(BotEvent::Log(msg));
        Ok(())
    })?)?;

    // 2. Overwrite global print() (Lua side)
    // We do this immediately so it applies to all plugins loaded later.
    lua.load(r#"
        -- Save the old print just in case
        local old_print = print
        
        -- New Print Function
        _G.print = function(...)
            local args = {...}
            local parts = {}
            for i, v in ipairs(args) do
                table.insert(parts, tostring(v))
            end
            local msg = table.concat(parts, "\t")
            
            -- Get the caller's filename
            local info = debug.getinfo(2, "S")
            local source = info and info.short_src or "Unknown"
            
            -- Clean up path: "plugins/hello.lua" -> "hello.lua"
            source = source:gsub("plugins[/\\]", "")
            
            -- Send to TUI
            navi.log(string.format("[%s] %s", source, msg))
        end
    "#).exec()?;

    // --- MESSAGING ---
    let http_client = ctx.http.clone();
    let say_fn = lua.create_function(move |_, (channel_id, text): (u64, String)| {
        let http = http_client.clone();
        tokio::spawn(async move {
            let channel = serenity::ChannelId::new(channel_id);
            if let Err(e) = channel.say(&http, text).await {
                println!("Error sending message: {}", e);
            }
        });
        Ok(())
    })?;
    navi.set("say", say_fn)?;

    // Add Role
    let http_add_role = ctx.http.clone();
    navi.set("add_role", lua.create_function(move |_, (guild_id, user_id, role_id): (String, String, String)| {
        let http = http_add_role.clone();
        tokio::spawn(async move {
            let g_id = serenity::GuildId::new(guild_id.parse().unwrap_or(0));
            let u_id = serenity::UserId::new(user_id.parse().unwrap_or(0));
            let r_id = serenity::RoleId::new(role_id.parse().unwrap_or(0));
            
            if let Err(e) = http.add_member_role(g_id, u_id, r_id, None).await {
                println!("Failed to add role: {}", e);
            }
        });
        Ok(())
    })?)?;

    // Remove Role
    let http_remove_role = ctx.http.clone();
    navi.set("remove_role", lua.create_function(move |_, (guild_id, user_id, role_id): (String, String, String)| {
        let http = http_remove_role.clone();
        tokio::spawn(async move {
            let g_id = serenity::GuildId::new(guild_id.parse().unwrap_or(0));
            let u_id = serenity::UserId::new(user_id.parse().unwrap_or(0));
            let r_id = serenity::RoleId::new(role_id.parse().unwrap_or(0));

            if let Err(e) = http.remove_member_role(g_id, u_id, r_id, None).await {
                println!("Failed to remove role: {}", e);
            }
        });
        Ok(())
    })?)?;

    // React to Message
    let http_react = ctx.http.clone();
    navi.set("react", lua.create_function(move |_, (channel_id, message_id, emoji): (String, String, String)| {
        let http = http_react.clone();
        tokio::spawn(async move {
            let c_id = serenity::ChannelId::new(channel_id.parse().unwrap_or(0));
            let m_id = serenity::MessageId::new(message_id.parse().unwrap_or(0));
            let reaction_type = serenity::ReactionType::try_from(emoji.as_str()).unwrap_or(serenity::ReactionType::Unicode(emoji));

            if let Err(e) = c_id.create_reaction(&http, m_id, reaction_type).await {
                println!("Failed to react: {}", e);
            }
        });
        Ok(())
    })?)?;


    
    // navi.send_message(channel_id, { title="...", components={...} })
    // We are renaming this to 'send_message' to reflect it does more than just embeds now.
    // (You can keep 'send_embed' as an alias if you want, or just update your plugins)
    let http_msg = ctx.http.clone();
    
    // Helper to parse Hex Color
    fn parse_color(c: Option<u32>) -> serenity::Color {
        match c {
            Some(val) => serenity::Color::new(val),
            None => serenity::Color::new(0x000000),
        }
    }

    navi.set("send_message", lua.create_function(move |_, (channel_id, data): (String, LuaTable)| {
        let http = http_msg.clone();
        
        // 1. Parse Embed Fields
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

        // 2. Parse Components (Action Rows)
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
                        current_buttons.push(serenity::CreateButton::new_link(url).label(label));
                    } else {
                        let custom_id: String = c.get("id").unwrap_or("unknown".into());
                        let style = match style_str.as_str() {
                            "secondary" | "gray" => serenity::ButtonStyle::Secondary,
                            "success" | "green" => serenity::ButtonStyle::Success,
                            "danger" | "red" => serenity::ButtonStyle::Danger,
                            _ => serenity::ButtonStyle::Primary,
                        };
                        current_buttons.push(serenity::CreateButton::new(custom_id).style(style).label(label));
                    }
                } 
                else if c_type == "select" {
                    // Discord requires Select Menus to be on their own row!
                    // If we have pending buttons, flush them to a row first.
                    if !current_buttons.is_empty() {
                        action_rows.push(serenity::CreateActionRow::Buttons(current_buttons.clone()));
                        current_buttons.clear(); // Reset for the next batch
                    }
                    
                    let custom_id: String = c.get("id").unwrap_or("select_menu".into());
                    let placeholder: String = c.get("placeholder").unwrap_or("Select an option...".into());
                    
                    let mut options = Vec::new();
                    if let Ok(lua_opts) = c.get::<_, Vec<LuaTable>>("options") {
                        for opt in lua_opts {
                            let label: String = opt.get("label").unwrap_or("Option".into());
                            let value: String = opt.get("value").unwrap_or(label.clone());
                            let desc: Option<String> = opt.get("description").ok();
                            let emoji: Option<String> = opt.get("emoji").ok();
                            
                            let mut builder = serenity::CreateSelectMenuOption::new(label, value);
                            if let Some(d) = desc { builder = builder.description(d); }
                            if let Some(e) = emoji { builder = builder.emoji(serenity::ReactionType::Unicode(e)); }
                            
                            options.push(builder);
                        }
                    }

                    let menu = serenity::CreateSelectMenu::new(
                        custom_id, 
                        serenity::CreateSelectMenuKind::String { options }
                    ).placeholder(placeholder);
                    
                    action_rows.push(serenity::CreateActionRow::SelectMenu(menu));
                }
            }
        }

        // Flush any remaining buttons at the very end
        if !current_buttons.is_empty() {
            action_rows.push(serenity::CreateActionRow::Buttons(current_buttons));
        }

        tokio::spawn(async move {
            let mut msg = serenity::CreateMessage::new();

            // Attach Embed
            if title.is_some() || description.is_some() {
                let mut embed = serenity::CreateEmbed::new();
                if let Some(t) = title { embed = embed.title(t); }
                if let Some(d) = description { embed = embed.description(d); }
                embed = embed.color(parse_color(color));
                for (n, v, i) in fields { embed = embed.field(n, v, i); }
                msg = msg.embed(embed);
            }

            // Attach the completely formatted Action Rows
            if !action_rows.is_empty() {
                msg = msg.components(action_rows);
            }

            let c_id = serenity::ChannelId::new(channel_id.parse().unwrap_or(0));
            if let Err(e) = c_id.send_message(&http, msg).await {
                println!("Error sending message: {}", e);
            }
        });
        Ok(())
    })?)?;

    // Config registry
    let registry_for_lua = config_registry.clone();

    navi.set("register_config", lua.create_function(move |_, (plugin_name, schema): (String, mlua::Table)| {
        let mut fields = Vec::new();

        // Iterate over the Lua array of tables
        // Example: { { key = "channel_id", type = "string" }, ... }
        for pair in schema.pairs::<mlua::Integer, mlua::Table>() {
            let (_, field_table) = pair?;
            
            let key: String = field_table.get("key")?;
            let name: String = field_table.get("name").unwrap_or_else(|_| key.clone());
            let description: String = field_table.get("description").unwrap_or_default();
            let type_str: String = field_table.get("type").unwrap_or_else(|_| "string".to_string());
            
            // Smart coercion: Convert whatever they typed as 'default' into a String
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
                _ => ConfigType::String,
            };

            fields.push(ConfigField {
                key: key.clone(),
                name,
                description,
                field_type,
                default_value: default_value.clone(),
            });

            // --- DATABASE DEFAULT INJECTION (Optional but recommended) ---
            // If you want the bot to automatically populate the database with defaults 
            // so they aren't 'nil' the first time the plugin runs, do it here!
            /*
            let db_key = format!("config:{}:{}", plugin_name, key);
            // Pseudo-code assuming your DB implementation:
            if db_for_config.get(&db_key).is_none() {
                db_for_config.set(&db_key, &default_value);
            }
            */
        }

        let plugin_schema = PluginSchema {
            plugin_name: plugin_name.clone(),
            fields,
        };

        // Lock the shared registry and save the schema so the TUI can read it
        {
            let mut registry = registry_for_lua.lock().unwrap();
            registry.insert(plugin_name.clone(), plugin_schema);
        }

        println!("⚙️ Registered config schema for plugin: {}", plugin_name);

        Ok(())
    })?)?;


    // --- DB API ---
    let db_table = lua.create_table()?;
    let db_conn_set = db.clone();
    db_table.set(
        "set",
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
    db_table.set(
        "get",
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
    navi.set("db", db_table)?;

    // --- REGISTRIES (This is what you were missing!) ---
    
    // A. Listeners (for dispatcher)
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

    // B. Text Commands (for dispatcher)
    let commands = lua.create_table()?;
    navi.set("commands", commands)?;

    navi.set(
        "create_command",
        lua.create_function(|lua, (name, func): (String, Function)| {
            let navi: LuaTable = lua.globals().get("navi")?;
            let commands: LuaTable = navi.get("commands")?;
            commands.set(name, func)?;
            Ok(())
        })?,
    )?;

    // C. Slash Commands
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

    // 3. FINISH SETUP
    lua.globals().set("navi", navi)?;

    // 4. LOAD CONDUCTOR (The generic handler that calls listeners)
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
    let load_report = load_plugins(&lua);
    let _ = tui_tx.send(BotEvent::Log(load_report));

    Ok(Data {
        lua: Arc::new(Mutex::new(lua)),
        db,
        tui_tx,
    })
}