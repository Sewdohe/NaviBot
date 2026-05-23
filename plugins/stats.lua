navi.log.info("Loading Bot Statistics Plugin")

-- ---------------------------------------------------------------------------
-- Counters — updated by event listeners and other plugins via the event bus
-- ---------------------------------------------------------------------------

navi.on("member_join", function(data)
    local current = tonumber(navi.db.get("stats:total_joins")) or 0
    navi.db.set("stats:total_joins", tostring(current + 1))
end)

navi.on("ticket_created", function()
    local current = tonumber(navi.db.get("stats:total_tickets")) or 0
    navi.db.set("stats:total_tickets", tostring(current + 1))
end)

navi.on("message", function()
    local current = tonumber(navi.db.get("stats:total_messages")) or 0
    navi.db.set("stats:total_messages", tostring(current + 1))
end)

navi.on("leveling:levelup", function()
    local current = tonumber(navi.db.get("stats:total_levelups")) or 0
    navi.db.set("stats:total_levelups", tostring(current + 1))
end)

navi.on("economy:transfer", function()
    local current = tonumber(navi.db.get("stats:total_transfers")) or 0
    navi.db.set("stats:total_transfers", tostring(current + 1))
end)

-- ---------------------------------------------------------------------------
-- Embed builder
-- ---------------------------------------------------------------------------

local function build_embed()
    local joins     = navi.db.get("stats:total_joins")     or "0"
    local tickets   = navi.db.get("stats:total_tickets")   or "0"
    local messages  = navi.db.get("stats:total_messages")  or "0"
    local counts    = navi.db.get("counting:total_counts")  or "0"
    local highest   = navi.db.get("counting:highest_count") or "0"
    local levelups  = navi.db.get("stats:total_levelups")  or "0"
    local transfers = navi.db.get("stats:total_transfers") or "0"

    -- Aggregate totals queried directly from the DB
    local xp_rows  = navi.db.query("SELECT SUM(CAST(value AS INTEGER)) as total FROM kv_store WHERE key LIKE 'leveling:xp:%'")
    local bal_rows = navi.db.query("SELECT SUM(CAST(value AS INTEGER)) as total FROM kv_store WHERE key LIKE 'economy:balance:%'")
    local total_xp  = (xp_rows[1]  and xp_rows[1].total)  or "0"
    local total_bal = (bal_rows[1] and bal_rows[1].total) or "0"

    local currency = navi.db.get("config:economy:currency_name") or "Credits"

    return {
        title       = "📊 Server Statistics",
        color       = 0x5865F2,
        description = "Live server metrics — updates every 5 minutes.",
        fields      = {
            -- General
            { name = "👥 Total Joins",            value = joins,    inline = true },
            { name = "🎫 Tickets Opened",          value = tickets,  inline = true },
            { name = "💬 Messages Tracked",        value = messages, inline = true },
            -- Leveling
            { name = "⭐ XP in Circulation",       value = tostring(total_xp),  inline = true },
            { name = "🆙 Total Level-Ups",         value = levelups,            inline = true },
            -- Economy
            { name = "💰 " .. currency .. " in Circulation", value = tostring(total_bal), inline = true },
            { name = "💸 Transfers Made",          value = transfers,           inline = true },
            -- Counting
            { name = "🔢 Total Counts",            value = counts,   inline = true },
            { name = "🏆 Highest Count",           value = highest,  inline = true },
        },
    }
end

-- ---------------------------------------------------------------------------
-- Auto-update interval
-- ---------------------------------------------------------------------------

-- Refresh the pinned stats embed every 5 minutes if one has been posted.
navi.set_interval(function()
    local channel_id = navi.db.get("stats:live_channel_id")
    local message_id = navi.db.get("stats:live_message_id")
    if channel_id and message_id then
        navi.edit_embed(channel_id, message_id, build_embed())
    end
end, 5, "m")

-- ---------------------------------------------------------------------------
-- /stats  (admin-only)
-- ---------------------------------------------------------------------------

navi.create_slash("stats", "Post a live-updating server statistics embed (admin only)", {}, function(ctx)
    if not navi.require_perm(ctx, "admin") then return end

    -- Delete the previous live embed if one exists
    local old_channel = navi.db.get("stats:live_channel_id")
    local old_message = navi.db.get("stats:live_message_id")
    if old_channel and old_message then
        navi.delete_message(old_channel, old_message)
    end

    -- Post the new embed and store its location
    local message_id = navi.send_message_sync(ctx.channel_id, build_embed())
    if message_id then
        navi.db.set("stats:live_channel_id", ctx.channel_id)
        navi.db.set("stats:live_message_id", message_id)
        ctx.reply("📊 Stats embed posted! It will update automatically every 5 minutes.", true)
    else
        ctx.reply("❌ Failed to post the stats embed.", true)
    end
end)
