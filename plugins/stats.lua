navi.log.info("Loading Bot Statistics Plugin")

-- 1. Listen for new members joining
navi.on("member_join", function(data)
    local current = tonumber(navi.db.get("stats:total_joins")) or 0
    navi.db.set("stats:total_joins", tostring(current + 1))
    navi.log.info("Stats: new join for " .. data.username .. ", total: " .. tostring(current + 1))
end)

-- 2. Listen for tickets being opened
navi.on("ticket_created", function(user_id)
    local current = tonumber(navi.db.get("stats:total_tickets")) or 0
    navi.db.set("stats:total_tickets", tostring(current + 1))
    navi.log.info("Stats: new ticket, total: " .. tostring(current + 1))
end)

navi.on("message", function(msg)
    navi.log.info("a message was sent")
    local current = tonumber(navi.db.get("stats:total_messages")) or 0
    navi.db.set("stats:total_messages", tostring(current + 1))
end)

-- 3. A command to view the stats!
navi.create_slash("stats", "View bot and server statistics", {},
---@param ctx NaviSlashCtx
function(ctx)
    local joins    = navi.db.get("stats:total_joins")    or "0"
    local tickets  = navi.db.get("stats:total_tickets")  or "0"
    local messages = navi.db.get("stats:total_messages")  or "0"

    -- Counting stats are stored by the counting plugin under its own namespace
    local count_total   = navi.db.get("counting:total_counts")  or "0"
    local count_highest = navi.db.get("counting:highest_count") or "0"

    local output = "📊 **Server Stats**\n"
    output = output .. "👥 Total Joins: `"        .. joins          .. "`\n"
    output = output .. "🎫 Tickets Opened: `"     .. tickets        .. "`\n"
    output = output .. "💬 Messages Sent: `"      .. messages       .. "`\n"
    output = output .. "🔢 Total Counts: `"       .. count_total    .. "`\n"
    output = output .. "🏆 Highest Count Ever: `" .. count_highest  .. "`"

    ctx.reply(output)
end)
