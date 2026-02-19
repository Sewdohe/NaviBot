print("--- Loading Gamble Module ---")

-- 1. Simple Coin Flip (No args)
navi.create_slash("coinflip", "Flip a coin!", {}, function(ctx)
    local result = math.random(1, 2)
    if result == 1 then
        ctx.reply("🪙 **Heads!**")
    else
        ctx.reply("🪙 **Tails!**")
    end
end)

-- 2. Dice Roll (With Argument)
navi.create_slash("roll", "Roll a die with N sides", {
    { name = "sides", description = "How many sides? (default 6)", type = "integer", required = false }
}, function(ctx)
    -- If they didn't provide a number, default to 6
    local sides = ctx.args.sides or 6
    
    local roll = math.random(1, sides)
    ctx.reply("🎲 You rolled a **" .. roll .. "** (1-" .. sides .. ")")
end)

-- 3. High/Low (Testing Booleans)
navi.create_slash("bet", "Bet on High or Low", {
    { name = "amount", description = "Amount to bet", type = "integer", required = true },
    { name = "is_high", description = "Bet on High (True) or Low (False)?", type = "boolean", required = true }
}, function(ctx)
    local amount = ctx.args.amount
    local betting_high = ctx.args.is_high
    
    -- Simulate a roll (1-100)
    local roll = math.random(1, 100)
    local is_high_roll = roll > 50
    
    local msg = "🎲 Rolled: **" .. roll .. "** "
    
    if is_high_roll == betting_high then
        msg = msg .. "\n✅ **WINNER!** You won " .. (amount * 2) .. " coins!"
    else
        msg = msg .. "\n❌ **LOST!** Better luck next time."
    end
    
    ctx.reply(msg)
end)