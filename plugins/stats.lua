print("--- Loading Bot Statistics ---")

-- 1. Listen for new members joining
navi.on("new_member_joined", function(username)
    local current = tonumber(navi.db.get("stats:total_joins")) or 0
    navi.db.set("stats:total_joins", tostring(current + 1))
    print("📈 Stats: Logged new join! Total: " .. tostring(current + 1))
end)

-- 2. Listen for tickets being opened
navi.on("ticket_created", function(user_id)
    local current = tonumber(navi.db.get("stats:total_tickets")) or 0
    navi.db.set("stats:total_tickets", tostring(current + 1))
    print("📈 Stats: Logged new ticket! Total: " .. tostring(current + 1))
end)

-- 3. A command to view the stats!
navi.create_slash("stats", "View bot and server statistics", {}, function(ctx)
    local joins = navi.db.get("stats:total_joins") or "0"
    local tickets = navi.db.get("stats:total_tickets") or "0"

    ctx.reply("📊 **Server Stats**\n👥 Total Joins: `" .. joins .. "`\n🎫 Tickets Opened: `" .. tickets .. "`", false)
end)