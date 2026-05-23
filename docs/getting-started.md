# Getting Started

## How Plugins Work

Every `.lua` file placed in the `plugins/` directory is a **plugin**. The Rust engine:

1. Scans `plugins/` on startup (and every time you press `r` in the TUI).
2. Sorts files to respect any `navi.depends_on()` declarations, then alphabetically.
3. Executes each file once, in order, inside a shared Lua state.
4. After loading all plugins, syncs any registered slash commands to Discord automatically.

All plugin code runs at the top level of the file during loading. This is where you call `navi.register`, `navi.create_slash`, etc. to wire up your handlers.

> **Hot Reload:** Pressing `r` in the TUI re-runs all plugin files from scratch. Your handlers are re-registered and slash commands are re-synced. You do **not** need to restart the bot process to apply code changes.

> **No sandbox:** All plugins share the same Lua globals. Be careful not to overwrite another plugin's globals accidentally — use `local` for everything that doesn't need to be shared.

## Plugin Anatomy

A well-structured plugin follows this order:

```lua
-- ============================================================
-- my_plugin.lua
-- ============================================================

-- (Optional) Declare that this plugin needs another one to load first
navi.depends_on("economy")

-- 1. Log that we're loading (helps debug load errors)
navi.log.info("Loading My Plugin")

-- 2. Declare TUI-configurable settings
navi.register_config("my_plugin", {
    { key = "some_channel", name = "Output Channel", description = "Where to post.", type = "channel", default = "" }
})

-- 3. Local helper functions
local function do_something(user_id)
    return navi.db.get("my_plugin:data:" .. user_id) or "nothing"
end

-- 4. Register message listeners
navi.register(function(msg)
    if msg.content == "hello" then
        navi.say(msg.channel_id, "Hi there!")
    end
end)

-- 5. Register slash commands
navi.create_slash("mycommand", "Does something cool", {}, function(ctx)
    ctx.reply("Hello, " .. ctx.username .. "!")
end)

-- 6. Register component / modal handlers
navi.register_component("my_button", function(ctx)
    ctx.reply("You clicked it!", true)
end)

-- 7. Subscribe to events from other plugins
navi.on("economy:balance_changed", function(data)
    navi.log.info("Balance changed for " .. data.user_id)
end)
```

## Plugin Load Order

By default, plugins load in alphabetical order. If your plugin uses data or functions from another plugin (e.g. casino reading economy balances), you need to make sure the dependency loads first.

### `navi.depends_on(plugin_name)`

Declares that this plugin depends on another. The Rust engine scans for these calls **before** executing any Lua and performs a topological sort to ensure dependencies load first.

Call this at the **very top** of your file, before any other code.

```lua
-- casino.lua
navi.depends_on("economy")  -- economy.lua will always load before casino.lua

navi.log.info("Loading Casino Plugin")
-- ...
```

You can declare multiple dependencies:

```lua
navi.depends_on("economy")
navi.depends_on("leveling")
```

> **Note:** `navi.depends_on` is a **no-op at runtime** — it does nothing when Lua actually executes it. Its only job is to be visible in the source file for the Rust loader to scan.

## Logging

Use `navi.log` to write structured messages to the TUI log pane. This is far better than `print()` because it respects log levels and is visible in the dashboard.

```lua
navi.log.info("Plugin loaded successfully")
navi.log.warn("Config value missing, using default")
navi.log.error("Something went wrong: " .. tostring(err))
```

| Function | Color | When to use |
| --- | --- | --- |
| `navi.log.info(msg)` | White | Normal status messages, load notices |
| `navi.log.warn(msg)` | Yellow | Non-fatal problems, missing optional config |
| `navi.log.error(msg)` | Red | Failures that affect functionality |

---
