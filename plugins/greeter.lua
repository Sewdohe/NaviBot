navi.log.info("Loading Greeter Plugin")

-- 1. Register the Config Schema for the TUI
navi.register_config("greeter", {
    { key = "enabled", name = "Enable Greeter", description = "Toggle welcome messages on or off", type = "boolean", default = false },
    { key = "channel_id", name = "Welcome Channel", description = "The channel to send welcome messages to", type = "channel", default = "" },
    { key = "title", name = "Embed Title", description = "The title of the welcome embed", type = "string", default = "Welcome to the Server!" },
    { key = "message", name = "Welcome Message", description = "Use {user} to ping the new member", type = "string", default = "Hello {user}, we're glad you are here!" },
    { key = "use_embed", name = "Use Embed", description = "Send as a rich embed (true) or plain text (false)", type = "boolean", default = true },
    { key = "color", name = "Embed Color", description = "Hex color code (Decimal format. Default is Blurple)", type = "number", default = 5793266 }
})

-- 2. The Logic (Called by Rust when someone joins)
navi.on("member_join", function(data)
    local user_id = data.user_id
    local username = data.username
    -- emit an event for new member joining.
    navi.emit("new_member_joined", username)

    -- Fetch current settings from the DB
    local enabled = navi.db.get("config:greeter:enabled")
    if enabled == "false" or enabled == nil then return end

    local channel_id = navi.db.get("config:greeter:channel_id")
    if channel_id == "" or channel_id == nil then
        navi.log.warn("Greeter triggered but no channel is configured")
        return 
    end

    -- Fetch the rest of the settings
    local title = navi.db.get("config:greeter:title") or "Welcome!"
    local raw_msg = navi.db.get("config:greeter:message") or "Hello {user}!"
    local use_embed = navi.db.get("config:greeter:use_embed")
    local color_val = tonumber(navi.db.get("config:greeter:color")) or 5793266

    -- Replace {user} with a Discord ping
    local formatted_msg = raw_msg:gsub("{user}", "<@" .. user_id .. ">")

    -- Send the message!
    if use_embed == "true" then
        navi.send_message(channel_id, {
            title = title,
            description = formatted_msg,
            color = color_val
        })
    else
        -- navi.say expects a number for the channel_id, so we convert it
        navi.say(tonumber(channel_id), formatted_msg)
    end
end)
