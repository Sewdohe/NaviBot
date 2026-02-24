navi.register_config("starboard", {
    { key = "channel",   name = "Starboard Channel", description = "Channel where starred messages are posted", type = "channel", default = "" },
    { key = "threshold", name = "Star Threshold",    description = "Reactions needed to appear on starboard",  type = "number",  default = 3 },
    { key = "emoji",     name = "Star Emoji",        description = "Which emoji counts as a star",             type = "string",  default = "⭐" }
})

function on_reaction_add(ctx)
    local sb_ch     = navi.db.get("config:starboard:channel")
    local threshold = tonumber(navi.db.get("config:starboard:threshold")) or 3
    local emoji     = navi.db.get("config:starboard:emoji") or "⭐"

    if not sb_ch or sb_ch == "" then return end
    if ctx.emoji ~= emoji then return end
    if ctx.channel_id == sb_ch then return end  -- don't star the starboard itself

    local count_key = "starboard:count:" .. ctx.message_id
    local post_key  = "starboard:post:"  .. ctx.message_id

    local count = (tonumber(navi.db.get(count_key)) or 0) + 1
    navi.db.set(count_key, tostring(count))

    local guild = ctx.guild_id or "@me"
    local jump  = "https://discord.com/channels/" .. guild .. "/" .. ctx.channel_id .. "/" .. ctx.message_id
    local text  = emoji .. " **" .. count .. "** | <#" .. ctx.channel_id .. "> | [Jump](" .. jump .. ")"

    local existing_id = navi.db.get(post_key)
    if existing_id then
        navi.edit_message(sb_ch, existing_id, text)
    elseif count >= threshold then
        local msg_id = navi.say_sync(sb_ch, text)
        if msg_id then
            navi.db.set(post_key, msg_id)
        end
    end
end
