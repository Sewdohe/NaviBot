navi.register_config("warns", {
    { key = "log_channel",    name = "Log Channel",     description = "Channel to log warnings (leave empty to disable)",           type = "channel", default = "" },
    { key = "auto_escalate",  name = "Auto-Escalate",   description = "Automatically punish users who accumulate warnings",         type = "boolean", default = "true" },
    { key = "warn_3_action",  name = "3-Warn Action",   description = "Action when a user reaches 3 warnings",                     type = "enum",    options = {"timeout","kick","ban"}, default = "timeout" },
    { key = "warn_5_action",  name = "5-Warn Action",   description = "Action when a user reaches 5 warnings",                     type = "enum",    options = {"kick","ban"},           default = "kick" },
})

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

local function cfg(key, fallback)
    local v = navi.db.get("config:warns:" .. key)
    if v == nil or v == "" then return fallback end
    return v
end

local function get_warns(user_id)
    local raw = navi.db.get("warns:" .. user_id)
    return raw and navi.json.decode(raw) or {}
end

local function save_warns(user_id, list)
    navi.db.set("warns:" .. user_id, navi.json.encode(list))
end

local function log_warn(log_ch, action, target_id, target_name, mod_id, mod_name, reason, total)
    if not log_ch or log_ch == "" then return end
    local color = action == "cleared" and 0x57F287 or 0xFFA500

    navi.send_message(log_ch, {
        title  = action == "cleared" and "🧹 Warnings Cleared" or "⚠️ Warning Issued",
        color  = color,
        fields = {
            { name = "User",        value = "<@" .. target_id .. "> (" .. target_name .. ")", inline = true  },
            { name = "Moderator",   value = "<@" .. mod_id .. "> (" .. mod_name .. ")",       inline = true  },
            { name = "Reason",      value = reason,                                            inline = false },
            { name = "Total Warns", value = tostring(total),                                   inline = true  },
        },
    })
end

-- ---------------------------------------------------------------------------
-- /warn <user> <reason>
-- ---------------------------------------------------------------------------

navi.create_slash("warn", "Issue a warning to a user", {
    { name = "user",   description = "User to warn",    type = "user",   required = true  },
    { name = "reason", description = "Reason for warn", type = "string", required = false },
}, function(ctx)
    if not navi.require_perm(ctx, "moderator") then return end

    local target_id = tostring(ctx.args.user)
    local reason    = ctx.args.reason or "No reason provided"
    local mod_id    = tostring(ctx.user_id)
    local guild_id  = ctx.guild_id

    local list = get_warns(target_id)
    table.insert(list, {
        reason    = reason,
        mod_id    = mod_id,
        mod_name  = ctx.username,
        timestamp = os.time(),
    })
    save_warns(target_id, list)

    local total = #list

    ctx.reply_embed({
        title  = "⚠️ Warning Issued",
        color  = 0xFFA500,
        fields = {
            { name = "User",   value = "<@" .. target_id .. ">", inline = true  },
            { name = "Warn #", value = tostring(total),           inline = true  },
            { name = "Reason", value = reason,                    inline = false },
        },
    }, false)

    log_warn(cfg("log_channel",""), "issued", target_id, target_id, mod_id, ctx.username, reason, total)

    -- Auto-escalation
    if cfg("auto_escalate","true") == "true" and guild_id then
        if total == 3 then
            local action = cfg("warn_3_action","timeout")
            if action == "timeout" then
                navi.timeout(guild_id, target_id, 3600)
                navi.say(ctx.channel_id, "🔇 <@" .. target_id .. "> has been timed out for 1 hour (3 warnings).")
            elseif action == "kick" then
                navi.kick(guild_id, target_id, "3 warnings reached")
                navi.say(ctx.channel_id, "👢 <@" .. target_id .. "> has been kicked (3 warnings).")
            elseif action == "ban" then
                navi.ban(guild_id, target_id, 0, "3 warnings reached")
                navi.say(ctx.channel_id, "🔨 <@" .. target_id .. "> has been banned (3 warnings).")
            end
        elseif total == 5 then
            local action = cfg("warn_5_action","kick")
            if action == "kick" then
                navi.kick(guild_id, target_id, "5 warnings reached")
                navi.say(ctx.channel_id, "👢 <@" .. target_id .. "> has been kicked (5 warnings).")
            elseif action == "ban" then
                navi.ban(guild_id, target_id, 0, "5 warnings reached")
                navi.say(ctx.channel_id, "🔨 <@" .. target_id .. "> has been banned (5 warnings).")
            end
        end
    end
end)

-- ---------------------------------------------------------------------------
-- /warnings <user>
-- ---------------------------------------------------------------------------

navi.create_slash("warnings", "View warnings for a user", {
    { name = "user", description = "User to check", type = "user", required = true },
}, function(ctx)
    if not navi.require_perm(ctx, "moderator") then return end

    local target_id = tostring(ctx.args.user)
    local list      = get_warns(target_id)

    if #list == 0 then
        ctx.reply_embed({
            title       = "📋 Warnings for <@" .. target_id .. ">",
            description = "This user has no warnings.",
            color       = 0x57F287,
        }, true)
        return
    end

    local lines = {}
    for i, w in ipairs(list) do
        local date = os.date("%Y-%m-%d", w.timestamp)
        table.insert(lines, "**#" .. i .. "** — " .. w.reason .. "\n*by <@" .. w.mod_id .. "> on " .. date .. "*")
    end

    ctx.reply_embed({
        title       = "📋 Warnings for <@" .. target_id .. "> (" .. #list .. " total)",
        description = table.concat(lines, "\n\n"),
        color       = 0xFFA500,
    }, true)
end)

-- ---------------------------------------------------------------------------
-- /clearwarns <user>
-- ---------------------------------------------------------------------------

navi.create_slash("clearwarns", "Clear all warnings for a user", {
    { name = "user",   description = "User to clear", type = "user",   required = true  },
    { name = "reason", description = "Reason",        type = "string", required = false },
}, function(ctx)
    if not navi.require_perm(ctx, "moderator") then return end

    local target_id = tostring(ctx.args.user)
    local reason    = ctx.args.reason or "No reason provided"
    local prev      = #get_warns(target_id)

    save_warns(target_id, {})

    ctx.reply_embed({
        title  = "🧹 Warnings Cleared",
        color  = 0x57F287,
        fields = {
            { name = "User",              value = "<@" .. target_id .. ">", inline = true  },
            { name = "Warnings Removed",  value = tostring(prev),           inline = true  },
            { name = "Reason",            value = reason,                   inline = false },
        },
    }, false)

    log_warn(cfg("log_channel",""), "cleared", target_id, target_id, tostring(ctx.user_id), ctx.username, reason, 0)
end)
