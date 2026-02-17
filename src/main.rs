use dotenvy::dotenv; // To load .env
use mlua::prelude::*;
use mlua::{Function, Table};
use poise::serenity_prelude as serenity; // Alias for the raw Discord types
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use std::sync::{Arc, Mutex}; // For SQLite database access

// The "Context" Object
// In Poise, every command gets a "Context" passed to it.
// This Context holds "Data" (your custom state) and "Error" (what happens if it fails).
struct Data {
    // We wrap Lua in Arc and Mutex.
    // Arc = Shared Ownership (Multiple parts of the code can hold it)
    // Mutex = Thread Safety (Only one part can use it at a time)
    lua: Arc<Mutex<Lua>>,
    db: Arc<Mutex<Connection>>,
}

#[derive(Debug, Deserialize)]
struct LuaEmbedField {
    name: String,
    value: String,
    inline: Option<bool>, // Option means it can be nil/null in Lua
}

// The Footer
#[derive(Debug, Deserialize)]
struct LuaEmbedFooter {
    text: String,
    icon_url: Option<String>,
}

// The Main Embed Container
#[derive(Debug, Deserialize)]
struct LuaEmbed {
    title: Option<String>,
    description: Option<String>,
    color: Option<u32>, // Hex color code (e.g., 0xFF0000)
    url: Option<String>,
    image: Option<String>,
    thumbnail: Option<String>,
    footer: Option<LuaEmbedFooter>,
    fields: Option<Vec<LuaEmbedField>>,
}

// Define the Error type for Poise commands. This allows us to use "?" for error handling in commands.
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// 1. Define the event handler as a standalone async function
async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    // Check for Message event
    if let serenity::FullEvent::Message { new_message } = event {
        // Prevent infinite loops
        if new_message.author.bot {
            return Ok(());
        }

        // 1. Extract data we need (cloning strings is cheap and safe)
        let content = new_message.content.clone();
        let author = new_message.author.name.clone();
        let channel_id = new_message.channel_id.get();

        // 2. Lock Lua in its own block
        {
            let lua = data.lua.lock().unwrap();

            // 3. GET THE FUNCTION (The fix is here)
            // We do NOT do: let globals = lua.globals();
            // Instead, we chain it. Rust will drop the 'globals' table immediately
            // after this line, but keep the 'func' alive.
            let func: Option<Function> = match lua.globals().get("on_message") {
                Ok(f) => Some(f),
                Err(_) => None, // Function not found in Lua
            };

            // 4. If we found the function, CALL IT
            if let Some(f) = func {
                let msg_table = lua.create_table()?;
                msg_table.set("content", content)?;
                msg_table.set("channel_id", channel_id)?;
                msg_table.set("message_id", new_message.id.get())?;
                msg_table.set("author", author)?;
                msg_table.set("author_id", new_message.author.id.get())?;
                msg_table.set("author_avatar", new_message.author.face())?;

                let mentions = lua.create_table()?;
                for (i, user) in new_message.mentions.iter().enumerate() {
                    let u = lua.create_table()?;
                    u.set("name", user.name.clone())?;
                    u.set("id", user.id.get())?;
                    u.set("avatar", user.face())?; // Reuse the face() helper!

                    // Lua lists are 1-indexed
                    mentions.set(i + 1, u)?;
                }
                msg_table.set("mentions", mentions)?; // Add it to the message object

                let attachments = lua.create_table()?;
                for (i, attachment) in new_message.attachments.iter().enumerate() {
                    // Lua arrays start at 1, so we use i + 1
                    attachments.set(i + 1, attachment.url.clone())?;
                }
                msg_table.set("attachments", attachments)?;

                if let Err(e) = f.call::<_, ()>(msg_table) {
                    println!("❌ Lua Error: {}", e);
                }
            }
        } // 'lua' lock is dropped here automatically
    }

    // 2. Slash Command Interaction Handler
    if let serenity::FullEvent::InteractionCreate { interaction } = event {
        if let serenity::Interaction::Command(command) = interaction {
            
            // --- STEP A: PREPARE DATA (No Lua yet) ---
            let cmd_name = command.data.name.clone();
            let user_id = command.user.id.get();
            let username = command.user.name.clone();
            
            let http = ctx.http.clone();
            let interaction_id = command.id;
            let interaction_token = command.token.clone();

            // --- STEP B: LOCK LUA ---
            {
                let lua = data.lua.lock().unwrap();

                // --- STEP C: FETCH THE CALLBACK ---
                // We use a closure here to look up the function and return ONLY the function.
                // This forces Rust to drop 'globals', 'navi', and 'slash_cmds' immediately.
                // This clears the "Borrow" on Lua so we can use it again in Step D.
                let callback: Option<Function> = (|| {
                    let globals = lua.globals();
                    let navi: Table = globals.get("navi").ok()?;
                    let slash: Table = navi.get("slash_commands").ok()?;
                    let cmd_data: Table = slash.get(cmd_name.as_str()).ok()?;
                    cmd_data.get("callback").ok()
                })();

                // --- STEP D: EXECUTE ---
                if let Some(func) = callback {
                    // Now we can safely create new tables because the old ones are gone!
                    
                    if let Ok(ctx_table) = lua.create_table() {
                        let _ = ctx_table.set("user_id", user_id);
                        let _ = ctx_table.set("username", username);

                        // Create the Reply Function
                        let reply_fn_result = lua.create_function(move |_, msg: String| {
                            let http = http.clone();
                            let token = interaction_token.clone();
                            let id = interaction_id;

                            tokio::spawn(async move {
                                let response = serenity::CreateInteractionResponse::Message(
                                    serenity::CreateInteractionResponseMessage::new().content(msg)
                                );
                                
                                // Send the reply
                                if let Err(e) = http.create_interaction_response(id, &token, &response, vec![]).await {
                                    println!("Error replying: {}", e);
                                }
                            });
                            Ok(())
                        });

                        if let Ok(reply_fn) = reply_fn_result {
                            let _ = ctx_table.set("reply", reply_fn);
                            
                            // CALL THE LUA FUNCTION
                            if let Err(e) = func.call::<_, ()>(ctx_table) {
                                println!("❌ Lua Slash Error: {}", e);
                            }
                        }
                    }
                }
            } // Lua lock drops here
        }
    }

    Ok(())
}

