print("--- Loading Command Dispatcher ---")

-- Config
local PREFIX = "!"

navi.register(function(msg)
    if msg.author.bot then return end

    navi.emit("message_sent", msg.author.id)

    -- 1. Check if it starts with the prefix
    if msg.content:sub(1, #PREFIX) ~= PREFIX then
        return -- Ignore normal chat
    end

    -- 2. Split the string into parts (e.g., "!pay @user 50")
    -- cmd = "pay", args = ["@user", "50"]
    local parts = {}
    for part in msg.content:gmatch("%S+") do
        table.insert(parts, part)
    end

    if #parts == 0 then return end

    -- 3. Extract Command Name (remove prefix)
    -- parts[1] is "!pay", so we substring it to get "pay"
    local command_name = parts[1]:sub(#PREFIX + 1)
    
    -- 4. Look up in Registry
    local command_func = navi.commands[command_name]

    if command_func then
        -- 5. Prepare Arguments (remove the command name from the list)
        table.remove(parts, 1)
        local args = parts 

        -- 6. EXECUTE (safely)
        local success, err = pcall(command_func, msg, args)
        if not success then
            navi.say(msg.channel_id, "❌ Command Error: " .. tostring(err))
        end
    end
end)