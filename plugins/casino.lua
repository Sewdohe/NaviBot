navi.log.info("Loading Casino Plugin")
navi.depends_on("economy")  -- Ensure economy plugin is loaded first

-- Seed the random number generator on load (Lua 5.4 style)
math.randomseed()

-- 1. Config
navi.register_config("casino", {
  { key = "min_bet", name = "Minimum Bet", description = "Smallest bet allowed", type = "number", default = "10" },
  { key = "max_bet", name = "Maximum Bet", description = "Largest bet allowed",  type = "number", default = "10000" },
})

-- ===========================
-- HELPERS
-- ===========================

local function get_currency()
  return navi.db.get("config:economy:currency_name") or "Credits"
end

-- Read balance directly from the economy plugin's DB namespace.
-- Keys with ":" bypass auto-namespacing and are used verbatim.
local function get_balance(user_id)
  return tonumber(navi.db.get("economy:balance:" .. user_id)) or 0
end

---@param user_id string
---@param bet integer
---@return string|nil  -- error message, or nil if valid
local function validate_bet(user_id, bet)
  local currency = get_currency()
  local min_bet  = tonumber(navi.db.get("config:casino:min_bet"))  or 10
  local max_bet  = tonumber(navi.db.get("config:casino:max_bet"))  or 10000

  if bet <= 0 then
    return "❌ Bet must be greater than **0**."
  end
  if bet < min_bet then
    return "❌ Minimum bet is **" .. min_bet .. " " .. currency .. "**."
  end
  if bet > max_bet then
    return "❌ Maximum bet is **" .. max_bet .. " " .. currency .. "**."
  end
  if get_balance(user_id) < bet then
    return "❌ You don't have enough " .. currency .. "! Balance: **" .. get_balance(user_id) .. "**."
  end
  return nil
end

-- Update casino-wide and per-user stats after each game.
-- net_change: positive = player won that much, negative = player lost that much
local function track_result(user_id, bet, net_change)
  -- Global totals
  local bets = tonumber(navi.db.get("casino:total_bets")) or 0
  navi.db.set("casino:total_bets", tostring(bets + 1))

  local wagered = tonumber(navi.db.get("casino:total_wagered")) or 0
  navi.db.set("casino:total_wagered", tostring(wagered + bet))

  if net_change > 0 then
    local paid_out = tonumber(navi.db.get("casino:total_paid_out")) or 0
    navi.db.set("casino:total_paid_out", tostring(paid_out + net_change))

    local biggest = tonumber(navi.db.get("casino:biggest_win")) or 0
    if net_change > biggest then
      navi.db.set("casino:biggest_win", tostring(net_change))
      navi.db.set("casino:biggest_win_user", tostring(user_id))
    end

    local w = tonumber(navi.db.get("casino:wins:" .. user_id)) or 0
    navi.db.set("casino:wins:" .. user_id, tostring(w + 1))
  else
    local l = tonumber(navi.db.get("casino:losses:" .. user_id)) or 0
    navi.db.set("casino:losses:" .. user_id, tostring(l + 1))
  end
end

-- Capitalise the first letter of a string
local function ucfirst(s)
  return string.upper(string.sub(s, 1, 1)) .. string.sub(s, 2)
end

-- ===========================
-- GAME 1: COIN FLIP
-- ===========================

navi.create_slash("coinflip", "Bet on a coin toss — 2x payout on a correct call", {
    { name = "bet",    description = "Amount to wager",  type = "integer", required = true },
    { name = "choice", description = "heads or tails",   type = "string",  required = true },
  },
  ---@param ctx NaviSlashCtx
  function(ctx)
    local bet      = tonumber(ctx.args.bet) or 0
    local choice   = string.lower(tostring(ctx.args.choice or ""))
    local currency = get_currency()

    if choice ~= "heads" and choice ~= "tails" then
      ctx.reply("❌ Choose **heads** or **tails**.")
      return
    end

    local err = validate_bet(ctx.user_id, bet)
    if err then ctx.reply(err); return end

    local result = math.random(0, 1) == 0 and "heads" or "tails"
    local won    = result == choice

    if won then
      navi.emit("economy:add", { user_id = ctx.user_id, amount = bet })
      track_result(ctx.user_id, bet, bet)
      navi.send_message(ctx.channel_id, {
        title       = "🪙 Coin Flip — " .. ucfirst(result) .. "!",
        description = "<@" .. ctx.user_id .. "> called **" .. choice .. "** — correct!\n"
                   .. "💰 +**" .. bet .. " " .. currency .. "**",
        color       = 0x2ECC71,
      })
    else
      navi.emit("economy:remove", { user_id = ctx.user_id, amount = bet })
      track_result(ctx.user_id, bet, -bet)
      navi.send_message(ctx.channel_id, {
        title       = "🪙 Coin Flip — " .. ucfirst(result) .. "!",
        description = "<@" .. ctx.user_id .. "> called **" .. choice .. "** — wrong!\n"
                   .. "💸 -**" .. bet .. " " .. currency .. "**",
        color       = 0xE74C3C,
      })
    end

    ctx.reply("Coin tossed!")
  end)

