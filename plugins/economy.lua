print("--- Loading Economy Plugin ---")

-- 1. Register Configuration for the TUI Dashboard
navi.register_config("economy", {
  { key = "currency_name",  name = "Currency Name",  description = "What your money is called.", type = "string", default = "Credits" },
  { key = "message_reward", name = "Message Reward", description = "Amount earned per message.", type = "number", default = "5" }
})

-- Helper functions to keep database calls clean
local function get_balance(user_id)
  -- We create a custom namespace just for user balances
  local bal = navi.db.get("economy:balance:" .. user_id)
  return tonumber(bal) or 0
end

-- 2. Passive Income via the Event Bus!
navi.on("message", function(msg)
  -- Using YOUR actual engine field this time!
  local user_id = tostring(msg.author_id)

  -- Safety check so we don't accidentally fund the void again
  if not user_id or user_id == "nil" then
    return
  end

  local reward = tonumber(navi.db.get("config:economy:message_reward")) or 5
  add_balance(user_id, reward)
end)

-- 3. Check Balance Command
navi.create_slash("balance", "Check your bank account", {},
  ---@param ctx NaviSlashCtx
  function(ctx)
    local currency = navi.db.get("config:economy:currency_name") or "Credits"
    local bal = get_balance(ctx.user_id)

    navi.send_message(ctx.channel_id, {
      title = "🏦 Bank of Navi",
      description = "<@" .. ctx.user_id .. ">'s balance: **" .. bal .. " " .. currency .. "**",
      color = 0xF1C40F -- Gold color
    })

    ctx.reply("Balance retrieved!", true)
  end)

-- 4. Pay Another User Command
navi.create_slash("pay", "Send money to another user", {
    { name = "user",   description = "Who to pay", type = "user",    required = true },
    { name = "amount", description = "How much",   type = "integer", required = true }
  },
  ---@param ctx NaviSlashCtx
  function(ctx)
    local target_id = ctx.options.user
    local amount = tonumber(ctx.options.amount)
    local currency = navi.db.get("config:economy:currency_name") or "Credits"

    -- Prevent infinite money exploits
    if amount <= 0 then
      ctx.reply("Nice try! Amount must be greater than 0.", true)
      return
    end

    local sender_bal = get_balance(ctx.user_id)
    if sender_bal < amount then
      ctx.reply("❌ You don't have enough " .. currency .. " for that.", true)
      return
    end

    -- Execute the transfer
    add_balance(ctx.user_id, -amount)
    add_balance(target_id, amount)

    navi.send_message(ctx.channel_id, {
      description = "💸 <@" .. ctx.user_id .. "> sent **" .. amount .. " " .. currency .. "** to <@" .. target_id .. ">.",
      color = 0x2ECC71 -- Emerald Green
    })

    ctx.reply("Transfer complete.", true)
  end)


local function add_balance(user_id, amount)
  local current = get_balance(user_id)
  local new_balance = current + amount
  navi.db.set("economy:balance:" .. user_id, tostring(new_balance))

  -- Broadcast the change so other plugins can trigger level-ups or alerts!
  navi.emit("economy:balance_changed", {
    user_id = user_id,
    old_balance = current,
    new_balance = new_balance,
    amount_changed = amount
  })

  return new_balance
end

-- 🔌 PUBLIC API: Allow other plugins to add money
navi.on("economy:add", function(data)
  if data.user_id and data.amount then
    add_balance(data.user_id, tonumber(data.amount) or 0)
  end
end)

-- 🔌 PUBLIC API: Allow other plugins to take money
navi.on("economy:remove", function(data)
  if data.user_id and data.amount then
    add_balance(data.user_id, -(tonumber(data.amount) or 0))
  end
end)

navi.create_slash("leaderboard", "See the richest users in the server", {},
  ---@param ctx NaviSlashCtx
  function(ctx)
    local currency = navi.db.get("config:economy:currency_name") or "Credits"

    -- Ask kv_store for all economy balances, sorted natively by SQLite!
    local sql =
    "SELECT key, value FROM kv_store WHERE key LIKE 'economy:balance:%' ORDER BY CAST(value AS INTEGER) DESC LIMIT 10"

    local top_users = navi.db.query(sql)
    local desc = ""

    for i, row in ipairs(top_users) do
      local user_id = string.sub(row.key, 17)

      -- Filter out the corrupted "nil" row from our earlier bug
      if user_id ~= "nil" then
        local medal = "🏅"
        if i == 1 then
          medal = "🥇"
        elseif i == 2 then
          medal = "🥈"
        elseif i == 3 then
          medal = "🥉"
        end

        desc = desc .. medal .. " **" .. i .. ".** <@" .. user_id .. "> - " .. row.value .. " " .. currency .. "\n"
      end
    end

    if desc == "" then desc = "The economy is completely empty!" end

    navi.send_message(ctx.channel_id, {
      title = "🏆 Rich List",
      description = desc,
      color = 0xF1C40F
    })

    ctx.reply("Leaderboard generated!", true)
  end)
