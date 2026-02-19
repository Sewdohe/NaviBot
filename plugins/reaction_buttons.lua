print("--- Loading Button Roles ---")

-- CONFIG: Map Button IDs to Role IDs
local button_roles = {
    ["role_java"] = { id = "111111111", label = "Java Dev" },
    ["role_rust"] = { id = "222222222", label = "Rustacean" },
    ["role_lua"]  = { id = "333333333", label = "Lua Scripter" }
}

-- 1. Setup Command (Admins run this)
navi.create_slash("setup_roles", "Send the role menu", {}, function(ctx)
    local buttons = {}
    
    -- Generate buttons dynamically
    for btn_id, data in pairs(button_roles) do
        table.insert(buttons, {
            type = "button",
            label = "Get " .. data.label,
            id = btn_id,
            style = "primary"
        })
    end

    navi.send_message(ctx.channel_id, {
        title = "🎭 Choose your Class",
        description = "Click a button below to equip a role!",
        color = 0x5865F2,
        components = buttons
    })
    
    ctx.reply("✅ Menu sent!")
end)

-- 2. Handle Clicks
-- The engine calls this global function when a button is clicked
---@param ctx ComponentContext
function on_component(ctx)
    local role_data = button_roles[ctx.custom_id]
    
    if role_data then
        -- Assume we are in a guild (we'd need guild_id in the event, 
        -- but for now let's assume the button was sent in the guild)
        -- Note: You might need to add `guild_id` to the event table in Rust!
        
        -- For now, let's just log it
        print("User " .. ctx.user_id .. " clicked " .. ctx.custom_id)
        
        -- Logic: We'd call navi.add_role here.
        -- navi.add_role(ctx.guild_id, ctx.user_id, role_data.id)
        
        -- Reply Ephemerally (True = only they see it)
        ctx.reply("✅ You selected: " .. role_data.label, true)
    else
        ctx.reply("❌ Unknown button.", true)
    end
end