-- ===========================
-- GAME 2: SLOT MACHINE
-- ===========================

-- Weighted symbol pool (total weight = 16)
local SLOT_SYMBOLS  = { "🍒", "🍋", "🍊", "🍇", "⭐", "💎" }
local SLOT_WEIGHTS  = {   4,    3,    3,    3,    2,    1  }

local function spin_reel()
  local r = math.random(1, 16)
  local acc = 0
  for i, w in ipairs(SLOT_WEIGHTS) do
    acc = acc + w
    if r <= acc then return SLOT_SYMBOLS[i] end
  end
  return SLOT_SYMBOLS[1]
end

navi.create_slash("slots", "Spin the slot machine — match symbols for big payouts", {
    { name = "bet", description = "Amount to wager", type = "integer", required = true },
  },
  ---@param ctx NaviSlashCtx
  function(ctx)
    local bet      = tonumber(ctx.args.bet) or 0
    local currency = get_currency()

    local err = validate_bet(ctx.user_id, bet)
    if err then ctx.reply(err); return end

    local r1 = spin_reel()
    local r2 = spin_reel()
    local r3 = spin_reel()
    local display = "[ **" .. r1 .. "** | **" .. r2 .. "** | **" .. r3 .. "** ]"

    -- Determine multiplier and result label
    local multiplier   = 0
    local result_label = ""

    if r1 == r2 and r2 == r3 then
      if r1 == "💎" then
        multiplier   = 50
        result_label = "💎 **TRIPLE DIAMOND — JACKPOT!** 💎"
      elseif r1 == "⭐" then
        multiplier   = 20
        result_label = "⭐ **Triple Star!**"
      else
        multiplier   = 8
        result_label = "🎰 **Triple " .. r1 .. "!**"
      end
    elseif r1 == r2 or r2 == r3 or r1 == r3 then
      multiplier   = 2
      result_label = "👍 **Two of a kind!**"
    else
      result_label = "❌ No match."
    end

    local net_change = bet * multiplier - bet
    local description
    local color

    if net_change > 0 then
      navi.emit("economy:add", { user_id = ctx.user_id, amount = net_change })
      track_result(ctx.user_id, bet, net_change)
      description = display .. "\n\n" .. result_label
                 .. "\n💰 +**" .. net_change .. " " .. currency .. "** (**" .. multiplier .. "x**)"
      color = 0x2ECC71
    else
      navi.emit("economy:remove", { user_id = ctx.user_id, amount = bet })
      track_result(ctx.user_id, bet, -bet)
      description = display .. "\n\n" .. result_label
                 .. "\n💸 -**" .. bet .. " " .. currency .. "**"
      color = 0xE74C3C
    end

    navi.send_message(ctx.channel_id, {
      title       = "🎰 Slot Machine",
      description = description,
      color       = color,
    })

    ctx.reply("Reels spun!")
  end)

-- ===========================
-- GAME 3: DICE ROLL
-- ===========================
-- Guess the number 1-6. Correct = 5x total payout (net +4x).

local DIE_EMOJI = { "⚀", "⚁", "⚂", "⚃", "⚄", "⚅" }

navi.create_slash("dice", "Guess the die roll (1-6) for a 5x payout", {
    { name = "bet",   description = "Amount to wager",  type = "integer", required = true },
    { name = "guess", description = "Your guess (1-6)", type = "integer", required = true },
  },
  ---@param ctx NaviSlashCtx
  function(ctx)
    local bet      = tonumber(ctx.args.bet)   or 0
    local guess    = tonumber(ctx.args.guess) or 0
    local currency = get_currency()

    if guess < 1 or guess > 6 then
      ctx.reply("❌ Pick a number between **1** and **6**.")
      return
    end

    local err = validate_bet(ctx.user_id, bet)
    if err then ctx.reply(err); return end

    local roll = math.random(1, 6)
    local won  = roll == guess

    if won then
      local payout = bet * 4 -- net gain; total returned = bet * 5
      navi.emit("economy:add", { user_id = ctx.user_id, amount = payout })
      track_result(ctx.user_id, bet, payout)
      navi.send_message(ctx.channel_id, {
        title       = DIE_EMOJI[roll] .. " Dice Roll — Correct!",
        description = "<@" .. ctx.user_id .. "> guessed **" .. guess .. "** — rolled **" .. roll .. "**!\n"
                   .. "💰 +**" .. payout .. " " .. currency .. "** (5x payout)",
        color       = 0x2ECC71,
      })
    else
      navi.emit("economy:remove", { user_id = ctx.user_id, amount = bet })
      track_result(ctx.user_id, bet, -bet)
      navi.send_message(ctx.channel_id, {
        title       = DIE_EMOJI[roll] .. " Dice Roll — Nope!",
        description = "<@" .. ctx.user_id .. "> guessed **" .. guess .. "** — rolled **" .. roll .. "**.\n"
                   .. "💸 -**" .. bet .. " " .. currency .. "**",
        color       = 0xE74C3C,
      })
    end

    ctx.reply("Dice rolled!")
  end)

