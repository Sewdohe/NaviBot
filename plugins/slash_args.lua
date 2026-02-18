navi.create_slash("gamble", "Bet some coins", {
    { name = "amount", description = "How much?", type = "integer", required = true },
    { name = "all_in", description = "Risk it all?", type = "boolean", required = false }
}, function(ctx)
    
    -- "amount" is now a real number!
    local amount = ctx.args.amount 
    
    -- "all_in" is now a real boolean!
    if ctx.args.all_in then
        amount = 1000 -- logic
    end

    -- Math works instantly
    local win = amount * 2 
    ctx.reply("You won " .. win .. " coins!")
end)