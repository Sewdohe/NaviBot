print("--- Loading Slash Module ---")

-- Create the definition
navi.create_slash("hello", "Says hello back to you!", function(ctx)
    -- Instead of 'msg.channel_id', we use the 'ctx' object we built
    ctx.reply("👋 Hello " .. ctx.username .. "! This is a slash command.")
end)

navi.create_slash("gimme", "Gives you a random item", function(ctx)
    local items = {"🍎 Apple", "⚔️ Sword", "🛡️ Shield", "💰 Coin"}
    local item = items[math.random(#items)]
    ctx.reply("You received: " .. item)
end)