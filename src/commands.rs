use crate::types::{Context, Error};
use poise::serenity_prelude as serenity;
use mlua::prelude::*;

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

    // 1. Build Commands from Lua
    let commands_builder = {
        let lua = data.lua.lock().unwrap();
        let navi: LuaTable = lua.globals().get("navi")?;
        let slash_cmds: LuaTable = navi.get("slash_commands")?;

        let mut commands = Vec::new();
        for pair in slash_cmds.pairs::<String, LuaTable>() {
            let (name, data) = pair?;
            let desc: String = data.get("description")?;
            let command = serenity::CreateCommand::new(name).description(desc);
            commands.push(command);
        }
        commands 
    };

    // 2. Sync to Discord
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