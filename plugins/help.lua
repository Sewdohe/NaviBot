print("--- Loading Help Module ---")

navi.create_slash("help", "List all available commands", {}, function(ctx)
    local fields = {}
    
    -- 1. READ THE REGISTRY
    -- We loop through the internal table where the engine stores commands
    for name, data in pairs(navi.slash_commands) do
        local desc = data.description or "No description provided."
        
        -- Add to our embed fields
        table.insert(fields, {
            name = "/" .. name,
            value = desc,
            inline = false
        })
    end

    -- 2. SORT THEM (Optional, but looks nicer)
    table.sort(fields, function(a, b) return a.name < b.name end)

    -- 3. BUILD THE EMBED
    -- We construct the raw JSON object for the embed
    local embed = {
        title = "📚 Navi Command List",
        description = "Here are the modules currently loaded into the engine:",
        color = 0x00FF00, -- Green
        fields = fields,
        footer = { text = "Powered by Rust + Lua Architecture" }
    }

    -- 4. SEND
    -- We use the helper we made earlier. 
    -- Note: We need to use 'reply' for slash commands, but 'send_embed' 
    -- is for channel messages. Let's make a quick custom reply logic.
    
    -- Since ctx.reply only takes a string, we can't send a fancy embed as a *reply* -- without editing Rust. BUT, we can just send it to the channel normally!
    navi.send_embed(ctx.channel_id, embed)
    
    -- Acknowledge the interaction so it doesn't fail
    ctx.reply("✅ Help menu sent below!")
end)