print("--- Loading Reaction Roles Plugin ---")

-- 1. Register the Config Schema
navi.register_config("reaction_roles", {{
    key = "channel_id",
    name = "Menu Channel",
    description = "The channel to spawn the role menu in",
    type = "channel",
    default = ""
}, {
    key = "role_1_name",
    name = "Role 1 Name",
    description = "The name shown in the dropdown",
    type = "string",
    default = "Announcements"
}, {
    key = "role_1_id",
    name = "Role 1 ID",
    description = "The actual Discord Role ID",
    type = "role",
    default = ""
}, {
    key = "role_2_name",
    name = "Role 2 Name",
    description = "The name shown in the dropdown",
    type = "string",
    default = "Events"
}, {
    key = "role_2_id",
    name = "Role 2 ID",
    description = "The actual Discord Role ID",
    type = "role",
    default = ""
}})

-- 2. Slash Command to Spawn the Menu
navi.create_slash("spawn_roles", "Spawns the self-assign role menu", {},
---@param ctx NaviCtx
function(ctx)
    local channel_id = navi.db.get("config:reaction_roles:channel_id")

    if channel_id == "" or channel_id == nil then
        ctx.reply("❌ Please configure the Menu Channel in the TUI first!", true)
        return
    end

    local r1_name = navi.db.get("config:reaction_roles:role_1_name") or "Role 1"
    local r2_name = navi.db.get("config:reaction_roles:role_2_name") or "Role 2"

    -- Spawn the Dropdown Menu!
    navi.send_message(channel_id, {
        title = "🎭 Self-Assign Roles",
        description = "Use the dropdown menu below to select the roles you want.",
        color = 0x9B59B6,
        components = {{
            type = "select",
            id = "role_dropdown",
            placeholder = "Pick a role...",
            options = {{
                label = r1_name,
                value = "role_1",
                description = "Click to receive " .. r1_name,
                emoji = "✨"
            }, {
                label = r2_name,
                value = "role_2",
                description = "Click to receive " .. r2_name,
                emoji = "🔥"
            }}
        }}
    })

    ctx.reply("✅ Role menu spawned successfully!", true)
end)

-- 3. Handle the Dropdown Selection (Using the new clean API!)
navi.register_component("role_dropdown", 
---@param ctx NaviCtx
function(ctx)
    local choice = ctx.values[1]
    local role_id = ""
    local role_name = ""

    -- Match their choice to the TUI config
    if choice == "role_1" then
        role_id = navi.db.get("config:reaction_roles:role_1_id")
        role_name = navi.db.get("config:reaction_roles:role_1_name")
    elseif choice == "role_2" then
        role_id = navi.db.get("config:reaction_roles:role_2_id")
        role_name = navi.db.get("config:reaction_roles:role_2_name")
    end

    -- Safety check
    if role_id == "" or role_id == nil then
        ctx.reply("❌ This role hasn't been fully configured by the admin yet.", true)
        return
    end

    -- Give them the role!
    navi.add_role(ctx.guild_id, ctx.user_id, role_id)
    ctx.reply("✅ You have been given the **" .. role_name .. "** role!", true)
end)
