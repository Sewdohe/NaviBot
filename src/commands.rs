use crate::types::{Context, Error};
use crate::engine; // Import engine

#[poise::command(prefix_command, owners_only)]
pub async fn reload(ctx: Context<'_>) -> Result<(), Error> {
    let (_, report) = {
        let lua = ctx.data().lua.lock().unwrap();
        // Call the engine helper
        engine::load_plugins(&lua)
    };
    ctx.say(report).await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only)]
pub async fn sync(ctx: Context<'_>) -> Result<(), Error> {
    // 1. READ (Lock Lua, get data, Drop lock)
    let commands = {
        let lua = ctx.data().lua.lock().unwrap();
        engine::read_slash_commands(&lua)?
    }; 
    // <--- Lock is dropped here!

    // 2. UPLOAD (Async, safe to wait)
    ctx.say("⏳ Syncing commands...").await?;
    let report = engine::upload_slash_commands(ctx.http(), commands).await?;
    
    ctx.say(report).await?;
    Ok(())
}