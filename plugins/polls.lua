navi.register_config("polls", {
    { key = "max_duration", name = "Max Duration (minutes)", description = "Cap on poll duration", type = "number", default = 1440 }
})

-- Register vote buttons for a given poll (called at creation and on reload for active polls)
local function register_vote_handler(poll_id)
    local data_json = navi.db.get("polls:data:" .. poll_id)
    if not data_json then return end
    local poll = navi.json.decode(data_json)
    if not poll then return end

    for i = 1, #poll.options do
        local opt_label = poll.options[i]
        local cid = "poll_" .. poll_id .. "_" .. i
        navi.register_component(cid, function(ctx)
            -- Re-read poll state from DB (handles close between reload and vote)
            local current_json = navi.db.get("polls:data:" .. poll_id)
            local current = current_json and navi.json.decode(current_json)
            if not current or current.closed or os.time() >= current.expires_at then
                ctx.reply("❌ This poll is already closed.", true)
                return
            end
            local vote_key = "polls:voted:" .. poll_id .. ":" .. ctx.user_id
            if navi.db.get(vote_key) then
                ctx.reply("You already voted!", true)
                return
            end
            navi.db.set(vote_key, tostring(i))
            local count = (tonumber(navi.db.get("polls:count:" .. poll_id .. ":" .. i)) or 0) + 1
            navi.db.set("polls:count:" .. poll_id .. ":" .. i, tostring(count))
            ctx.reply("✅ Voted for **" .. opt_label .. "**!", true)
        end)
    end
end

-- Restore handlers for polls that survived a reload
local active_str = navi.db.get("polls:active") or ""
if active_str ~= "" then
    for pid in active_str:gmatch("[^,]+") do
        register_vote_handler(pid)
    end
end

local function close_poll(poll_id)
    local poll = navi.json.decode(navi.db.get("polls:data:" .. poll_id) or "")
    if not poll then return end

    local best, winner = 0, "No votes"
    local lines = { "📊 **Poll Closed: " .. poll.title .. "**" }
    for i, opt in ipairs(poll.options) do
        local count = tonumber(navi.db.get("polls:count:" .. poll_id .. ":" .. i)) or 0
        table.insert(lines, string.format("• %s: **%d** vote(s)", opt, count))
        if count > best then best, winner = count, opt end
    end
    table.insert(lines, "\n🏆 Winner: **" .. winner .. "**")
    navi.say(poll.channel_id, table.concat(lines, "\n"))

    -- Mark closed and remove from active list
    poll.closed = true
    navi.db.set("polls:data:" .. poll_id, navi.json.encode(poll))
    local ids = {}
    for pid in (navi.db.get("polls:active") or ""):gmatch("[^,]+") do
        if pid ~= poll_id then table.insert(ids, pid) end
    end
    navi.db.set("polls:active", table.concat(ids, ","))
end

navi.set_interval(function()
    local now = os.time()
    for pid in (navi.db.get("polls:active") or ""):gmatch("[^,]+") do
        local poll_json = navi.db.get("polls:data:" .. pid)
        local poll = poll_json and navi.json.decode(poll_json)
        if poll and not poll.closed and now >= poll.expires_at then
            close_poll(pid)
        end
    end
end, 60, "s")

navi.create_slash("poll", "Create a timed poll with button voting", {}, function(ctx)
    ctx.modal("poll_create", "Create a Poll", {
        { id = "title",    label = "Poll Question",        style = "short",     placeholder = "What should we do?",  required = true  },
        { id = "opt1",     label = "Option 1",             style = "short",     placeholder = "Option A",             required = true  },
        { id = "opt2",     label = "Option 2",             style = "short",     placeholder = "Option B",             required = true  },
        { id = "opt3",     label = "Option 3 (optional)",  style = "short",     placeholder = "Leave blank to skip",  required = false },
        { id = "duration", label = "Duration (minutes)",   style = "short",     placeholder = "60",                   required = true  }
    })
end)

navi.register_modal("poll_create", function(ctx)
    local title    = ctx.values.title
    local opt1     = ctx.values.opt1
    local opt2     = ctx.values.opt2
    local opt3     = ctx.values.opt3
    local duration = tonumber(ctx.values.duration)
    local max_dur  = tonumber(navi.db.get("config:polls:max_duration")) or 1440

    if not duration or duration <= 0 then
        ctx.reply("❌ Invalid duration.", true) return
    end
    duration = math.min(duration, max_dur)

    local options = { opt1, opt2 }
    if opt3 and opt3 ~= "" then table.insert(options, opt3) end

    local poll_id = tostring((tonumber(navi.db.get("polls:next_id")) or 0) + 1)
    navi.db.set("polls:next_id", poll_id)

    local poll_data = {
        title      = title,
        options    = options,
        channel_id = ctx.channel_id,
        guild_id   = ctx.guild_id,
        expires_at = os.time() + duration * 60,
        closed     = false
    }
    navi.db.set("polls:data:" .. poll_id, navi.json.encode(poll_data))

    local active = navi.db.get("polls:active") or ""
    navi.db.set("polls:active", active == "" and poll_id or active .. "," .. poll_id)

    local components = {}
    for i, opt in ipairs(options) do
        table.insert(components, { type = "button", label = opt, id = "poll_" .. poll_id .. "_" .. i, style = "primary" })
    end

    navi.send_message(ctx.channel_id, {
        description = "📊 **" .. title .. "**\nPoll closes in **" .. duration .. " minute(s)**. Vote below!",
        color = 0x5865F2,
        components = components
    })

    register_vote_handler(poll_id)
    ctx.reply("✅ Poll created!", true)
end)