-- ===========================
-- GAME 4: ROULETTE
-- ===========================
-- 36 numbers + 2 green (house pockets).
-- Red / Black: ~47.4% chance, 2x payout.
-- Green: ~5.3% chance, 14x payout.

local ROULETTE_EMOJI = { red = "🔴", black = "⚫", green = "🟢" }
local ROULETTE_PAYOUTS = { red = 2, black = 2, green = 14 }

navi.create_slash("roulette", "Spin the roulette wheel — red, black, or green", {
    { name = "bet",    description = "Amount to wager",         type = "integer", required = true },
    { name = "choice", description = "red, black, or green",    type = "string",  required = true },
  },
  ---@param ctx NaviSlashCtx
  function(ctx)
    local bet      = tonumber(ctx.args.bet) or 0
    local choice   = string.lower(tostring(ctx.args.choice or ""))
    local currency = get_currency()

    if choice ~= "red" and choice ~= "black" and choice ~= "green" then
      ctx.reply("❌ Choose **red**, **black**, or **green**.")
      return
    end

    local err = validate_bet(ctx.user_id, bet)
    if err then ctx.reply(err); return end

    -- 18 red + 18 black + 2 green = 38 total
    local spin = math.random(1, 38)
    local result
    if spin <= 18 then
      result = "red"
    elseif spin <= 36 then
      result = "black"
    else
      result = "green"
    end

    local multiplier = ROULETTE_PAYOUTS[result]
    local emoji      = ROULETTE_EMOJI[result]
    local won        = result == choice
    local net_change = bet * (multiplier - 1)

    if won then
      navi.emit("economy:add", { user_id = ctx.user_id, amount = net_change })
      track_result(ctx.user_id, bet, net_change)
      navi.send_message(ctx.channel_id, {
        title       = "🎡 Roulette — " .. emoji .. " " .. ucfirst(result) .. "!",
        description = "<@" .. ctx.user_id .. "> bet on " .. ROULETTE_EMOJI[choice]
                   .. " **" .. choice .. "** and **won**!\n"
                   .. "💰 +**" .. net_change .. " " .. currency .. "** (**" .. multiplier .. "x**)",
        color       = (result == "red") and 0xE74C3C or (result == "green") and 0x2ECC71 or 0x99AAB5,
      })
    else
      navi.emit("economy:remove", { user_id = ctx.user_id, amount = bet })
      track_result(ctx.user_id, bet, -bet)
      navi.send_message(ctx.channel_id, {
        title       = "🎡 Roulette — " .. emoji .. " " .. ucfirst(result) .. "!",
        description = "<@" .. ctx.user_id .. "> bet on " .. ROULETTE_EMOJI[choice]
                   .. " **" .. choice .. "** and **lost**.\n"
                   .. "💸 -**" .. bet .. " " .. currency .. "**",
        color       = 0xE74C3C,
      })
    end

    ctx.reply("Wheel spun!")
  end)

-- ===========================
-- CASINO STATS COMMAND
-- ===========================

navi.create_slash("casino_stats", "View your personal casino record", {},
  ---@param ctx NaviSlashCtx
  function(ctx)
    local wins   = tonumber(navi.db.get("casino:wins:"   .. ctx.user_id)) or 0
    local losses = tonumber(navi.db.get("casino:losses:" .. ctx.user_id)) or 0
    local total  = wins + losses
    local rate   = total > 0 and math.floor((wins / total) * 100) or 0

    local biggest_win  = navi.db.get("casino:biggest_win")      or "0"
    local biggest_user = navi.db.get("casino:biggest_win_user") or "?"
    local currency     = get_currency()

    local desc = "🏆 Wins: **" .. wins .. "**\n"
              .. "💀 Losses: **" .. losses .. "**\n"
              .. "📊 Win Rate: **" .. rate .. "%**\n\n"
              .. "🌍 All-time biggest win: **" .. biggest_win .. " " .. currency
              .. "** by <@" .. biggest_user .. ">"

    navi.send_message(ctx.channel_id, {
      title       = "🎰 Casino Stats — " .. ctx.user_id,
      description = desc,
      color       = 0xF1C40F,
    })

    ctx.reply("Stats loaded!")
  end)
