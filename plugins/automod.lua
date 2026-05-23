navi.register_config("automod", {
    { key = "log_channel",     name = "Log Channel",      description = "Channel for violation logs (leave empty to disable)",         type = "channel", default = "" },
    { key = "warn_in_channel", name = "Warn In Channel",  description = "Post a visible warning in chat on violations",                type = "boolean", default = "true" },

    { key = "banned_words",    name = "Banned Words",     description = "Words/phrases that trigger automod",                         type = "list",
      item_schema = {
          { key = "word",   name = "Word or Phrase", type = "string" },
          { key = "action", name = "Action",         type = "enum", options = {"delete","warn","timeout","kick","ban"} },
      }
    },

    { key = "max_mentions",   name = "Max Mentions",     description = "Max @mentions per message before action (0 = off)",           type = "number", default = 5 },
    { key = "mention_action", name = "Mention Action",   description = "Action taken on mention spam",                                type = "enum",   options = {"delete","warn","timeout"}, default = "warn" },

    { key = "spam_limit",     name = "Spam Limit",       description = "Max messages per window before action (0 = off)",             type = "number", default = 5 },
    { key = "spam_window",    name = "Spam Window (s)",  description = "Rolling time window in seconds for spam detection",           type = "number", default = 10 },
    { key = "spam_action",    name = "Spam Action",      description = "Action taken on message spam",                                type = "enum",   options = {"warn","timeout","kick","ban"}, default = "timeout" },

    { key = "link_filter",    name = "Link Filter",      description = "Delete messages containing URLs",                             type = "boolean", default = "false" },
    { key = "link_action",    name = "Link Action",      description = "Action taken on link violations",                             type = "enum",    options = {"delete","warn","timeout"}, default = "delete" },

    { key = "bomb_threshold", name = "Bomb Threshold",   description = "Same message in N+ channels within 30s triggers action (0 = off)", type = "number", default = 3 },
    { key = "bomb_action",    name = "Bomb Action",      description = "Action taken on channel bombing",                             type = "enum",   options = {"timeout","kick","ban"}, default = "timeout" },
})

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

local function cfg(key, fallback)
    local v = navi.db.get("config:automod:" .. key)
    if v == nil or v == "" then return fallback end
    return v
end

local function do_action(action, guild_id, user_id, reason)
    if action == "timeout" then
        navi.timeout(guild_id, user_id, 300)
    elseif action == "kick" then
        navi.kick(guild_id, user_id, reason)
    elseif action == "ban" then
        navi.ban(guild_id, user_id, 1, reason)
    end
end

local function log_violation(log_ch, vtype, author, author_id, channel_id, detail)
    if not log_ch or log_ch == "" then return end
    navi.send_message(log_ch, {
        title       = "⚠️ AutoMod — " .. vtype,
        color       = 0xFF6B6B,
        description = detail,
        fields      = {
            { name = "User",    value = "<@" .. author_id .. "> (" .. author .. ")", inline = true  },
            { name = "Channel", value = "<#" .. channel_id .. ">",                  inline = true  },
        },
    })
end

local function check_spam(user_id, limit, window)
    local key  = "spam_ts:" .. user_id
    local now  = os.time()
    local raw  = navi.db.get(key)
    local timestamps = raw and navi.json.decode(raw) or {}

    local fresh = {}
    for _, ts in ipairs(timestamps) do
        if now - ts < window then
            table.insert(fresh, ts)
        end
    end
    table.insert(fresh, now)
    navi.db.set(key, navi.json.encode(fresh))

    return #fresh >= limit
end

local function check_bombing(user_id, channel_id, content, threshold)
    local fp  = string.lower(string.sub(content, 1, 80))
    local key = "bomb_chans:" .. user_id .. ":" .. fp
    local now = os.time()
    local raw = navi.db.get(key)
    local entries = raw and navi.json.decode(raw) or {}

    local seen     = {}
    local fresh    = {}
    for _, e in ipairs(entries) do
        if now - e.ts < 30 and not seen[e.ch] then
            seen[e.ch] = true
            table.insert(fresh, e)
        end
    end

    if not seen[channel_id] then
        table.insert(fresh, { ch = channel_id, ts = now })
    end

    navi.db.set(key, navi.json.encode(fresh))
    return #fresh >= threshold
end

-- ---------------------------------------------------------------------------
-- Message listener
-- ---------------------------------------------------------------------------

