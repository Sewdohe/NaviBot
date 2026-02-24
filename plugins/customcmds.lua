navi.register_config("customcmds", {
    { key = "prefix",   name = "Command Prefix", description = "Prefix for custom commands (e.g. !)", type = "string", default = "!" },
    { key = "commands", name = "Commands",       description = "Trigger → response pairs",           type = "list",
      item_schema = {
          { key = "trigger",  name = "Trigger",  type = "string" },
          { key = "response", name = "Response", type = "string" }
      }
    }
})

navi.register(function(msg)
    if msg.author_bot then return end
    local prefix = navi.db.get("config:customcmds:prefix") or "!"
    if msg.content:sub(1, #prefix) ~= prefix then return end

    local input    = msg.content:sub(#prefix + 1):lower()
    local commands = navi.db.get_list("config:customcmds:commands")
    for _, cmd in ipairs(commands) do
        if cmd.trigger and input:sub(1, #cmd.trigger) == cmd.trigger:lower() then
            navi.say(msg.channel_id, cmd.response)
            return
        end
    end
end)
