# Events & Timers

## Inter-Plugin Event Bus

The event bus lets plugins communicate without direct coupling. One plugin emits an event; any number of others can listen for it.

### `navi.emit(event_name, data)`

Publishes an event to all current listeners.

- `event_name` (string): A namespaced string. Convention: `"plugin_name:event"`.
- `data` (any): Data passed to the listeners. Usually a table.

```lua
navi.emit("economy:balance_changed", {
    user_id        = user_id,
    old_balance    = old_balance,
    new_balance    = new_balance,
    amount_changed = amount
})
```

### `navi.on(event_name, callback)`

Subscribes to an event. The callback receives whatever data was passed to `emit`.

```lua
navi.on("economy:balance_changed", function(data)
    navi.log.info(data.user_id .. " now has " .. data.new_balance .. " credits")
end)
```

### Built-in Events

| Event | Data | Description |
| --- | --- | --- |
| `"message"` | `NaviMsg` | Fired for every message (same data as `navi.register`) |
| `"member_join"` | `{user_id, username, guild_id}` | Fired when a new member joins the guild |

You can use `navi.on("message", ...)` as an alternative to `navi.register`. Both work identically.

### Public Plugin APIs via Events

The economy plugin exposes two events as a public API any plugin can use:

```lua
-- Give a user money (from any plugin)
navi.emit("economy:add", { user_id = user_id, amount = 50 })

-- Take money from a user (from any plugin)
navi.emit("economy:remove", { user_id = user_id, amount = 25 })
```

This is the recommended pattern for cross-plugin data modification. It avoids tight coupling and keeps the economy logic in one place.

## Discord Event Callbacks

These are **optional global functions** you can define in your plugin. If the engine sees them, it calls them when the corresponding Discord event fires.

> **Important:** Only one plugin should define each global callback (e.g. `on_reaction_add`). If two plugins define it, the second one overwrites the first. Use the event bus instead when multiple plugins need the same event.

### `on_reaction_add` and `on_reaction_remove`

```lua
function on_reaction_add(ctx)
    if ctx.emoji == "⭐" then
        navi.say(STARBOARD_CHANNEL, "Starred message: " .. ctx.message_id)
    end
end
```

| Field | Type | Description |
| --- | --- | --- |
| `ctx.user_id` | string\|nil | The user who reacted |
| `ctx.channel_id` | string | The channel containing the message |
| `ctx.message_id` | string | The message that was reacted to |
| `ctx.guild_id` | string\|nil | The guild, or `nil` in DMs |
| `ctx.emoji` | string | Unicode emoji or `<:name:id>` for custom emojis |

### `on_member_leave`

```lua
function on_member_leave(data)
    navi.say(LOG_CHANNEL, "👋 " .. data.username .. " has left the server.")
end
```

| Field | Type | Description |
| --- | --- | --- |
| `data.user_id` | string | The user's snowflake ID |
| `data.username` | string | The user's username |
| `data.guild_id` | string | The guild's snowflake ID |

### `on_message_edit`

Called when any message is edited. `new_content` may be `nil` if Discord's gateway event did not include the message body.

```lua
function on_message_edit(data)
    if data.new_content then
        navi.log.info("Message " .. data.message_id .. " edited: " .. data.new_content)
    end
end
```

| Field | Type | Description |
| --- | --- | --- |
| `data.message_id` | string | The edited message's snowflake ID |
| `data.channel_id` | string | The channel's snowflake ID |
| `data.guild_id` | string\|nil | The guild's snowflake ID |
| `data.new_content` | string\|nil | The new text content, or `nil` if unavailable |

### `on_message_delete`

```lua
function on_message_delete(data)
    navi.log.warn("Message deleted in channel " .. data.channel_id)
end
```

| Field | Type | Description |
| --- | --- | --- |
| `data.message_id` | string | The deleted message's snowflake ID |
| `data.channel_id` | string | The channel's snowflake ID |
| `data.guild_id` | string\|nil | The guild's snowflake ID |

### `on_voice_state_update`

Called whenever a user's voice state changes — joining, leaving, muting, deafening, going live, etc.

```lua
function on_voice_state_update(data)
    if data.channel_id then
        navi.log.info(data.user_id .. " joined voice: " .. data.channel_id)
    else
        navi.log.info(data.user_id .. " left voice entirely")
    end
end
```

| Field | Type | Description |
| --- | --- | --- |
| `data.user_id` | string | The user's snowflake ID |
| `data.guild_id` | string\|nil | The guild's snowflake ID |
| `data.channel_id` | string\|nil | The channel they are now in, or `nil` if they disconnected |
| `data.self_mute` | boolean | Whether the user has muted themselves |
| `data.self_deaf` | boolean | Whether the user has deafened themselves |
| `data.self_stream` | boolean | Whether the user is streaming (Go Live) |
| `data.self_video` | boolean | Whether the user has their camera on |

## Timed Intervals

### `navi.set_interval(callback, amount, unit?)`

Schedules a function to run repeatedly on a fixed interval. All active intervals are automatically cancelled when plugins are reloaded.

- `unit`: `"ms"` (default), `"s"` / `"seconds"`, `"m"` / `"minutes"`, `"h"` / `"hours"`, `"d"` / `"days"`

```lua
-- Check for expired polls every 60 seconds
navi.set_interval(function()
    local now = os.time()
    for pid in (navi.db.get("polls:active") or ""):gmatch("[^,]+") do
        local poll = navi.json.decode(navi.db.get("polls:data:" .. pid) or "")
        if poll and not poll.closed and now >= poll.expires_at then
            close_poll(pid)
        end
    end
end, 60, "s")
```

```lua
-- Refresh a live embed every 5 minutes
navi.set_interval(function()
    local ch = navi.db.get("stats:live_channel_id")
    local id = navi.db.get("stats:live_message_id")
    if ch and id then
        navi.edit_embed(ch, id, build_embed())
    end
end, 5, "m")
```

### `navi.clear_interval(id)`

Cancels a running interval by the ID returned from `set_interval`. No-op if the ID doesn't exist.

```lua
navi.clear_interval(my_interval)
```

---