navi.register(function(msg)
    if msg.author_bot then return end
    if not msg.guild_id  then return end

    local guild_id   = msg.guild_id
    local user_id    = tostring(msg.author_id)
    local channel_id = tostring(msg.channel_id)
    local message_id = tostring(msg.message_id)
    local content    = msg.content
    local author     = msg.author

    local log_ch     = cfg("log_channel", "")
    local warn_in_ch = cfg("warn_in_channel", "true") ~= "false"

    local function punish(vtype, action, warn_text, detail)
        navi.delete_message(channel_id, message_id)
        if warn_in_ch then
            navi.say(channel_id, "⚠️ <@" .. user_id .. ">: " .. warn_text)
        end
        log_violation(log_ch, vtype, author, user_id, channel_id, detail)
        do_action(action, guild_id, user_id, vtype)
    end

    -- 1. Banned words
    local banned = navi.db.get_list("banned_words")
    for _, entry in ipairs(banned) do
        local word = entry.word
        if word and word ~= "" then
            if string.find(string.lower(content), string.lower(word), 1, true) then
                local action = entry.action or "delete"
                punish("Banned Word", action,
                    "Your message was removed — prohibited word: `" .. word .. "`",
                    "Matched word: `" .. word .. "`")
                return
            end
        end
    end

    -- 2. Mention spam
    local max_mentions = tonumber(cfg("max_mentions", "5")) or 5
    if max_mentions > 0 and #msg.mentions >= max_mentions then
        local action = cfg("mention_action", "warn")
        punish("Mention Spam", action,
            "Too many mentions in one message.",
            "Mentions: " .. #msg.mentions .. " (limit: " .. max_mentions .. ")")
        return
    end

    -- 3. Link filter
    if cfg("link_filter", "false") == "true" and content:match("https?://") then
        local action = cfg("link_action", "delete")
        punish("Link Filter", action,
            "Links are not permitted here.",
            "URL detected in message")
        return
    end

    -- 4. Message spam
    local spam_limit  = tonumber(cfg("spam_limit",  "5"))  or 5
    local spam_window = tonumber(cfg("spam_window", "10")) or 10
    if spam_limit > 0 and check_spam(user_id, spam_limit, spam_window) then
        local action = cfg("spam_action", "timeout")
        punish("Message Spam", action,
            "You're sending messages too quickly.",
            "Exceeded " .. spam_limit .. " messages in " .. spam_window .. "s")
        return
    end

    -- 5. Channel bombing
    local bomb_threshold = tonumber(cfg("bomb_threshold", "3")) or 3
    if bomb_threshold > 0 and content ~= "" and check_bombing(user_id, channel_id, content, bomb_threshold) then
        local action = cfg("bomb_action", "timeout")
        punish("Channel Bombing", action,
            "Cross-channel spam detected.",
            "Same message posted in " .. bomb_threshold .. "+ channels within 30s")
        return
    end
end)

-- ---------------------------------------------------------------------------
-- /automod status
-- ---------------------------------------------------------------------------

navi.create_slash("automod", "AutoMod admin controls", {
    { name = "action", description = "Action to perform", type = "string", required = true,
      options = { { name = "status", value = "status" } }, autocomplete = false }
}, function(ctx)
    if not navi.require_perm(ctx, "admin") then return end

    local action = ctx.args.action
    if action ~= "status" then
        ctx.reply("Unknown action.", true)
        return
    end

    local function yn(key, fallback)
        local v = cfg(key, fallback)
        if v == "true" then return "✅ enabled" end
        return "❌ disabled"
    end

    local banned_count = #navi.db.get_list("banned_words")

    ctx.reply_embed({
        title = "🛡️ AutoMod Status",
        color = 0x5865F2,
        fields = {
            { name = "Log Channel",     value = (cfg("log_channel","") ~= "" and "<#"..cfg("log_channel","")..">" or "*(not set)*"),  inline = true  },
            { name = "Warn In Channel", value = yn("warn_in_channel","true"),                                                          inline = true  },
            { name = "Banned Words",    value = banned_count .. " word(s) configured",                                                inline = false },
            { name = "Mention Spam",    value = "Max " .. cfg("max_mentions","5") .. " mentions → " .. cfg("mention_action","warn"),   inline = true  },
            { name = "Link Filter",     value = yn("link_filter","false") .. " → " .. cfg("link_action","delete"),                    inline = true  },
            { name = "Message Spam",    value = cfg("spam_limit","5") .. " msg/" .. cfg("spam_window","10") .. "s → " .. cfg("spam_action","timeout"), inline = false },
            { name = "Channel Bombing", value = cfg("bomb_threshold","3") .. "+ channels/30s → " .. cfg("bomb_action","timeout"),     inline = false },
        },
    }, true)
end)
