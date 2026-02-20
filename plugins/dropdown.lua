print("--- Loading Dropdown Plugin ---")

navi.create_slash("pick_class", "Select your RPG Class", {}, function(ctx)
    navi.send_message(ctx.channel_id, {
        title = "⚔️ Class Selection",
        description = "Choose your destiny from the menu below.",
        color = 0x9B59B6,
        components = {
            {
                type = "select",
                id = "class_select",
                placeholder = "Pick a class...",
                options = {
                    { label = "Warrior", value = "warrior", description = "Strong and brave", emoji = "⚔️" },
                    { label = "Mage", value = "mage", description = "Master of spells", emoji = "🔮" },
                    { label = "Rogue", value = "rogue", description = "Sneaky and fast", emoji = "🗡️" }
                }
            }
        }
    })
    ctx.reply("Menu sent!")
end)

-- Handle the selection
function on_component(ctx)
    if ctx.custom_id == "class_select" then
        -- ctx.values is an array of strings (because users can multi-select if configured)
        local choice = ctx.values[1]
        
        ctx.reply("✅ You have chosen path: **" .. choice .. "**", true)
    end
end