print("--- Loading Counting Game Plugin ---")

-- 1. Register Configuration for the TUI Dashboard
navi.register_config("counting", {
  { key = "counting_channel",  name = "Counting Channel",    description = "Where the game happens",                type = "channel", default = "" },
  { key = "reward_amount",     name = "Reward per Count",    description = "Credits earned for a correct number",   type = "number",  default = "1" },
  { key = "penalty_amount",    name = "Penalty for Mistake", description = "Credits lost for ruining the count",    type = "number",  default = "5" },
  { key = "restart_on_fail",   name = "Restart on Fail",     description = "Reset count to 0 if someone messes up", type = "boolean", default = "true" },
  { key = "allow_talking",     name = "Allow Talking",       description = "Ignore non-number messages?",           type = "boolean", default = "false" },
  { key = "allow_consecutive", name = "Allow Consecutive",   description = "Can a user count twice in a row?",      type = "boolean", default = "false" }
})

-- Helpers to track the game state in the DB
local function get_current_count()
  return tonumber(navi.db.get("counting:current_number")) or 0
end

local function set_current_count(val)
  navi.db.set("counting:current_number", tostring(val))
end

local function get_last_counter()
  return navi.db.get("counting:last_counter") or ""
end

local function set_last_counter(uid)
  navi.db.set("counting:last_counter", tostring(uid))
end

-- 2. Game Logic Listener
navi.on("message", function(msg)
  local channel_id = tostring(msg.channel_id)
  local target_channel = navi.db.get("config:counting:counting_channel")

  -- Ignore if it's the wrong channel, channel isn't configured, or author is void/bot
  if target_channel == "" or channel_id ~= target_channel then return end
  if not msg.author_id or tostring(msg.author_id) == "nil" then return end

  local user_id = tostring(msg.author_id)

  -- Extract the number from the start of the message (e.g., "5" or "5 is my favorite")
  local number_match = msg.content:match("^%s*(%d+)")
  local allow_talking = navi.db.get("config:counting:allow_talking") == "true"

  -- If there's no number in the message
  if not number_match then
    if allow_talking then
      return              -- Silently ignore casual conversation
    else
      number_match = "-1" -- Force a failure below
    end
  end

  local attempt = tonumber(number_match)
  if not attempt then return end

  local current = get_current_count()
  local next_num = current + 1
  local last_user = get_last_counter()
  local allow_consecutive = navi.db.get("config:counting:allow_consecutive") == "true"

  local failed = false
  local fail_reason = ""

  -- Check the rules!
  if attempt ~= next_num then
    failed = true
    fail_reason = "Wrong number! The next number was **" .. next_num .. "**."
  elseif not allow_consecutive and user_id == last_user then
    failed = true
    fail_reason = "You can't count twice in a row!"
  end

  -- Handle the outcome
  if failed then
    -- Cast IDs to string explicitly to keep the Rust bridge happy
    navi.react(channel_id, tostring(msg.message_id), "❌")

    -- Charge them money!
    local penalty = tonumber(navi.db.get("config:counting:penalty_amount")) or 5
    if penalty > 0 then
      navi.emit("economy:remove", { user_id = user_id, amount = penalty })
    end

    local restart = navi.db.get("config:counting:restart_on_fail") == "true"
    local response = "🚨 <@" .. user_id .. "> ruined the count at **" .. current .. "**! " .. fail_reason

    if restart then
      set_current_count(0)
      set_last_counter("")
      response = response .. " The count has been reset to 0."
    end

    navi.send_message(channel_id, {
      title = "Count Ruined!",
      description = response,
      color = 0xE74C3C -- Red
    })
  else
    -- Success!
    navi.react(channel_id, tostring(msg.message_id), "✅")
    set_current_count(next_num)
    set_last_counter(user_id)

    -- Pay them!
    local reward = tonumber(navi.db.get("config:counting:reward_amount")) or 1
    if reward > 0 then
      navi.emit("economy:add", { user_id = user_id, amount = reward })
    end

    -- Throw a little party every 100 numbers
    if next_num % 100 == 0 then
      navi.send_message(channel_id, {
        description = "🎉 The count reached **" .. next_num .. "**!",
        color = 0x2ECC71 -- Green
      })
    end
  end
end)
