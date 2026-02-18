use crate::types::{Data, Error, LuaEmbed, BotEvent}; // Added BotEvent
use poise::serenity_prelude as serenity;
use mlua::prelude::*;
use rusqlite::{Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;

// --- NEW HELPER: Load Plugins ---
pub fn load_plugins(lua: &Lua) -> String {
    // 1. Reset Event Bus
    // We wrap this in a block to handle errors gracefully
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
    let mut error_msg = None;
    
    // Ensure plugins directory exists
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
                            error_msg = Some(format!("❌ Error in {:?}: \n```{}```", p, e));
                            break;
                        }
                        count += 1;
                        println!("   > Loaded plugin: {:?}", p.file_name().unwrap());
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

// PHASE 1: Read from Lua (Synchronous, needs Lock)
pub fn read_slash_commands(lua: &Lua) -> Result<Vec<serenity::CreateCommand>, Error> {
    let navi: LuaTable = lua.globals().get("navi")?;
    let slash_cmds: LuaTable = navi.get("slash_commands")?;

    let mut commands = Vec::new();
    
    for pair in slash_cmds.pairs::<String, LuaTable>() {
        let (name, data) = pair?;
        let desc: String = data.get("description")?;
        
        let mut command = serenity::CreateCommand::new(name).description(desc);

        // Handle Options
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

                let option = serenity::CreateCommandOption::new(kind, name, desc).required(required);
                command = command.add_option(option);
            }
        }
        commands.push(command);
    }
    
    Ok(commands)
}

// PHASE 2: Upload to Discord (Async, NO Lock allowed)
pub async fn upload_slash_commands(http: &serenity::Http, commands: Vec<serenity::CreateCommand>) -> Result<String, Error> {
    // Fetch all guilds the bot is in
    let guilds = http.get_guilds(None, None).await?;
    let count = guilds.len();
    
    // Upload the command list to every guild
    for guild in guilds {
        guild.id.set_commands(http, commands.clone()).await?;
    }

    Ok(format!("✅ Synced slash commands to {} guilds.", count))
}

pub async fn init(
    ctx: &serenity::Context, 
    tui_tx: UnboundedSender<BotEvent>
) -> Result<Data, Error> {
    println!("--- Engine Initialization ---");

    let lua = Lua::new();

    // ... (KEEP ALL YOUR EXISTING DB AND API SETUP CODE HERE) ...
    // ... (Database setup, navi.say, navi.db, navi.listeners, etc.) ...
    
    // [PASTE THE MIDDLE PART OF YOUR OLD INIT FUNCTION HERE]
    // For brevity, I am skipping the boilerplate we wrote before.
    // Make sure you define the 'navi' table, DB, and registries BEFORE running plugins.

    // === START OF OLD INIT BOILERPLATE (Re-paste this from your file) ===
    let conn = Connection::open("navi.db").expect("Failed to open DB");
    conn.execute("CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT)", ()).unwrap();
    let db = Arc::new(Mutex::new(conn));
    let navi = lua.create_table()?;
    // ... setup navi.say, navi.db, registries ...
    lua.globals().set("navi", navi)?;
    lua.load(r#"function on_message(msg) if navi.listeners then for i, l in ipairs(navi.listeners) do pcall(l, msg) end end end"#).exec()?;
    // === END OF BOILERPLATE ===


    // --- NEW: Auto-Load Plugins ---
    let load_report = load_plugins(&lua);
    println!("{}", load_report);
    let _ = tui_tx.send(BotEvent::Log(load_report));

    Ok(Data {
        lua: Arc::new(Mutex::new(lua)),
        db,
        tui_tx,
    })
}