#[tokio::main] // 1. Starts the Tokio Event Loop
async fn main() {
    dotenv().ok(); // Load .env file

    // 1. Create the Lua VM
    // unsafe { } is needed here because loading standard libraries *can* be unsafe
    // in rare edge cases, but for a bot it's standard practice.
    let _lua = unsafe { Lua::unsafe_new() }; // Or Lua::new() if you don't need all libs yet

    // 2. Load standard libraries (print, math, string, etc.)
    // Note: In mlua 0.9+, Lua::new() loads std libs by default safely.
    // Let's stick to the safe one for now:
    let lua = Lua::new();

    // 3. Wrap it in the Arc<Mutex> so it can be shared
    let _lua = Arc::new(Mutex::new(lua));

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![run_lua(), reload(), sync()], // Make sure run_lua is registered!
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            // We wrap our async function in a Box::pin so Poise can store it
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        // Add 'move' before |ctx, ...|
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Navi is waking up...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                // Create Lua HERE (inside the bot startup)
                let lua = Lua::new();

                // Open the Database (creates navi.db if it doesn't exist)
                let conn = Connection::open("navi.db").expect("Failed to open DB");

                // Create the Key-Value Table (if it doesn't exist)
                // We use a simple schema: key (Text) -> value (Text)
                // We store everything as strings for simplicity in this version.
                conn.execute(
                    "CREATE TABLE IF NOT EXISTS kv_store (
                        key TEXT PRIMARY KEY,
                        value TEXT
                    )",
                    (), // No parameters
                )
                .expect("Failed to create table");

                let db = Arc::new(Mutex::new(conn));

                // Create the "navi" table (namespace)
                // This is like creating a JSON object: navi = {}
                let navi = lua.create_table()?;

                // Get the HTTP Client
                // We need to CLONE it so we can give a copy to the Lua function.
                // ctx.http() returns a reference, so we clone the Arc to own it.
                let http_client = ctx.http.clone();

                // Create the 'say' function
                // We use 'create_function_mut' because we are moving 'http_client' into it.
                let say_fn = lua.create_function(move |_, (channel_id, text): (u64, String)| {
                    // We have to clone http_client AGAIN because this function might be called multiple times
                    let http = http_client.clone();

                    // Spawn a new Tokio task to handle the network request
                    // We do this because Lua is synchronous, but Discord is async.
                    // This is "Fire and Forget."
                    tokio::spawn(async move {
                        let channel = serenity::ChannelId::new(channel_id);
                        if let Err(e) = channel.say(&http, text).await {
                            println!("Error sending message from Lua: {}", e);
                        }
                    });

                    Ok(())
                })?;

                // Get the HTTP client (we need a clone for this new function)
                let http_client_embed = ctx.http.clone();

                // Create the 'send_embed' function
                let send_embed =
                    lua.create_function(move |lua, (channel_id, embed_data): (u64, LuaValue)| {
                        // A. The Magic Trick: Convert Lua Table -> Rust Struct
                        // We use "from_value" to ask Serde to parse the Lua table.
                        let embed_struct: LuaEmbed =
                            lua.from_value(embed_data).map_err(mlua::Error::external)?;

                        // B. Build the Discord Embed (Serenity Builder Pattern)
                        // We start with a blank CreateEmbed and add fields one by one.
                        let mut builder = serenity::CreateEmbed::default();

                        if let Some(t) = embed_struct.title {
                            builder = builder.title(t);
                        }
                        if let Some(d) = embed_struct.description {
                            builder = builder.description(d);
                        }
                        if let Some(c) = embed_struct.color {
                            builder = builder.color(serenity::Color::from(c));
                        }
                        if let Some(u) = embed_struct.url {
                            builder = builder.url(u);
                        }
                        if let Some(i) = embed_struct.image {
                            builder = builder.image(i);
                        }
                        if let Some(th) = embed_struct.thumbnail {
                            builder = builder.thumbnail(th);
                        }

                        if let Some(f) = embed_struct.footer {
                            let mut footer = serenity::CreateEmbedFooter::new(f.text);
                            if let Some(icon) = f.icon_url {
                                footer = footer.icon_url(icon);
                            }
                            builder = builder.footer(footer);
                        }

                        if let Some(fields) = embed_struct.fields {
                            for field in fields {
                                builder = builder.field(
                                    field.name,
                                    field.value,
                                    field.inline.unwrap_or(false),
                                );
                            }
                        }

                        // C. Send it! (Async Fire-and-Forget)
                        let http = http_client_embed.clone();
                        tokio::spawn(async move {
                            let channel = serenity::ChannelId::new(channel_id);
                            // We use create_message to send the embed
                            let msg = serenity::CreateMessage::new().embed(builder);

                            if let Err(e) = channel.send_message(&http, msg).await {
                                println!("❌ Failed to send embed: {}", e);
                            }
                        });

                        Ok(())
                    })?;

                // --- DATABASE MODULE ---
                let db_table = lua.create_table()?;

                // Function 1: SET (key, value)
                let db_conn_set = db.clone(); // Clone ARC for the closure
                let db_set = lua.create_function(move |_, (key, value): (String, String)| {
                    let conn = db_conn_set.lock().unwrap();
                    // SQL: Insert or Update if key exists
                    conn.execute(
                        "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?1, ?2)",
                        (key, value),
                    )
                    .map_err(mlua::Error::external)?;
                    Ok(())
                })?;
                db_table.set("set", db_set)?;

                // Function 2: GET (key) -> Returns String or nil
                let db_conn_get = db.clone();
                let db_get = lua.create_function(move |lua, key: String| {
                    let conn = db_conn_get.lock().unwrap();

                    // Prepare the query
                    let mut stmt = conn
                        .prepare("SELECT value FROM kv_store WHERE key = ?1")
                        .map_err(mlua::Error::external)?;

                    // Execute and get result
                    // query_row returns an error if not found, so we handle that via Optional
                    let result: Option<String> = stmt
                        .query_row([key], |row| row.get(0))
                        .optional()
                        .map_err(mlua::Error::external)?;

                    // If found, return the string. If not, return nil (None)
                    match result {
                        Some(val) => Ok(mlua::Value::String(lua.create_string(&val)?)),
                        None => Ok(mlua::Value::Nil),
                    }
                })?;
                db_table.set("get", db_get)?;

                // Attach the function to the 'navi' table
                // In Lua, this becomes: navi.say(id, text)
                navi.set("say", say_fn)?;
                navi.set("send_embed", send_embed)?;
                navi.set("db", db_table)?;

                // --- EVENT BUS SYSTEM ---

                // 1. Create a table to hold all listener functions
                // effectively: navi.listeners = {}
                let listeners = lua.create_table()?;
                navi.set("listeners", listeners)?;

                // 2. Create the 'register' function
                // Usage in Lua: navi.register(function(msg) ... end)
                let register_fn = lua.create_function(|lua, func: Function| {
                    // Get the 'navi' table
                    let navi: Table = lua.globals().get("navi")?;
                    // Get the 'listeners' list inside it
                    let listeners: Table = navi.get("listeners")?;

                    // Add the new function to the end of the list
                    let len = listeners.len()?;
                    listeners.set(len + 1, func)?;

                    Ok(())
                })?;
                navi.set("register", register_fn)?;

                // --- COMMAND REGISTRY SYSTEM ---

                // 1. Create a table to hold named commands
                // effectively: navi.commands = {}
                let commands = lua.create_table()?;
                navi.set("commands", commands)?;

                // 2. Create the 'create_command' function
                // Usage: navi.create_command("ping", function(msg, args) ... end)
                let command_fn = lua.create_function(|lua, (name, func): (String, Function)| {
                    let navi: Table = lua.globals().get("navi")?;
                    let commands: Table = navi.get("commands")?;
                    
                    // FIX: We use 'name.clone()' here so we don't give away the original
                    commands.set(name.clone(), func)?;
                    
                    // Now we can still use 'name' because we only gave away a copy!
                    println!("   > Registered command: !{}", name); 
                    Ok(())
                })?;
                navi.set("create_command", command_fn)?;

                // 3. Define the MASTER on_message function in Lua
                // This acts as the "Conductor". Rust calls THIS function,
                // and this function calls everyone else.
                lua.load(
                    r#"
                            function on_message(msg)
                                -- Loop through every registered listener
                                if navi.listeners then
                                    for i, listener in ipairs(navi.listeners) do
                                        -- Use pcall (Protected Call) so one buggy plugin doesn't crash the bot
                                        local success, err = pcall(listener, msg)
                                        if not success then
                                            print("Error in plugin listener: " .. tostring(err))
                                        end
                                    end
                                end
                            end
                        "#,
                )
                .exec()?;

                // --- SLASH COMMAND SYSTEM ---

                // 1. Create a table to store Slash Definitions (name -> {desc, func})
                let slash_cmds = lua.create_table()?;
                navi.set("slash_commands", slash_cmds)?;

                // 2. Create 'create_slash' function
                // Usage: navi.create_slash("ping", "Description", function(ctx) ... end)
                let slash_fn = lua.create_function(|lua, (name, desc, func): (String, String, Function)| {
                    let navi: Table = lua.globals().get("navi")?;
                    let slash_cmds: Table = navi.get("slash_commands")?;
                    
                    // We need to store both the Description (for Discord) and the Function (for us)
                    let cmd_data = lua.create_table()?;
                    cmd_data.set("description", desc)?;
                    cmd_data.set("callback", func)?;
                    
                    slash_cmds.set(name.clone(), cmd_data)?;
                    println!("   > Registered Slash: /{}", name);
                    Ok(())
                })?;
                navi.set("create_slash", slash_fn)?;

                // Make 'navi' global
                lua.globals().set("navi", navi)?;

                // Wrap Lua in Arc<Mutex> and return it
                Ok(Data {
                    lua: Arc::new(Mutex::new(lua)),
                    db: db,
                })
            })
        })
        .build();

    // Connect to Discord
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::Client::builder(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}

