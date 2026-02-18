use crate::types::{Data, Error, LuaEmbed};
use poise::serenity_prelude as serenity;
use mlua::prelude::*;
use mlua::Function;
use rusqlite::{Connection, OptionalExtension};
use std::sync::{Arc, Mutex};

pub async fn init(ctx: &serenity::Context) -> Result<Data, Error> {
    println!("Navi is waking up...");

    // 1. Initialize Lua
    let lua = Lua::new();

    // 2. Initialize DB
    let conn = Connection::open("navi.db").expect("Failed to open DB");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT)",
        (),
    ).expect("Failed to create table");
    let db = Arc::new(Mutex::new(conn));

    // 3. Setup 'navi' Table
    let navi = lua.create_table()?;
    
    // --- MESSAGING (Say/Embed) ---
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

    let http_client_embed = ctx.http.clone();
    let send_embed = lua.create_function(move |lua, (channel_id, embed_data): (u64, LuaValue)| {
        let embed_struct: LuaEmbed = lua.from_value(embed_data).map_err(mlua::Error::external)?;
        
        // Build the embed using Serenity builder
        let mut builder = serenity::CreateEmbed::default();
        if let Some(t) = embed_struct.title { builder = builder.title(t); }
        if let Some(d) = embed_struct.description { builder = builder.description(d); }
        if let Some(c) = embed_struct.color { builder = builder.color(serenity::Color::from(c)); }
        if let Some(u) = embed_struct.url { builder = builder.url(u); }
        if let Some(i) = embed_struct.image { builder = builder.image(i); }
        if let Some(th) = embed_struct.thumbnail { builder = builder.thumbnail(th); }
        
        if let Some(f) = embed_struct.footer {
            let mut footer = serenity::CreateEmbedFooter::new(f.text);
            if let Some(icon) = f.icon_url { footer = footer.icon_url(icon); }
            builder = builder.footer(footer);
        }
        
        if let Some(fields) = embed_struct.fields {
            for field in fields {
                builder = builder.field(field.name, field.value, field.inline.unwrap_or(false));
            }
        }

        let http = http_client_embed.clone();
        tokio::spawn(async move {
            let channel = serenity::ChannelId::new(channel_id);
            let msg = serenity::CreateMessage::new().embed(builder);
            if let Err(e) = channel.send_message(&http, msg).await {
                println!("❌ Failed to send embed: {}", e);
            }
        });
        Ok(())
    })?;
    navi.set("send_embed", send_embed)?;

    // --- DATABASE (Get/Set) ---
    let db_table = lua.create_table()?;
    
    let db_conn_set = db.clone();
    db_table.set("set", lua.create_function(move |_, (key, value): (String, String)| {
        let conn = db_conn_set.lock().unwrap();
        conn.execute("INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)", (key, value))
            .map_err(mlua::Error::external)?;
        Ok(())
    })?)?;

    let db_conn_get = db.clone();
    db_table.set("get", lua.create_function(move |lua, key: String| {
        let conn = db_conn_get.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM kv_store WHERE key = ?1").map_err(mlua::Error::external)?;
        let result: Option<String> = stmt.query_row([key], |row| row.get(0)).optional().map_err(mlua::Error::external)?;
        
        match result {
            Some(val) => Ok(mlua::Value::String(lua.create_string(&val)?)),
            None => Ok(mlua::Value::Nil),
        }
    })?)?;
    navi.set("db", db_table)?;

    // --- REGISTRIES ---
    // Event Bus
    let listeners = lua.create_table()?;
    navi.set("listeners", listeners)?;
    navi.set("register", lua.create_function(|lua, func: Function| {
        let navi: LuaTable = lua.globals().get("navi")?;
        let listeners: LuaTable = navi.get("listeners")?;
        listeners.set(listeners.len()? + 1, func)?;
        Ok(())
    })?)?;

    // Commands
    navi.set("commands", lua.create_table()?)?;
    navi.set("create_command", lua.create_function(|lua, (name, func): (String, Function)| {
        let navi: LuaTable = lua.globals().get("navi")?;
        let commands: LuaTable = navi.get("commands")?;
        commands.set(name.clone(), func)?;
        println!("   > Registered command: !{}", name); 
        Ok(())
    })?)?;

    // Slash
    navi.set("slash_commands", lua.create_table()?)?;
    navi.set("create_slash", lua.create_function(|lua, (name, desc, func): (String, String, Function)| {
        let navi: LuaTable = lua.globals().get("navi")?;
        let slash_cmds: LuaTable = navi.get("slash_commands")?;
        let cmd_data = lua.create_table()?;
        cmd_data.set("description", desc)?;
        cmd_data.set("callback", func)?;
        slash_cmds.set(name.clone(), cmd_data)?;
        println!("   > Registered Slash: /{}", name);
        Ok(())
    })?)?;

    // Finish Setup
    lua.globals().set("navi", navi)?;
    
    // Conductor
    lua.load(r#"
        function on_message(msg)
            if navi.listeners then
                for i, listener in ipairs(navi.listeners) do
                    local success, err = pcall(listener, msg)
                    if not success then print("Error in plugin: " .. tostring(err)) end
                end
            end
        end
    "#).exec()?;

    Ok(Data {
        lua: Arc::new(Mutex::new(lua)),
        db,
    })
}