use crate::types::{Context, Error};
use poise::serenity_prelude as serenity;
use mlua::prelude::*;
use poise::serenity_prelude::CreateCommandOption;

#[poise::command(prefix_command, owners_only)]
pub async fn run_lua(ctx: Context<'_>, #[rest] code: String) -> Result<(), Error> {
    let result = {
        let lua = ctx.data().lua.lock().unwrap();
        lua.load(&code).exec()
    };

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
pub async fn reload(ctx: Context<'_>) -> Result<(), Error> {
    let report_message = {
        let lua = ctx.data().lua.lock().unwrap();

        // 1. Reset Event Bus
        let navi: LuaTable = lua.globals().get("navi")?;
        navi.set("listeners", lua.create_table()?)?;

        // 2. Read Files
        let paths = std::fs::read_dir("plugins")?;
        let mut count = 0;
        let mut error_msg = None;

        for path in paths {
            let path = path?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                let code = std::fs::read_to_string(&path)?;
                let chunk = lua.load(&code).set_name(path.to_string_lossy());

                if let Err(e) = chunk.exec() {
                    error_msg = Some(format!("❌ Error in {:?}: \n```{}```", path, e));
                    break;
                }
                count += 1;
                println!("Loaded: {:?}", path);
            }
        }

        if let Some(err) = error_msg { err } else { format!("✅ Reloaded {} plugins!", count) }
    }; 
    
    ctx.say(report_message).await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only)]
pub async fn sync(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();

    let commands_builder = {
        let lua = data.lua.lock().unwrap();
        let navi: LuaTable = lua.globals().get("navi")?;
        let slash_cmds: LuaTable = navi.get("slash_commands")?;

        let mut commands = Vec::new();

        for pair in slash_cmds.pairs::<String, LuaTable>() {
            let (name, data) = pair?;
            let desc: String = data.get("description")?;
            
            let mut command = serenity::CreateCommand::new(name).description(desc);

            // HANDLE OPTIONS
            if let Ok(options) = data.get::<_, Vec<LuaTable>>("options") {
                for opt in options {
                    let name: String = opt.get("name")?;
                    let desc: String = opt.get("description")?;
                    let type_str: String = opt.get("type")?;
                    let required: bool = opt.get("required").unwrap_or(false);

                    // Map string types to Discord types
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
        commands 
    };

    // (The rest of the sync function stays the same)
    let http = ctx.http();
    if let Some(guild_id) = ctx.guild_id() {
        ctx.say("⏳ Syncing commands...").await?;
        guild_id.set_commands(http, commands_builder).await?;
        ctx.say("✅ Synced!").await?;
    } else {
        ctx.say("❌ Run in a server.").await?;
    }
    Ok(())
}