#[poise::command(prefix_command, owners_only)]
async fn run_lua(ctx: Context<'_>, #[rest] code: String) -> Result<(), Error> {
    // STEP 1: Run the Lua code in a separate block
    // The result (Ok or Err) will be saved to 'result'
    // The moment this block ends (}), the Mutex is UNLOCKED.
    let result = {
        let lua = ctx.data().lua.lock().unwrap();
        lua.load(&code).exec() // We return the result of the execution
    };
    // <--- The 'lua' variable dies here. The lock is released.

    // STEP 2: Now we can safely await, because we aren't holding the lock.
    match result {
        Ok(_) => {
            ctx.say("✅ Lua executed successfully.").await?;
        }
        Err(e) => {
            ctx.say(format!("❌ Lua Error: ```{}```", e)).await?;
        }
    }

    Ok(())
}

#[poise::command(prefix_command, owners_only)]
async fn reload(ctx: Context<'_>) -> Result<(), Error> {
    // STEP 1: Do all the "Logic" inside a block
    // We will return a String message to send later.
    let report_message = {
        let _data = ctx.data();
        let lua = ctx.data().lua.lock().unwrap();

        // 1. CLEAR THE EVENT BUS (The Fix)
        let navi: mlua::Table = lua.globals().get("navi")?;  // Get navi
        navi.set("listeners", lua.create_table()?)?;

        // 2. Clear the Package Cache (Optional but recommended)
        // This forces Lua to re-read files that use 'require'
        let globals = lua.globals();
        let package: mlua::Table = globals.get("package")?;
        let _loaded: mlua::Table = package.get("loaded")?;
        // You can iterate and clear 'loaded' if you use require(), 
        // but for now, clearing 'listeners' solves the immediate problem.

        // We use std::fs because we are inside a sync block now
        let paths = std::fs::read_dir("plugins")?;
        let mut count = 0;
        let mut error_msg = None;

        for path in paths {
            let path = path?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                let code = std::fs::read_to_string(&path)?;

                // Try to load and execute
                let chunk = lua.load(&code).set_name(path.to_string_lossy());

                if let Err(e) = chunk.exec() {
                    // If it fails, save the error and STOP loading
                    error_msg = Some(format!("❌ Error in {:?}: \n```{}```", path, e));
                    break;
                }

                count += 1;
                println!("Loaded: {:?}", path);
            }
        }

        // Return either the error or the success message
        if let Some(err) = error_msg {
            err
        } else {
            format!("✅ Reloaded {} plugins!", count)
        }
    }; // <--- MUTEX IS DROPPED HERE (The Lock is released)

    
    // STEP 2: Now we can safely await because the lock is gone
    ctx.say(report_message).await?;

    Ok(())
}

