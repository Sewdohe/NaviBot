navi.register_config("leveling", {
    { key = "xp_per_message",  name = "XP Per Message",          description = "XP awarded per qualifying message",                       type = "number",  default = 15 },
    { key = "cooldown_seconds",name = "Cooldown (seconds)",       description = "Seconds between XP-eligible messages per user",            type = "number",  default = 60 },
    { key = "xp_per_level",    name = "XP Per Level",             description = "XP needed per level (flat, e.g. 100 XP = 1 level)",       type = "number",  default = 100 },
    { key = "levelup_channel", name = "Level-Up Channel",         description = "Channel for announcements (blank = announce in-channel)",  type = "channel", default = "" },
    { key = "role_rewards",    name = "Role Rewards",             description = "Roles granted at specific levels",                        type = "list",
      item_schema = {
          { key = "level",   name = "Level", type = "number" },
          { key = "role_id", name = "Role",  type = "role"   }
      }
    }
})

local function get_xp(user_id)
    return tonumber(navi.db.get("leveling:xp:" .. user_id)) or 0
end

local function calc_level(xp)
    local xpl = tonumber(navi.db.get("config:leveling:xp_per_level")) or 100
    return math.floor(xp / xpl)
end

navi.register(function(msg)
    if msg.author_bot then return end

    local cooldown = tonumber(navi.db.get("config:leveling:cooldown_seconds")) or 60
    local reward   = tonumber(navi.db.get("config:leveling:xp_per_message"))   or 15

    local now  = os.time()
    local last = tonumber(navi.db.get("leveling:cd:" .. msg.author_id)) or 0
    if now - last < cooldown then return end
    navi.db.set("leveling:cd:" .. msg.author_id, tostring(now))

    local old_xp  = get_xp(msg.author_id)
    local new_xp  = old_xp + reward
    navi.db.set("leveling:xp:" .. msg.author_id, tostring(new_xp))

    local old_lvl = calc_level(old_xp)
    local new_lvl = calc_level(new_xp)

    if new_lvl > old_lvl then
        local ch = navi.db.get("config:leveling:levelup_channel")
        local target = (ch and ch ~= "") and ch or tostring(msg.channel_id)
        navi.say(target, "🎉 <@" .. msg.author_id .. "> leveled up to **Level " .. new_lvl .. "**!")

        if msg.guild_id then
            local rewards = navi.db.get_list("config:leveling:role_rewards")
            for _, r in ipairs(rewards) do
                if tonumber(r.level) == new_lvl and r.role_id and r.role_id ~= "" then
                    navi.add_role(msg.guild_id, tostring(msg.author_id), r.role_id)
                end
            end
        end
    end
end)

navi.create_slash("rank", "View your level and XP", {
    { name = "user", description = "User to check (default: yourself)", type = "user", required = false }
}, function(ctx)
    local uid = ctx.args.user or tostring(ctx.user_id)
    local xp  = get_xp(uid)
    local lvl = calc_level(xp)
    local xpl = tonumber(navi.db.get("config:leveling:xp_per_level")) or 100
    ctx.reply(string.format("🏆 **Level %d** | %d XP | %d / %d to next level", lvl, xp, xp % xpl, xpl))
end)

navi.create_slash("xp_leaderboard", "Top XP earners", {}, function(ctx)
    local rows = navi.db.query("SELECT key, value FROM kv_store WHERE key LIKE 'leveling:xp:%' ORDER BY CAST(value AS INTEGER) DESC LIMIT 10")
    if #rows == 0 then ctx.reply("No XP data yet.") return end
    local lines = { "🏆 **XP Leaderboard**" }
    for i, row in ipairs(rows) do
        local uid = row.key:match("leveling:xp:(.+)")
        local lvl = calc_level(tonumber(row.value) or 0)
        table.insert(lines, string.format("%d. <@%s> — Level %d (%s XP)", i, uid, lvl, row.value))
    end
    ctx.reply(table.concat(lines, "\n"))
end)
