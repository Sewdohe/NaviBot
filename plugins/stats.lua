print("--- Loading Statistics Module ---")

navi.register(function(msg)
    -- 1. Generate a unique key for this user
    -- Key format: "msg_count_USERID"
    local key = "msg_count_" .. msg.author_id
    
    -- 2. Get current count (default to 0 if nil)
    local count = tonumber(navi.db.get(key)) or 0
    
    -- 3. Increment
    count = count + 1
    
    -- 4. Save it back
    navi.db.set(key, tostring(count))
    
    -- Fun logic: Congratulate them on milestones
    if count == 10 then
        navi.say(msg.channel_id, "🎉 " .. msg.author .. " just sent their 10th message!")
    end
end)