print("--- Loading Ticket System ---")

-- 1. Register the TUI Settings
navi.register_config("tickets", {
    { key = "panel_channel", name = "Panel Channel", description = "Where the Create Ticket button goes", type = "channel", default = "" },
    { key = "ticket_category", name = "Ticket Category", description = "The category to spawn tickets under", type = "category", default = "" },
    { key = "support_role", name = "Support Role", description = "The role that can see tickets", type = "role", default = "" }
})

-- 2. The Slash Command to spawn the panel
navi.create_slash("spawn_tickets", "Spawns the ticket creation panel", {}, 
---@param ctx NaviSlashCtx
function(ctx)
    local channel_id = navi.db.get("config:tickets:panel_channel")
    if channel_id == "" or channel_id == nil then
        ctx.reply("❌ Please configure the Panel Channel first!", true)
        return
    end

    navi.send_message(channel_id, {
        title = "🛠️ Mod Support",
        description = "Need help with the mod? Click the button below to open a private ticket with our staff.",
        color = 0x3498DB,
        components = {
            { type = "button", id = "btn_create_ticket", label = "Create Ticket", style = 1, emoji = "🎫" }
        }
    })
    ctx.reply("✅ Ticket panel spawned!", true)
end)

-- 3. The Button Click Handler (Clean API!)
navi.register_component("btn_create_ticket", 
---@param ctx NaviComponentCtx
function(ctx)
    local category_id = navi.db.get("config:tickets:ticket_category")
    local support_role = navi.db.get("config:tickets:support_role")

    if category_id == "" or support_role == "" then
        ctx.reply("❌ The ticket category or support role isn't configured in the TUI yet.", true)
        return
    end

    local safe_name = string.gsub(string.lower(ctx.username), "[^%w_]", "")
    local ticket_name = "ticket-" .. safe_name
    
    local welcome_msg = string.format("Welcome <@%s>! The <@&%s> team will be with you shortly.\n\nPlease describe your issue in detail.", ctx.user_id, support_role)

    -- Pass the table of options to the generic Rust function we made!
    navi.create_channel(ctx.guild_id, ticket_name, {
        category_id = category_id,
        user_id = ctx.user_id,
        role_id = support_role,
        welcome_message = welcome_msg,
        close_button = true -- <--- Tell Rust to spawn the red button!
    })

    navi.emit("ticket_created", ctx.user_id)

    ctx.reply("✅ Your ticket is being created! Check your channel list.", true)
end)

navi.register_component("btn_close_ticket", 
---@param ctx NaviComponentCtx
function(ctx)
    -- We could add logic here to save a transcript first, 
    -- but for now, we just immediately delete the channel!
    
    navi.delete_channel(ctx.channel_id)
end)