print("--- Loading Reaction Roles ---")

-- CONFIG: You can edit this table manually for now, or use the command below
-- Format: [MessageID] = { [Emoji] = RoleID }
local role_map = {
    -- Example: Replace these IDs with real ones from your server!
    -- ["1234567890"] = { ["✅"] = "9876543210" }
}

-- 1. Command to Create a Reaction Role Message
navi.create_command("rr_setup", function(msg, args)
    if not args[1] then
        navi.say(msg.channel_id, "Usage: !rr_setup @Role Description...")
        return
    end

    -- Dirty hack to parse role ID from mention "<@&12345>" -> "12345"
    local role_id = args[1]:match("%d+")
    if not role_id then
        navi.say(msg.channel_id, "Please mention a valid role!")
        return
    end
    
    table.remove(args, 1)
    local description = table.concat(args, " ")

    -- Send the message
    navi.say(msg.channel_id, "React with ✅ to get the role: **" .. description .. "**")
    
    -- We need a way to know the ID of the message we just sent to save it.
    -- Since our navi.say is async/fire-and-forget, we can't get the ID back immediately in Lua yet.
    -- SO: We will ask the user to right-click copy ID for now.
    navi.say(msg.channel_id, "(Right-click the message above, Copy ID, and use `!rr_link <msg_id> <role_id> ✅`)")
end)

-- 2. Link a Message to a Role (The logic part)
navi.create_command("rr_link", function(msg, args)
    local msg_id = args[1]
    local role_id = args[2]
    local emoji = args[3]

    if not msg_id or not role_id or not emoji then
        navi.say(msg.channel_id, "Usage: !rr_link <msg_id> <role_id> <emoji>")
        return
    end

    if not role_map[msg_id] then role_map[msg_id] = {} end
    role_map[msg_id][emoji] = role_id
    
    -- Bot reacts to show it's working
    navi.react(msg.channel_id, msg_id, emoji)
    navi.say(msg.channel_id, "✅ Linked! Reacting adds the role.")
end)


-- 3. The Event Listeners
function on_reaction_add(evt)
    if evt.user_id == navi.bot_id then return end -- Ignore ourselves

    local rules = role_map[evt.message_id]
    if rules then
        local role_to_give = rules[evt.emoji]
        if role_to_give then
            print("Giving role " .. role_to_give .. " to " .. evt.user_id)
            navi.add_role(evt.guild_id, evt.user_id, role_to_give)
        end
    end
end

function on_reaction_remove(evt)
    if evt.user_id == navi.bot_id then return end

    local rules = role_map[evt.message_id]
    if rules then
        local role_to_take = rules[evt.emoji]
        if role_to_take then
            print("Taking role " .. role_to_take .. " from " .. evt.user_id)
            navi.remove_role(evt.guild_id, evt.user_id, role_to_take)
        end
    end
end