# Reference

## Data Types

### Snowflake IDs

Discord uses 64-bit integer "snowflake" IDs for users, channels, guilds, messages, roles, etc. Always use `tostring()` before passing them to `navi.db.set` or string concatenation.

```lua
local uid = tostring(msg.author_id)
navi.db.set("xp:" .. uid, tostring(new_xp))
```

### Discord Mentions

In `description` or message text, you can use Discord's mention syntax:

```lua
"<@"  .. user_id    .. ">"   -- mention a user
"<@&" .. role_id    .. ">"   -- mention a role
"<#"  .. channel_id .. ">"   -- mention a channel
```

## Patterns & Common Mistakes

### Always default database reads

`navi.db.get` returns `nil` if the key doesn't exist. Always provide a fallback:

```lua
-- Bad — can crash if key is missing
local xp = tonumber(navi.db.get("xp:" .. uid))
local new_xp = xp + 10  -- error: attempt to add nil and number

-- Good
local xp = tonumber(navi.db.get("xp:" .. uid)) or 0
local new_xp = xp + 10
```

### Convert IDs to strings early

Message and user IDs come from the engine as numbers. Store them as strings right away:

```lua
navi.register(function(msg)
    local uid = tostring(msg.author_id)
    local bal = tonumber(navi.db.get("balance:" .. uid)) or 0
    navi.db.set("balance:" .. uid, tostring(bal + 5))
end)
```

### Guard against bot feedback loops

If you respond to every message and your bot also sends messages, you'll get an infinite loop:

```lua
navi.register(function(msg)
    if msg.author_bot then return end
    -- safe to process now
end)
```

### Use `ctx.defer()` for slow commands

Any slash command that makes an HTTP request or takes more than ~2 seconds to respond **must** call `ctx.defer()` first:

```lua
navi.create_slash("weather", "Get current weather", {
    { name = "city", description = "City name", type = "string", required = true }
}, function(ctx)
    ctx.defer()
    local body = navi.http.get("https://api.weather.example.com/city/" .. ctx.args.city, nil)
    local data = body and navi.json.decode(body)
    if data then
        ctx.followup("🌤️ " .. ctx.args.city .. ": " .. data.description)
    else
        ctx.followup("❌ Couldn't fetch weather data.", true)
    end
end)
```

### Use the event bus for cross-plugin writes

Don't modify another plugin's data directly when the event bus is available:

```lua
-- Preferred: use the public API
navi.emit("economy:add",    { user_id = uid, amount = 100 })
navi.emit("economy:remove", { user_id = uid, amount = 50  })

-- Acceptable: read-only access to another plugin's data
local balance = tonumber(navi.db.get("economy:balance:" .. uid)) or 0
```

### Store complex data as JSON

The database stores strings only. Encode tables before saving:

```lua
local poll_data = {
    title      = "Best pet?",
    options    = { "Dog", "Cat", "Fish" },
    expires_at = os.time() + 3600,
    closed     = false
}
navi.db.set("polls:data:42", navi.json.encode(poll_data))

local raw  = navi.db.get("polls:data:42")
local poll = raw and navi.json.decode(raw)
if poll and not poll.closed then
    navi.log.info("Poll is still open: " .. poll.title)
end
```

### Use `local` for all helper functions

Declaring a non-local function pollutes Lua globals. If two plugins define a non-local function with the same name, the second one silently overwrites the first.

```lua
-- Bad
function get_balance(uid)
    return tonumber(navi.db.get("balance:" .. uid)) or 0
end

-- Good
local function get_balance(uid)
    return tonumber(navi.db.get("balance:" .. uid)) or 0
end
```

### Reloading and global state

When you press `r` in the TUI, every plugin file is re-executed from scratch:

- All `navi.register`, `navi.create_slash`, `navi.register_component`, and `navi.register_modal` calls run again — this is expected and correct.
- Any **global variables** set during the previous load are still present. If your plugin does something like `INITIALIZED = true` and checks it, that check will be `true` on the second load.
- Intervals are cancelled automatically before reload. You don't need to track and cancel them yourself.

### Config IDs are ready to use

Config values of type `channel` and `role` store the snowflake ID as a string. Use them directly — no extra lookup needed:

```lua
local role_id = navi.db.get("config:my_plugin:mod_role")

navi.say(channel_id, "Paging <@&" .. role_id .. ">!")
navi.add_role(guild_id, user_id, role_id)
```
