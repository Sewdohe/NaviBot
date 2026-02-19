print("--- Loading Embed Builder ---")

navi.create_slash("embed", "Send a custom embed", {
    { name = "title", description = "Title of the embed", type = "string", required = true },
    { name = "description", description = "Main text", type = "string", required = true },
    { name = "color", description = "Hex Color (e.g. #FF0000)", type = "string", required = false },
    { name = "image", description = "Big Image URL", type = "string", required = false },
    { name = "thumbnail", description = "Small Thumbnail URL", type = "string", required = false }
}, function(ctx)
    
    -- 1. Handle Color Conversion (Hex String -> Integer)
    local color_int = 0x5865F2 -- Default Blurple
    if ctx.args.color then
        -- Remove '#' if present
        local hex = ctx.args.color:gsub("#", "")
        -- Convert hex string to number
        color_int = tonumber(hex, 16) or 0x000000
    end

    -- 2. Build the Embed Table
    local embed = {
        title = ctx.args.title,
        description = ctx.args.description,
        color = color_int,
        image = ctx.args.image,
        thumbnail = ctx.args.thumbnail
    }

    -- 3. Send it!
    -- We use the new channel_id we just added to Rust
    navi.send_embed(ctx.channel_id, embed)
    
    -- 4. Confirm to the user (Slash commands always need a reply)
    ctx.reply("✅ Embed sent!")
end)