#[poise::command(prefix_command, owners_only)]
async fn sync(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();

    // STEP 1: Get data from Lua (Synchronous)
    // We wrap this in a block { ... } so the lock is DROPPED immediately after.
    let commands_builder = {
        let lua = data.lua.lock().unwrap();
        
        let navi: Table = lua.globals().get("navi")?;
        let slash_cmds: Table = navi.get("slash_commands")?;

        let mut commands = Vec::new();

        for pair in slash_cmds.pairs::<String, Table>() {
            let (name, data) = pair?;
            let desc: String = data.get("description")?;
            
            // Build the command struct purely in memory
            let command = serenity::CreateCommand::new(name).description(desc);
            commands.push(command);
        }
        commands 
    }; // <--- 🔓 LOCK IS DROPPED HERE!

    // STEP 2: Talk to Discord (Asynchronous)
    // Now we are safe to .await because we aren't holding the Lua lock anymore.
    let http = ctx.http();
    
    if let Some(guild_id) = ctx.guild_id() {
        ctx.say("⏳ Syncing commands to this server...").await?;
        
        // This takes time, but it's okay because we aren't blocking Lua!
        guild_id.set_commands(http, commands_builder).await?;
        
        ctx.say("✅ Slash commands synced! Try typing /").await?;
    } else {
        ctx.say("❌ Please run this in a server, not DMs.").await?;
    }

    Ok(())
}