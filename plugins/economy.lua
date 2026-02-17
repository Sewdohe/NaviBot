print("--- Loading Economy Module ---")

-- Configuration
local COIN_NAME = "BitCubes"
local DAILY_AMOUNT = 100
local COOLDOWN = 86400 -- 24 hours in seconds

-- !bal
navi.create_command("bal", function(msg, args)
    local bal = tonumber(navi.db.get("bal_" .. msg.author_id)) or 0
    navi.say(msg.channel_id, "💰 You have **" .. bal .. "** BitCubes")
end)

-- !daily
navi.create_command("daily", function(msg, args)
    -- (Keep your existing daily logic here)
    -- ...
    navi.say(msg.channel_id, "✅ Daily claimed!")
end)

-- !pay @user 100
navi.create_command("pay", function(msg, args)
    -- We don't need string manipulation anymore!
    -- args[1] is the user, args[2] is the amount
    
    if #msg.mentions == 0 or not args[2] then
        navi.say(msg.channel_id, "Usage: !pay @user [amount]")
        return
    end

    local amount = tonumber(args[2])
    local target_id = msg.mentions[1].id
    
    -- (Keep your transaction logic here)
    -- ...
    navi.say(msg.channel_id, "💸 Sent " .. amount)
end)