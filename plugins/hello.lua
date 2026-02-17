print("--- Plugin System Online ---")

-- This function is called by Rust automatically!
navi.register(function(msg)
    -- Check the content
    if msg.content == "ping" then
        print("I heard a ping from " .. msg.author)
        
        -- Use our "mouth" to reply
        -- We use the channel_id that Rust gave us in the msg object
        navi.say(msg.channel_id, "Pong! (from automatic plugin)")
    end

    if msg.content == "who am i?" then
        navi.say(msg.channel_id, "You are " .. msg.author)
    end
end)