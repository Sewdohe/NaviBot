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
    local user_id = msg.author_id  -- available for future use
    local current = tonumber(navi.db.get("stats:total_messages")) or 0
    navi.db.set("stats:total_messages", tostring(current + 1))
end)

-- 3. A command to view the stats!
navi.create_slash("stats", "View bot and server statistics", {}, 
---@param ctx NaviSlashCtx
function(ctx)
    local joins = navi.db.get("stats:total_joins") or "0"
    local tickets = navi.db.get("stats:total_tickets") or "0"
    local messages = navi.db.get("stats:total_messages") or "0" -- NEW!

    local output = "📊 **Server Stats**\n"
    output = output .. "👥 Total Joins: `" .. joins .. "`\n"
    output = output .. "🎫 Tickets Opened: `" .. tickets .. "`\n"
    output = output .. "💬 Messages Sent: `" .. messages .. "`"

    ctx.reply(output, false)
end)