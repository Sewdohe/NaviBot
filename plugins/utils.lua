print("--- Loading Utils ---")

-- Parse a hex color string like "#3498DB" or "0x3498DB" to a number
local function parse_color(s)
    if not s or s == "" then return nil end
    s = s:gsub("^#", ""):gsub("^0[xX]", "")
    return tonumber(s, 16)
end

-- Build a NaviEmbed table from modal submitted values
local function build_embed(vals)
    local embed = {}
    if vals.title ~= "" then embed.title = vals.title end
    if vals.description ~= "" then embed.description = vals.description end
    local color = parse_color(vals.color)
    if color then embed.color = color end
    if vals.image ~= "" then embed.image = vals.image end
    return embed
end

-- /sendembed: pick a channel, fill out the design modal, then send
navi.create_slash("sendembed", "Send a custom embed to a channel", {
    { name = "channel", description = "The channel to send the embed to", type = "channel", required = true }
}, function(ctx)
    if not perms.require(ctx, "admin") then
        return
    end
    navi.db.set("pending_send_" .. ctx.user_id, ctx.args.channel)
    ctx.modal("modal_sendembed", "Design Embed", {
        { id = "title",       label = "Title",       style = "short",     placeholder = "My Embed",           required = false },
        { id = "description", label = "Description", style = "paragraph", placeholder = "Embed body text...", required = false },
        { id = "color",       label = "Color (hex)", style = "short",     placeholder = "#3498DB",            required = false },
        { id = "image",       label = "Image URL",   style = "short",     placeholder = "https://...",        required = false },
    })
end)

navi.register_modal("modal_sendembed", function(ctx)
    local channel_id = navi.db.get("pending_send_" .. ctx.user_id)
    if not channel_id or channel_id == "" then
        ctx.reply("❌ Session expired. Please run /sendembed again.", true)
        return
    end

    local vals = ctx.values
    local embed = build_embed(vals)

    if not embed.title and not embed.description then
        ctx.reply("❌ An embed needs at least a Title or Description.", true)
        return
    end

    -- Store the embed data so /editembed can retrieve it later
    navi.db.set("embed_" .. channel_id, navi.json.encode({
        title       = vals.title       or "",
        description = vals.description or "",
        color       = vals.color       or "",
        image       = vals.image       or "",
    }))

    navi.send_message(channel_id, embed)
    ctx.reply("✅ Embed sent to <#" .. channel_id .. ">!", true)
    navi.db.set("pending_send_" .. ctx.user_id, "")
end)

-- /editembed: pull the stored embed for a channel, open edit modal, then re-send
navi.create_slash("editembed", "Edit and re-send a stored embed", {
    { name = "channel", description = "The channel the embed was originally sent to", type = "channel", required = true }
}, function(ctx)
    local channel_id = ctx.args.channel
    local json = navi.db.get("embed_" .. channel_id)
    if not json or json == "" then
        ctx.reply("❌ No stored embed found for that channel. Use /sendembed first.")
        return
    end

    local stored = navi.json.decode(json)
    navi.db.set("pending_edit_" .. ctx.user_id, channel_id)

    -- Use the current values as placeholders so the user knows what they're replacing
    ctx.modal("modal_editembed", "Edit Embed", {
        { id = "title",       label = "Title",       style = "short",     placeholder = (stored.title ~= "" and stored.title) or "My Embed",           required = false },
        { id = "description", label = "Description", style = "paragraph", placeholder = (stored.description ~= "" and stored.description) or "...",      required = false },
        { id = "color",       label = "Color (hex)", style = "short",     placeholder = (stored.color ~= "" and stored.color) or "#3498DB",             required = false },
        { id = "image",       label = "Image URL",   style = "short",     placeholder = (stored.image ~= "" and stored.image) or "https://...",         required = false },
    })
end)

navi.register_modal("modal_editembed", function(ctx)
    local channel_id = navi.db.get("pending_edit_" .. ctx.user_id)
    if not channel_id or channel_id == "" then
        ctx.reply("❌ Session expired. Please run /editembed again.", true)
        return
    end

    local vals = ctx.values
    local embed = build_embed(vals)

    if not embed.title and not embed.description then
        ctx.reply("❌ An embed needs at least a Title or Description.", true)
        return
    end

    -- Update the stored embed
    navi.db.set("embed_" .. channel_id, navi.json.encode({
        title       = vals.title       or "",
        description = vals.description or "",
        color       = vals.color       or "",
        image       = vals.image       or "",
    }))

    navi.send_message(channel_id, embed)
    ctx.reply("✅ Embed updated and re-sent to <#" .. channel_id .. ">!", true)
    navi.db.set("pending_edit_" .. ctx.user_id, "")
end)
