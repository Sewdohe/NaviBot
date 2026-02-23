print("--- Loading Reaction Roles Plugin ---")

-- 1. Register Config Schema
navi.register_config("reaction_roles", {
    {
        key = "channel_id",
        name = "Menu Channel",
        description = "The channel to spawn the role menu in",
        type = "channel",
        default = ""
    },
    {
        key = "mappings",
        name = "Role Mappings",
        description = "Each entry defines one option in the dropdown",
        type = "list",
        item_schema = {
            { key = "name",    name = "Display Name", type = "string" },
            { key = "role_id", name = "Role",         type = "role"   }
        }
    }
})

-- 2. Slash Command to Spawn the Menu
navi.create_slash("spawn_roles", "Spawns the self-assign role menu", {},
    ---@param ctx NaviSlashCtx
    function(ctx)
        local channel_id = navi.db.get("config:reaction_roles:channel_id")

        if channel_id == nil or channel_id == "" then
            ctx.reply("❌ Please configure the Menu Channel in the TUI first!")
            return
        end

        local mappings = navi.db.get_list("mappings")

        if #mappings == 0 then
            ctx.reply("❌ No role mappings configured yet. Add some in the TUI config!")
            return
        end

        -- Build dropdown options from the list
        local options = {}
        for i, entry in ipairs(mappings) do
            local display = entry.name or ("Role " .. i)
            table.insert(options, {
                label = display,
                value = tostring(i - 1),   -- 0-based index matched in component handler
                description = "Click to receive the " .. display .. " role",
                emoji = "🎭"
            })
        end

        navi.send_message(channel_id, {
            title = "🎭 Self-Assign Roles",
            description = "Use the dropdown below to pick a role.",
            color = 0x9B59B6,
            components = {{
                type = "select",
                id = "role_dropdown",
                placeholder = "Pick a role...",
                options = options
            }}
        })

        ctx.reply("✅ Role menu spawned in <#" .. channel_id .. ">!")
    end
)

-- 3. Handle Dropdown Selection
---@param ctx NaviComponentCtx
local function on_role_dropdown(ctx)
    local choice = ctx.values[1]
    local idx = tonumber(choice)

    if idx == nil then
        ctx.reply("❌ Invalid selection.", true)
        return
    end

    local mappings = navi.db.get_list("mappings")
    local entry = mappings[idx + 1]   -- convert 0-based back to 1-based Lua index

    if entry == nil or entry.role_id == nil or entry.role_id == "" then
        ctx.reply("❌ This role hasn't been configured by an admin yet.", true)
        return
    end

    navi.add_role(ctx.guild_id, ctx.user_id, entry.role_id)
    navi.log.info("Gave the " .. entry.name .. " role to " .. ctx.username)
    ctx.reply("✅ You've been given the **" .. (entry.name or "role") .. "** role!", true)
end

navi.register_component("role_dropdown", on_role_dropdown)
