-- interval_test.lua: smoke-test for navi.set_interval / navi.clear_interval
-- Remove or comment out this plugin once you're satisfied it works.

local count = 0

-- fires every 3 seconds
local tick_id = navi.set_interval(function()
    count = count + 1
    navi.log.info("tick #" .. count)
end, 1, "h")

-- fires every 5000 ms (raw ms default)
-- navi.set_interval(function()
--     navi.log.info("tick every 5000ms")
-- end, 5000)

-- slash command to stop the test timer
navi.create_slash("stoptimer", "Stop the interval test timer", {}, function(ctx)
    navi.clear_interval(tick_id)
    ctx.reply("Stopped interval #" .. tick_id .. " (was at tick #" .. count .. ")", true)
end)
