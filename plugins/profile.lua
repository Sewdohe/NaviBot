print("--- Loading Profile Module ---")

navi.register(function(msg)
    if msg.content:find("!profile") then
        
        -- 1. Determine Target (Self or Mention)
        local target_id = msg.author_id
        local target_name = msg.author
        local target_avatar = msg.author_avatar

        if #msg.mentions > 0 then
            target_id = msg.mentions[1].id
            target_name = msg.mentions[1].name
            target_avatar = msg.mentions[1].avatar
        end

        -- 2. Fetch Data from Database
        -- We can grab as many keys as we want!
        local xp = navi.db.get("msg_count_" .. target_id) or "0"
        local bal = navi.db.get("bal_" .. target_id) or "0"

        -- 3. Build the Embed
        navi.send_embed(msg.channel_id, {
            title = "Profile: " .. target_name,
            thumbnail = target_avatar,
            color = 0xFFD700, -- Gold Color
            fields = {
                -- We use 'inline = true' so they sit next to each other
                { name = "Messages Sent", value = xp, inline = true },
                { name = "BitCubes", value = bal, inline = true },
                
                -- This one is 'inline = false', so it drops to the next line
                { name = "User ID", value = tostring(target_id), inline = false }
            },
            footer = { text = "Navi Global System" }
        })
    end
end)