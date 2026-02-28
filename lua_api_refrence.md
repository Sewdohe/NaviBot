# Navi Bot — Lua Plugin API Reference

This document is the complete guide for writing Lua plugins for the Navi Bot engine. It covers every API function, every data structure, and the patterns used in the real plugins that ship with the bot.

---

## Table of Contents

1. [How Plugins Work](#1-how-plugins-work)
2. [Plugin Anatomy](#2-plugin-anatomy)
3. [Logging](#3-logging)
4. [Configuration (TUI Dashboard)](#4-configuration-tui-dashboard)
5. [Database](#5-database)
6. [Message Listeners](#6-message-listeners)
7. [Slash Commands](#7-slash-commands)
8. [Slash Command Context](#8-slash-command-context)
9. [Buttons and Select Menus](#9-buttons-and-select-menus)
10. [Component Context](#10-component-context)
11. [Modal Dialogs](#11-modal-dialogs)
12. [Modal Context](#12-modal-context)
13. [Embeds and Components (UI)](#13-embeds-and-components-ui)
14. [Inter-Plugin Event Bus](#14-inter-plugin-event-bus)
15. [Global Event Callbacks](#15-global-event-callbacks)
16. [Messaging API](#16-messaging-api)
17. [Member & Role Management](#17-member--role-management)
18. [Channel Management](#18-channel-management)
19. [Discord Cache](#19-discord-cache)
20. [HTTP Client](#20-http-client)
21. [JSON](#21-json)
22. [Timed Intervals](#22-timed-intervals)
23. [Permissions](#23-permissions)
24. [Plugin Load Order](#24-plugin-load-order)
25. [Bot Status / Presence](#25-bot-status--presence)
26. [Data Type Reference](#26-data-type-reference)
27. [Patterns, Tips & Common Mistakes](#27-patterns-tips--common-mistakes)

---

## 1. How Plugins Work

Every `.lua` file placed in the `plugins/` directory is a **plugin**. The Rust engine:

1. Scans `plugins/` on startup (and every time you press `r` in the TUI).
2. Sorts files to respect any `navi.depends_on()` declarations, then alphabetically.
3. Executes each file once, in order, inside a shared Lua state.
4. After loading all plugins, syncs any registered slash commands to Discord automatically.

All plugin code runs at the top level of the file during loading. This is where you call `navi.register`, `navi.create_slash`, etc. to wire up your handlers.

> **Hot Reload:** Pressing `r` in the TUI re-runs all plugin files from scratch. Your handlers are re-registered and slash commands are re-synced. You do **not** need to restart the bot process to apply code changes.

> **No sandbox:** All plugins share the same Lua globals. Be careful not to overwrite another plugin's globals accidentally — use `local` for everything that doesn't need to be shared.

---

## 2. Plugin Anatomy

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

---

## 3. Logging

Use `navi.log` to write structured messages to the TUI log pane. This is far better than `print()` because it respects log levels and is visible in the dashboard.

```lua
navi.log.info("Plugin loaded successfully")
navi.log.warn("Config value missing, using default")
navi.log.error("Something went wrong: " .. tostring(err))
```

| Function | Color | When to use |
|---|---|---|
| `navi.log.info(msg)` | White | Normal status messages, load notices |
| `navi.log.warn(msg)` | Yellow | Non-fatal problems, missing optional config |
| `navi.log.error(msg)` | Red | Failures that affect functionality |

---

## 4. Configuration (TUI Dashboard)

### `navi.register_config(plugin_name, schema)`

Declares settings that can be edited from the TUI (`c` key) and are persisted in SQLite. Call this **once per plugin**, at the top of the file.

- `plugin_name` (string): The unique namespace for this plugin's config (usually the filename without `.lua`).
- `schema` (table): A list of setting definitions.

```lua
navi.register_config("my_plugin", {
    { key = "welcome_channel", name = "Welcome Channel", description = "Where to greet new users.", type = "channel", default = "" },
    { key = "max_points",      name = "Max Points",      description = "Point cap per user.",       type = "number",  default = 1000 },
    { key = "enabled",         name = "Enabled",         description = "Turn the plugin on/off.",   type = "boolean", default = true },
})
```

#### Config Item Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `key` | string | Yes | The DB key used to store the value. Read back as `config:plugin_name:key`. |
| `name` | string | Yes | Human-readable label shown in the TUI. |
| `description` | string | Yes | Help text shown below the field in the TUI. |
| `type` | string | Yes | Controls the input widget. See types below. |
| `default` | any | No | Value written to DB if the user hasn't configured it yet. Omit for `list` fields. |
| `item_schema` | table | Only for `list` | Defines the sub-fields of each list item. |
| `options` | string[] | Only for `enum` | The allowed values shown in the TUI dropdown. |

#### Config Types

| Type | TUI Widget | Notes |
|---|---|---|
| `"string"` | Text input | Free-form text |
| `"number"` | Number input | Stored as a string; use `tonumber()` when reading |
| `"boolean"` | Toggle | `true` / `false` |
| `"channel"` | Channel picker | Stores a channel snowflake ID string |
| `"role"` | Role picker | Stores a role snowflake ID string |
| `"category"` | Category picker | Stores a category (channel group) snowflake ID string |
| `"list"` | Expandable list | Each item is a sub-table; requires `item_schema` |
| `"enum"` | Option dropdown | Stores one of the strings declared in `options`; requires `options` |

#### Reading Config Values

Config values are stored with the key format `config:plugin_name:key`. You read them with `navi.db.get`:

```lua
local channel = navi.db.get("config:my_plugin:welcome_channel")
local max     = tonumber(navi.db.get("config:my_plugin:max_points")) or 1000
local enabled = navi.db.get("config:my_plugin:enabled") == "true"
```

#### List Config

A `list` config field stores multiple structured items — for example, a list of level-up role rewards. Define the sub-fields with `item_schema` and read them back with `navi.db.get_list`.

```lua
navi.register_config("leveling", {
    { key = "role_rewards", name = "Role Rewards", description = "Roles granted at specific levels.", type = "list",
      item_schema = {
          { key = "level",   name = "Level", type = "number" },
          { key = "role_id", name = "Role",  type = "role"   }
      }
    }
})

-- Reading the list back:
local rewards = navi.db.get_list("config:leveling:role_rewards")
for _, item in ipairs(rewards) do
    -- item.level and item.role_id are available as strings
    navi.log.info("Level " .. item.level .. " → Role " .. item.role_id)
end
```

Each entry in `item_schema` supports these fields:

| Field | Required | Description |
|---|---|---|
| `key` | Yes | The key used inside each item table |
| `name` | Yes | Human-readable label shown in the TUI |
| `type` | Yes | Sub-field input type (see below) |
| `options` | Only for `enum` | List of valid string values shown in the dropdown |

**Sub-field types** (all types except `"list"` are allowed):

| Type | TUI Widget |
|---|---|
| `"string"` | Text input |
| `"number"` | Text input (use `tonumber()` when reading) |
| `"boolean"` | Toggle |
| `"channel"` | Channel picker |
| `"role"` | Role picker |
| `"category"` | Category picker |
| `"enum"` | Option dropdown — requires `options` |

#### Enum Config

Use `type = "enum"` when a field (top-level or inside a list's `item_schema`) must be one of a fixed set of strings. The TUI shows a green dropdown instead of a free-text box, preventing invalid input.

```lua
-- Top-level enum field
navi.register_config("myplugin", {
    {
        key = "mode",
        name = "Operating Mode",
        description = "Controls how the plugin behaves.",
        type = "enum",
        options = { "strict", "lenient", "disabled" },
        default = "lenient"
    }
})

local mode = navi.db.get("config:myplugin:mode")  -- "strict", "lenient", or "disabled"
```

```lua
-- Enum sub-field inside a list
navi.register_config("permissions", {
    { key = "mappings", name = "Role → Permission Level", description = "Map roles to levels.", type = "list",
      item_schema = {
          { key = "role_id", name = "Discord Role",     type = "role" },
          { key = "level",   name = "Permission Level", type = "enum",
            options = { "helper", "moderator", "admin" } }
      }
    }
})
```

---

## 5. Database

The database is a single SQLite table (`kv_store`) with `key` and `value` columns. All values are stored as strings. The Lua API gives you get/set, raw SQL queries, and list helpers.

### Automatic Namespacing

`navi.db.get` and `navi.db.set` **automatically prepend the calling plugin's filename** as a namespace. A call to `navi.db.get("score")` from `trivia.lua` actually reads the key `trivia:score`.

**To bypass namespacing** and use an exact key, include a `:` anywhere in the key string:

```lua
-- From casino.lua — reads economy.lua's balance data directly
local balance = navi.db.get("economy:balance:" .. user_id)
```

This is intentional. Cross-plugin data access is fine as long as you know the key format.

### `navi.db.get(key)`

Reads a value from the database.

- **Returns:** `string | nil` — the value, or `nil` if the key doesn't exist.

```lua
local xp = navi.db.get("xp:" .. user_id)
if xp == nil then
    xp = "0"  -- default value
end
local xp_num = tonumber(xp) or 0
```

### `navi.db.set(key, value)`

Writes a value to the database. Creates the key if it doesn't exist; overwrites it if it does.

- `value` can be a string, number, or boolean. Numbers and booleans are automatically converted to strings.

```lua
navi.db.set("xp:" .. user_id, tostring(new_xp))
navi.db.set("score", 42)          -- stored as "42"
navi.db.set("enabled", true)      -- stored as "true"
```

> **Important:** Always use `tonumber()` when reading a value you intend to do math with. The database always gives you strings back.

### `navi.db.query(sql)`

Executes a raw SQL statement against the `kv_store` table. Returns an array of row tables.

- Each row has a `key` field and a `value` field.
- Use this for sorted queries, `LIKE` pattern searches, and leaderboards — things you can't do with simple get/set.

```lua
-- Top 10 XP earners, sorted descending
local rows = navi.db.query(
    "SELECT key, value FROM kv_store " ..
    "WHERE key LIKE 'leveling:xp:%' " ..
    "ORDER BY CAST(value AS INTEGER) DESC LIMIT 10"
)

for i, row in ipairs(rows) do
    -- Extract the user_id from the key "leveling:xp:<user_id>"
    local user_id = row.key:match("leveling:xp:(.+)")
    navi.log.info(i .. ". " .. user_id .. " — " .. row.value .. " XP")
end
```

### `navi.db.get_list(key)`

Reads a `list`-type config field (registered with `navi.register_config`) and returns it as an array of tables. Each table has the keys defined in `item_schema`.

```lua
local rewards = navi.db.get_list("config:leveling:role_rewards")
for _, r in ipairs(rewards) do
    print(r.level, r.role_id)
end
```

---

## 6. Message Listeners

### `navi.register(callback)`

Registers a function that is called every time any message is sent in a channel the bot can see. You can register multiple listeners from multiple plugins — they all fire independently.

```lua
navi.register(function(msg)
    -- Always ignore bots to prevent feedback loops
    if msg.author_bot then return end

    -- React to a specific word
    if msg.content:lower():find("good bot") then
        navi.react(tostring(msg.channel_id), tostring(msg.message_id), "❤️")
    end
end)
```

#### Message Object (`msg`)

| Field | Type | Description |
|---|---|---|
| `msg.content` | string | The text of the message |
| `msg.message_id` | number | The message's snowflake ID |
| `msg.channel_id` | number | The channel's snowflake ID |
| `msg.author` | string | The sender's username |
| `msg.author_id` | number | The sender's snowflake ID |
| `msg.author_bot` | boolean | `true` if the sender is a bot or webhook |
| `msg.author_avatar` | string | URL to the sender's avatar |
| `msg.guild_id` | string\|nil | The guild's snowflake ID, or `nil` in DMs |
| `msg.mentions` | table | Array of mentioned user objects (`{id, name, avatar}`) |
| `msg.attachments` | string[] | Array of attachment URLs |

> **Tip:** `msg.author_id` and `msg.channel_id` are numbers. Use `tostring()` when passing them to `navi.db.set`, string concatenation, or functions that expect a string.

---

## 7. Slash Commands

### `navi.create_slash(name, description, options, callback)`

Registers a slash command. After registering, slash commands are automatically synced to Discord when plugins reload.

- `name` (string): The command name. Lowercase, no spaces (e.g. `"balance"`).
- `description` (string): The help text shown in Discord's command picker.
- `options` (table): A list of argument definitions. Pass `{}` if the command takes no arguments.
- `callback` (function): Called when the command is used. Receives a `ctx` object (see [Section 8](#8-slash-command-context)).

```lua
-- A simple no-argument command
navi.create_slash("ping", "Check if the bot is alive", {}, function(ctx)
    ctx.reply("Pong! 🏓")
end)
```

```lua
-- A command with arguments
navi.create_slash("greet", "Greet a user", {
    { name = "user",   description = "Who to greet", type = "user",    required = true  },
    { name = "shout",  description = "Shout it?",    type = "boolean", required = false }
}, function(ctx)
    local target = ctx.args.user    -- a user snowflake ID string
    local shout  = ctx.args.shout   -- a boolean (true/false) or nil if not provided

    local message = "Hey, <@" .. target .. ">!"
    if shout then
        message = string.upper(message)
    end
    ctx.reply(message)
end)
```

#### Command Grouping

If a single plugin file registers **two or more** slash commands, Discord automatically groups them under the plugin's filename as a **parent command**. For example, a plugin named `casino.lua` that calls `create_slash` for `coinflip`, `slots`, and `dice` will appear in Discord as `/casino coinflip`, `/casino slots`, and `/casino dice`.

If a plugin only registers **one** command, it stays flat (e.g. `/rank` from `leveling.lua` would stay as `/rank` if it were the only command, but since `leveling.lua` registers two commands it becomes `/leveling rank`).

This is fully automatic — you don't need to change any Lua code.

#### Slash Option Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | Yes | Argument name (lowercase, no spaces) |
| `description` | string | Yes | Help text in Discord |
| `type` | string | Yes | See option types below |
| `required` | boolean | No | Whether the user must provide this arg. Default: `false` |
| `autocomplete` | function | No | If set, Discord calls this as the user types. See below. |

#### Option Types

| Type | Lua value in `ctx.args` | Notes |
|---|---|---|
| `"string"` | `string` | Plain text |
| `"integer"` | `number` | Whole number; safe to use directly in math |
| `"number"` | `number` | Float/decimal |
| `"boolean"` | `boolean` | Real `true`/`false` |
| `"user"` | `string` | The selected user's snowflake ID |
| `"channel"` | `string` | The selected channel's snowflake ID |
| `"role"` | `string` | The selected role's snowflake ID |

> **Note:** `user`, `channel`, and `role` options give you an ID string, not the full object. Use `navi.get_member` if you need more info about a user.

#### Autocomplete

If an option has a function in its `autocomplete` field, Discord will call that function as the user types in that argument slot. Your function receives a context table and must return up to 25 `{name, value}` pairs.

```lua
navi.create_slash("give_item", "Give an item to a user", {
    {
        name = "item",
        description = "The item name",
        type = "string",
        required = true,
        autocomplete = function(ctx)
            -- ctx.current_value is what the user has typed so far
            local all_items = { "Sword", "Shield", "Potion", "Arrow", "Staff" }
            local results = {}
            for _, item in ipairs(all_items) do
                if item:lower():find(ctx.current_value:lower(), 1, true) then
                    table.insert(results, { name = item, value = item })
                end
            end
            return results
        end
    }
}, function(ctx)
    ctx.reply("You received a " .. ctx.args.item .. "!")
end)
```

The `autocomplete` callback receives:
| Field | Type | Description |
|---|---|---|
| `ctx.current_value` | string | What the user has typed so far (may be empty) |
| `ctx.user_id` | string | The user's snowflake ID |
| `ctx.guild_id` | string\|nil | The guild's snowflake ID |

---

## 8. Slash Command Context

The `ctx` object passed to a slash command callback.

| Field / Method | Type | Description |
|---|---|---|
| `ctx.user_id` | number | The invoking user's snowflake ID |
| `ctx.username` | string | The invoking user's username |
| `ctx.channel_id` | string | The channel's snowflake ID |
| `ctx.guild_id` | string\|nil | The guild's snowflake ID, `nil` in DMs |
| `ctx.member_roles` | string[] | Array of the invoking member's role IDs |
| `ctx.args` | table | Named arguments provided by the user |
| `ctx.reply(msg, ephemeral?)` | function | Send a plain-text response |
| `ctx.reply_embed(data, ephemeral?)` | function | Send an embed as the response |
| `ctx.defer(ephemeral?)` | function | Acknowledge the interaction immediately (for slow commands) |
| `ctx.followup(msg, ephemeral?)` | function | Send a follow-up message after `ctx.defer()` |
| `ctx.followup_embed(data, ephemeral?)` | function | Send an embed follow-up after `ctx.defer()` |
| `ctx.modal(custom_id, title, fields)` | function | Respond with a modal dialog form |

### `ctx.reply(message, ephemeral?)`

Sends a plain-text response to the slash command. This is the standard way to respond.

- `message` (string): The text to send.
- `ephemeral` (boolean, optional): If `true`, only the user who ran the command can see the response. Default: `false`.

```lua
ctx.reply("Done!")
ctx.reply("This is a secret.", true)  -- only you can see this
```

> **One response per interaction.** Discord only allows one direct response per command invocation. If you want to send a public embed as well as a silent acknowledgement, use `navi.send_message` for the embed and `ctx.reply("...", true)` for the hidden confirmation.

### `ctx.reply_embed(data, ephemeral?)`

Sends a rich embed as the direct response to the command.

- `data` (table): An embed table. See [Section 13](#13-embeds-and-components-ui) for the full structure.
- `ephemeral` (boolean, optional): If `true`, only the invoking user sees it.

```lua
ctx.reply_embed({
    title = "Your Stats",
    description = "Here is your profile.",
    color = 0x3498DB,
    fields = {
        { name = "Level", value = "12", inline = true },
        { name = "XP",    value = "840", inline = true }
    }
}, true)
```

### `ctx.defer(ephemeral?)` + `ctx.followup()`

For commands that take more than ~3 seconds (API calls, heavy computation), you must **defer** first. Deferring tells Discord "I received this, give me a moment" which prevents the "This application did not respond" error.

`ctx.defer` **blocks** until Discord confirms the acknowledgment, so it is safe to do slow work immediately after calling it.

```lua
navi.create_slash("slow_command", "Fetches data from the web", {}, function(ctx)
    -- Tell Discord to wait
    ctx.defer()

    -- Now do the slow work
    local body = navi.http.get("https://api.example.com/data", nil)
    local data = body and navi.json.decode(body)

    -- Respond with a follow-up (can be called multiple times)
    if data then
        ctx.followup("Got it: " .. tostring(data.result))
    else
        ctx.followup("Request failed.", true)
    end
end)
```

`ctx.followup_embed(data, ephemeral?)` works the same way but sends an embed:

```lua
ctx.defer()
-- ... do slow work ...
ctx.followup_embed({
    title = "Result",
    description = "Here's what I found."
})
```

### `ctx.modal(custom_id, title, fields)`

Instead of replying with a message, you can open a **modal dialog** (a popup form). The user fills it in and submits. You handle the submission with `navi.register_modal`.

> **Note:** A modal is a response to the interaction — you cannot also call `ctx.reply` in the same handler. Choose one or the other.

- `custom_id` (string): Identifier used to match the modal to its `register_modal` handler.
- `title` (string): The title of the popup window.
- `fields` (table): List of text input definitions.

```lua
navi.create_slash("feedback", "Submit feedback", {}, function(ctx)
    ctx.modal("feedback_form", "Share Your Feedback", {
        { id = "subject",  label = "Subject",          style = "short",     placeholder = "Brief topic",    required = true  },
        { id = "body",     label = "Your feedback",    style = "paragraph", placeholder = "Tell us more…",  required = true  },
        { id = "rating",   label = "Rating (1-10)",    style = "short",     placeholder = "e.g. 8",         required = false },
    })
end)

navi.register_modal("feedback_form", function(ctx)
    local subject = ctx.values.subject
    local body    = ctx.values.body
    local rating  = ctx.values.rating or "not given"

    navi.say(FEEDBACK_CHANNEL, "**Feedback from " .. ctx.username .. "**\n**" .. subject .. "**\n" .. body .. "\nRating: " .. rating)
    ctx.reply("Thanks for your feedback!", true)
end)
```

#### Modal Field Definition

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | Yes | Key used to read the value in `ctx.values` |
| `label` | string | Yes | The label shown above the input |
| `style` | `"short"` \| `"paragraph"` | No | Single-line vs multi-line input. Default: `"short"` |
| `placeholder` | string | No | Greyed-out hint text |
| `required` | boolean | No | Whether the user must fill this in. Default: `true` |

---

## 9. Buttons and Select Menus

Buttons and select menus are attached to messages via the `components` field in an embed table (see [Section 13](#13-embeds-and-components-ui)). When clicked or selected, they fire a `register_component` handler.

### `navi.register_component(custom_id, callback)`

Registers a handler function that fires when a button with the matching `custom_id` is clicked, or when a select menu with the matching `id` has its value changed.

- `custom_id` (string): The ID you put in the `id` field of the button or select menu.
- `callback` (function): Called with a `NaviComponentCtx` object.

```lua
-- Send a message with a button
navi.send_message(channel_id, {
    description = "Click the button!",
    components = {
        { type = "button", id = "my_button", label = "Click Me", style = "primary" }
    }
})

-- Handle the click
navi.register_component("my_button", function(ctx)
    ctx.reply("You clicked the button, " .. ctx.username .. "!", true)
end)
```

#### Button Styles

| Style | Color | Use for |
|---|---|---|
| `"primary"` | Blue | Main action |
| `"secondary"` | Grey | Secondary/neutral action |
| `"success"` | Green | Confirming or positive action |
| `"danger"` | Red | Destructive or risky action |
| `"link"` / `"url"` | Grey (opens URL) | External links — requires a `url` field instead of `id` |

#### Link Buttons

Link buttons open a URL instead of triggering an interaction. They do not need a `register_component` handler.

```lua
components = {
    { type = "button", style = "link", label = "Visit Website", url = "https://example.com" }
}
```

#### Select Menus

A string-select dropdown. The selected value(s) are available in `ctx.values` inside the handler.

```lua
navi.send_message(channel_id, {
    description = "Pick your favorite color.",
    components = {
        {
            type = "select",
            id = "color_picker",
            placeholder = "Choose a color…",
            options = {
                { label = "Red",   value = "red",   description = "Warm and bold" },
                { label = "Blue",  value = "blue",  description = "Cool and calm" },
                { label = "Green", value = "green", emoji = "🌿" }
            }
        }
    }
})

navi.register_component("color_picker", function(ctx)
    local selected = ctx.values[1]  -- ctx.values is an array
    ctx.reply("You chose: " .. selected, true)
end)
```

---

## 10. Component Context

The `ctx` object passed to a `register_component` handler.

| Field / Method | Type | Description |
|---|---|---|
| `ctx.custom_id` | string | The `id` of the clicked button or selected menu |
| `ctx.user_id` | string | The snowflake ID of the user who clicked |
| `ctx.username` | string | The username of the user who clicked |
| `ctx.channel_id` | string | The channel's snowflake ID |
| `ctx.guild_id` | string\|nil | The guild's snowflake ID, `nil` in DMs |
| `ctx.member_roles` | string[] | Role IDs of the clicking member |
| `ctx.values` | string[] | Selected values (non-empty only for select menus) |
| `ctx.reply(msg, ephemeral)` | function | Reply to the interaction |
| `ctx.reply_embed(data, ephemeral?)` | function | Reply with an embed |
| `ctx.modal(custom_id, title, fields)` | function | Respond with a modal dialog |

```lua
navi.register_component("btn_verify_human", function(ctx)
    local role_id = navi.db.get("config:verification:verified_role")
    if not role_id or role_id == "" then
        ctx.reply("Not configured yet.", true)
        return
    end
    navi.add_role(ctx.guild_id, ctx.user_id, role_id)
    ctx.reply("✅ You are now verified! Welcome.", true)
end)
```

---

## 11. Modal Dialogs

### `navi.register_modal(custom_id, callback)`

Registers a handler for when a user submits a modal form with the matching `custom_id`. The `custom_id` must match what was passed to `ctx.modal(...)` when the modal was opened.

```lua
navi.register_modal("report_form", function(ctx)
    local reason = ctx.values.reason
    navi.say(LOG_CHANNEL, ctx.username .. " filed a report: " .. reason)
    ctx.reply("Your report was received.", true)
end)
```

---

## 12. Modal Context

The `ctx` object passed to a `register_modal` handler.

| Field / Method | Type | Description |
|---|---|---|
| `ctx.custom_id` | string | The modal's `custom_id` |
| `ctx.user_id` | string | The submitting user's snowflake ID |
| `ctx.username` | string | The submitting user's username |
| `ctx.channel_id` | string | The channel's snowflake ID |
| `ctx.guild_id` | string\|nil | The guild's snowflake ID |
| `ctx.member_roles` | string[] | Role IDs of the submitting member |
| `ctx.values` | table | Map of `field_id → submitted_value` (all strings) |
| `ctx.reply(msg, ephemeral)` | function | Reply to the modal submission |
| `ctx.reply_embed(data, ephemeral?)` | function | Reply with an embed |

---

## 13. Embeds and Components (UI)

An embed is a rich message card. You pass the same embed table structure to `navi.send_message`, `ctx.reply_embed`, and `ctx.followup_embed`.

### Embed Table Structure

```lua
{
    title       = "Optional title",
    description = "Main body text. Supports **markdown**.",
    color       = 0x3498DB,  -- hex color integer
    image       = "https://example.com/image.png",  -- large image at bottom
    fields = {
        { name = "Field 1", value = "Some text",  inline = true  },
        { name = "Field 2", value = "More text",  inline = true  },
        { name = "Long one", value = "Full width", inline = false },
    },
    components = {
        -- Buttons and/or a select menu (see below)
    }
}
```

All fields are optional. You can use any combination.

| Field | Type | Description |
|---|---|---|
| `title` | string | Bold title at the top |
| `description` | string | Main body text (supports Discord markdown) |
| `color` | number | Hex color as an integer, e.g. `0xFF0000` for red |
| `image` | string | URL of a large image to display at the bottom |
| `fields` | table | Array of `{name, value, inline?}` objects |
| `components` | table | Array of buttons and/or one select menu |

#### Embed Fields

```lua
fields = {
    { name = "Level",  value = "42",      inline = true  },
    { name = "XP",     value = "12,500",  inline = true  },
    { name = "Status", value = "Active",  inline = false },
}
```

When `inline = true`, up to 3 fields display side-by-side. Use `inline = false` (or omit it) for fields that should take the full width.

#### Components in Embeds

You can attach up to 5 buttons and/or 1 select menu to any message. Mix them freely in the `components` array — the engine lays them out into Discord action rows automatically.

```lua
navi.send_message(channel_id, {
    title = "Choose your role",
    color = 0x5865F2,
    components = {
        { type = "button", id = "role_warrior",  label = "Warrior",  style = "primary"   },
        { type = "button", id = "role_mage",     label = "Mage",     style = "primary"   },
        { type = "button", id = "role_rogue",    label = "Rogue",    style = "secondary" },
        {
            type = "select",
            id = "faction_picker",
            placeholder = "Pick a faction…",
            options = {
                { label = "Alliance", value = "alliance" },
                { label = "Horde",    value = "horde"    },
            }
        }
    }
})
```

#### Common Colors

```lua
0x2ECC71  -- Emerald green  (success)
0xE74C3C  -- Alizarin red   (error / danger)
0xF1C40F  -- Sunflower gold (warning / economy)
0x3498DB  -- Peter River blue (info)
0x5865F2  -- Discord blurple
0x99AAB5  -- Grey/neutral
0x000000  -- Black (removes the color bar)
```

---

## 14. Inter-Plugin Event Bus

The event bus lets plugins talk to each other without direct coupling. One plugin emits an event; any number of other plugins can listen for it.

### `navi.emit(event_name, data)`

Publishes an event to all current listeners.

- `event_name` (string): A namespaced string. Convention: `"plugin_name:event"`.
- `data` (any): Data passed to the listeners. Usually a table.

```lua
-- In economy.lua, after a balance change
navi.emit("economy:balance_changed", {
    user_id       = user_id,
    old_balance   = old_balance,
    new_balance   = new_balance,
    amount_changed = amount
})
```

### `navi.on(event_name, callback)`

Subscribes to an event. The callback receives whatever data was passed to `emit`.

```lua
-- In leveling.lua, listen for economy changes
navi.on("economy:balance_changed", function(data)
    navi.log.info(data.user_id .. " now has " .. data.new_balance .. " credits")
end)
```

#### Built-in Events

The engine itself emits one event:

| Event | Data | Description |
|---|---|---|
| `"message"` | `NaviMsg` | Fired for every message (same data as `navi.register`) |
| `"member_join"` | `{user_id, username, guild_id}` | Fired when a new member joins the guild |

You can use `navi.on("message", ...)` as an alternative to `navi.register`. Both work identically.

```lua
-- Alternative to navi.register
navi.on("message", function(msg)
    if msg.content == "!hello" then
        navi.say(msg.channel_id, "Hi!")
    end
end)
```

#### Public Plugin APIs

The economy plugin exposes two events as a "public API" that any other plugin can call:

```lua
-- Give a user money (from any plugin)
navi.emit("economy:add", { user_id = user_id, amount = 50 })

-- Take money from a user (from any plugin)
navi.emit("economy:remove", { user_id = user_id, amount = 25 })
```

This is the recommended pattern for inter-plugin data modification. It avoids tight coupling and keeps the economy logic in one place.

---

## 15. Global Event Callbacks

These are **optional global functions** you can define in your plugin. If the engine sees them, it calls them when the corresponding Discord event fires.

### `on_reaction_add` and `on_reaction_remove`

Called when a reaction is added or removed from any message.

```lua
function on_reaction_add(ctx)
    -- ctx.user_id, ctx.channel_id, ctx.message_id, ctx.guild_id, ctx.emoji
    if ctx.emoji == "⭐" then
        navi.say(STARBOARD_CHANNEL, "Starred message: " .. ctx.message_id)
    end
end

function on_reaction_remove(ctx)
    navi.log.info(ctx.user_id .. " removed " .. ctx.emoji)
end
```

| Field | Type | Description |
|---|---|---|
| `ctx.user_id` | string\|nil | The user who reacted |
| `ctx.channel_id` | string | The channel containing the message |
| `ctx.message_id` | string | The message that was reacted to |
| `ctx.guild_id` | string\|nil | The guild, or `nil` in DMs |
| `ctx.emoji` | string | Unicode emoji or `<:name:id>` for custom emojis |

### `on_member_leave`

Called when a member leaves or is removed from the server.

```lua
function on_member_leave(data)
    navi.say(LOG_CHANNEL, "👋 " .. data.username .. " has left the server.")
end
```

| Field | Type | Description |
|---|---|---|
| `data.user_id` | string | The user's snowflake ID |
| `data.username` | string | The user's username |
| `data.guild_id` | string | The guild's snowflake ID |

### `on_message_edit`

Called when any message is edited. Note: `new_content` may be `nil` if Discord's gateway event did not include the message body (common for cached messages).

```lua
function on_message_edit(data)
    if data.new_content then
        navi.log.info("Message " .. data.message_id .. " edited: " .. data.new_content)
    end
end
```

| Field | Type | Description |
|---|---|---|
| `data.message_id` | string | The edited message's snowflake ID |
| `data.channel_id` | string | The channel's snowflake ID |
| `data.guild_id` | string\|nil | The guild's snowflake ID |
| `data.new_content` | string\|nil | The new text content, or `nil` if unavailable |

### `on_message_delete`

Called when a message is deleted.

```lua
function on_message_delete(data)
    navi.log.warn("Message deleted in channel " .. data.channel_id)
end
```

| Field | Type | Description |
|---|---|---|
| `data.message_id` | string | The deleted message's snowflake ID |
| `data.channel_id` | string | The channel's snowflake ID |
| `data.guild_id` | string\|nil | The guild's snowflake ID |

### `on_voice_state_update`

Called whenever a user's voice state changes — joining a channel, leaving, muting, deafening, going live, etc.

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
|---|---|---|
| `data.user_id` | string | The user's snowflake ID |
| `data.guild_id` | string\|nil | The guild's snowflake ID |
| `data.channel_id` | string\|nil | The channel they are now in, or `nil` if they disconnected |
| `data.self_mute` | boolean | Whether the user has muted themselves |
| `data.self_deaf` | boolean | Whether the user has deafened themselves |
| `data.self_stream` | boolean | Whether the user is streaming (Go Live) |
| `data.self_video` | boolean | Whether the user has their camera on |

> **Important:** Only one plugin should define each global callback (e.g. `on_reaction_add`). If two plugins both define it, the second one overwrites the first. Use `navi.on("message", ...)` and similar event bus patterns for multi-plugin handling instead.

---

## 16. Messaging API

### `navi.say(channel_id, text)`

Sends a plain-text message. Fire-and-forget (non-blocking).

```lua
navi.say(msg.channel_id, "Hello world!")
navi.say(tostring(msg.channel_id), "Also works with string IDs")
```

### `navi.say_sync(channel_id, text)`

Like `navi.say` but **blocks until the message is sent** and returns the new message's snowflake ID as a string, or `nil` on error.

```lua
local msg_id = navi.say_sync(channel_id, "This message was sent synchronously")
if msg_id then
    navi.log.info("Sent as message " .. msg_id)
end
```

### `navi.send_message(channel_id, embed_table)`

Sends a rich embed (with optional buttons/selects). Fire-and-forget. See [Section 13](#13-embeds-and-components-ui) for the full embed structure.

```lua
navi.send_message(channel_id, {
    title = "Welcome!",
    description = "Glad to have you here.",
    color = 0x2ECC71
})
```

### `navi.dm(user_id, text)`

Sends a plain-text direct message to a user. Fire-and-forget. If the user has DMs disabled, the error is logged but does not crash the plugin.

```lua
navi.dm(ctx.user_id, "Hey, check this out!")
```

### `navi.react(channel_id, message_id, emoji)`

Adds a reaction to a message.

- `emoji`: A Unicode emoji (`"❤️"`, `"⭐"`) or a custom emoji string in the format `"name:id"`.

```lua
navi.react(tostring(msg.channel_id), tostring(msg.message_id), "⭐")
```

### `navi.edit_message(channel_id, message_id, content)`

Edits the text content of a message the bot previously sent.

```lua
navi.edit_message(channel_id, message_id, "Updated content")
```

### `navi.delete_message(channel_id, message_id)`

Deletes a message.

```lua
navi.delete_message(tostring(msg.channel_id), tostring(msg.message_id))
```

### `navi.fetch_message(channel_id, message_id)`

Fetches a message from Discord by ID. **Blocks until complete.** Returns a table or `nil` if not found.

```lua
local fetched = navi.fetch_message(channel_id, message_id)
if fetched then
    navi.log.info("Message content: " .. fetched.content)
end
```

| Field | Type | Description |
|---|---|---|
| `fetched.message_id` | string | The message's snowflake ID |
| `fetched.channel_id` | string | The channel's snowflake ID |
| `fetched.guild_id` | string\|nil | The guild's snowflake ID |
| `fetched.content` | string | The message text |
| `fetched.author_id` | string | The author's snowflake ID |
| `fetched.author` | string | The author's username |
| `fetched.attachments` | string[] | Array of attachment URLs |

---

## 17. Member & Role Management

### `navi.add_role(guild_id, user_id, role_id)`

Assigns a role to a member. All arguments are strings.

```lua
navi.add_role(ctx.guild_id, ctx.user_id, verified_role_id)
```

### `navi.remove_role(guild_id, user_id, role_id)`

Removes a role from a member.

```lua
navi.remove_role(ctx.guild_id, ctx.user_id, unverified_role_id)
```

### `navi.get_member(guild_id, user_id)`

Fetches live member info from Discord. **Blocks until complete.** Returns a table or `nil`.

```lua
local member = navi.get_member(ctx.guild_id, ctx.user_id)
if member then
    navi.log.info("Display name: " .. member.display_name)
    navi.log.info("Roles: " .. table.concat(member.roles, ", "))
end
```

| Field | Type | Description |
|---|---|---|
| `member.user_id` | string | The member's snowflake ID |
| `member.username` | string | The member's username |
| `member.display_name` | string | Nickname if set, otherwise username |
| `member.nickname` | string\|nil | Server-specific nickname, or `nil` |
| `member.joined_at` | string\|nil | ISO 8601 timestamp of when they joined |
| `member.roles` | string[] | Array of role snowflake IDs |

### `navi.kick(guild_id, user_id, reason?)`

Kicks a member from the guild. The reason is optional and will appear in the audit log.

```lua
navi.kick(ctx.guild_id, target_user_id, "Spam")
```

### `navi.ban(guild_id, user_id, delete_message_days, reason?)`

Bans a member. `delete_message_days` (0–7) controls how many days of their recent messages to delete.

```lua
navi.ban(ctx.guild_id, target_user_id, 1, "Harassment")
```

### `navi.unban(guild_id, user_id)`

Removes a ban.

```lua
navi.unban(ctx.guild_id, target_user_id)
```

### `navi.timeout(guild_id, user_id, duration_seconds)`

Times out a member (they cannot send messages or join voice). Pass `0` to remove an existing timeout.

```lua
navi.timeout(ctx.guild_id, target_user_id, 600)  -- 10 minutes
navi.timeout(ctx.guild_id, target_user_id, 0)    -- remove timeout
```

---

## 18. Channel Management

### `navi.create_channel(guild_id, name, options)`

Creates a new text channel in the guild. Useful for dynamic systems like tickets.

```lua
navi.create_channel(ctx.guild_id, "ticket-username", {
    category_id     = category_id,      -- place under this category
    user_id         = ctx.user_id,      -- grant this user private access
    role_id         = support_role_id,  -- grant this role private access
    welcome_message = "Welcome! Staff will be with you shortly.",
    close_button    = true              -- attach a red "Close Ticket" button
})
```

| Option | Type | Description |
|---|---|---|
| `category_id` | string\|nil | Snowflake ID of the parent category |
| `user_id` | string\|nil | Grant this user private View + Send permissions |
| `role_id` | string\|nil | Grant this role private View + Send permissions |
| `welcome_message` | string\|nil | Text sent immediately after the channel is created |
| `close_button` | boolean\|nil | If `true`, attaches a red "Close Ticket" button to the welcome message |

### `navi.delete_channel(channel_id)`

Permanently deletes a channel. Use with care — this is instant and irreversible.

```lua
navi.delete_channel(ctx.channel_id)
```

### `navi.create_thread(channel_id, name, options?)`

Creates a thread and returns its channel ID as a string, or `nil` on failure. **Blocks until complete** so you can use the returned ID immediately.

```lua
-- Standalone public thread
local thread_id = navi.create_thread(ctx.channel_id, "My Thread")

-- Thread attached to a specific message
local thread_id = navi.create_thread(ctx.channel_id, "Discussion", {
    message_id = ctx.message_id
})

-- Private thread that archives after an hour
local thread_id = navi.create_thread(ctx.channel_id, "Private Chat", {
    private      = true,
    auto_archive = 60
})

if thread_id then
    navi.say(thread_id, "Thread is open!")
end
```

| Option | Type | Default | Description |
|---|---|---|---|
| `message_id` | string\|nil | nil | If set, creates the thread attached to this message |
| `private` | boolean | false | Create a private thread (invite-only) |
| `auto_archive` | number | 1440 | Minutes until the thread auto-archives: `60`, `1440`, `4320`, `10080` |

---

## 19. Discord Cache

These functions return data from the bot's **cached** guild state. The cache is populated on startup and when you press `u` in the TUI to refresh. For the most up-to-date data, press `u` before relying on these.

### `navi.get_roles(guild_id?)`

Returns an array of role objects for the guild.

```lua
local roles = navi.get_roles(ctx.guild_id)
for _, role in ipairs(roles) do
    navi.log.info(role.id .. " — " .. role.name)
end
```

| Field | Type | Description |
|---|---|---|
| `role.id` | string | The role's snowflake ID |
| `role.name` | string | The role's display name |
| `role.color` | integer[] | RGB tuple: `{r, g, b}` |

### `navi.get_channels(guild_id?)`

Returns an array of text channel objects for the guild.

```lua
local channels = navi.get_channels(ctx.guild_id)
for _, ch in ipairs(channels) do
    navi.log.info(ch.id .. " — #" .. ch.name)
end
```

| Field | Type | Description |
|---|---|---|
| `ch.id` | string | The channel's snowflake ID |
| `ch.name` | string | The channel's display name |

---

## 20. HTTP Client

### `navi.http.get(url, headers?)`

Sends an HTTP GET request. **Blocks until complete.** Returns the response body as a string, or `nil` on error.

```lua
local body = navi.http.get("https://api.example.com/endpoint", nil)
if body then
    local data = navi.json.decode(body)
end
```

```lua
-- With custom headers
local body = navi.http.get("https://api.private.com/data", {
    ["Authorization"] = "Bearer " .. api_key,
    ["Accept"]        = "application/json"
})
```

### `navi.http.post(url, body, headers?)`

Sends an HTTP POST request. **Blocks until complete.** Returns the response body as a string, or `nil` on error.

```lua
local response = navi.http.post(
    "https://api.example.com/submit",
    navi.json.encode({ key = "value" }),
    { ["Content-Type"] = "application/json" }
)
```

> **Important:** Both `http.get` and `http.post` block the Lua thread while the request is in flight. For commands that make HTTP calls, use `ctx.defer()` first so Discord doesn't time out while waiting.

---

## 21. JSON

### `navi.json.encode(value)`

Serializes a Lua table or value to a JSON string.

```lua
local poll_data = { title = "Best pet?", options = { "Dog", "Cat" }, closed = false }
local json_str = navi.json.encode(poll_data)
navi.db.set("polls:data:1", json_str)
```

### `navi.json.decode(str)`

Parses a JSON string into a Lua value (table, number, string, boolean, or `nil`).

```lua
local json_str = navi.db.get("polls:data:1")
local poll = json_str and navi.json.decode(json_str)
if poll then
    navi.log.info("Poll title: " .. poll.title)
end
```

JSON and the database together are the standard pattern for storing **structured data** (arrays, nested tables) since `navi.db.set` only accepts a flat string.

---

## 22. Timed Intervals

### `navi.set_interval(callback, amount, unit?)`

Schedules a function to run repeatedly every `amount` `unit`s. Returns an interval ID. All active intervals are automatically cancelled when plugins are reloaded.

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
-- Send a message every 5 minutes
local my_interval = navi.set_interval(function()
    navi.say(ANNOUNCE_CHANNEL, "Reminder: read the rules!")
end, 5, "m")
```

### `navi.clear_interval(id)`

Cancels a running interval by the ID returned from `set_interval`. No-op if the ID doesn't exist.

```lua
navi.clear_interval(my_interval)
```

---

## 23. Permissions

The permissions system maps users and roles to a level hierarchy. Levels from lowest to highest: `"user"` → `"helper"` → `"moderator"` → `"admin"` → `"owner"`. Permissions are configured from the TUI.

### `navi.require_perm(ctx, level)`

Checks whether the user meets or exceeds the required level. If they **do not**, it automatically sends them an ephemeral denial message and returns `false`. If they **do**, it returns `true`.

**This is the standard way to gate admin-only commands.** The pattern is always:

```lua
if not navi.require_perm(ctx, "admin") then return end
```

```lua
navi.create_slash("delete_all_posts", "Nuke every post", {}, function(ctx)
    if not navi.require_perm(ctx, "moderator") then return end

    -- Only reaches here if the user is a moderator, admin, or owner
    -- ... do the dangerous thing ...
end)
```

### `navi.check_perm(ctx, level)`

Like `require_perm` but **silent** — it just returns `true` or `false` with no side effects. Use this when you want to adjust behaviour based on permissions without sending a denial.

```lua
if navi.check_perm(ctx, "moderator") then
    -- Show extra fields to mods
end
```

### `navi.get_perm_level(ctx)`

Returns the user's highest permission level as a string. Never returns `nil`; defaults to `"user"`.

```lua
local level = navi.get_perm_level(ctx)
ctx.reply("Your permission level is: " .. level)
```

---

## 24. Plugin Load Order

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

---

## 25. Bot Status / Presence

### `navi.set_status(activity_type, text)`

Changes the bot's Discord presence (the "Playing X" message shown in the member list).

- `activity_type`: `"playing"`, `"listening"`, `"watching"`, `"competing"`, `"custom"`, or `"none"`

```lua
navi.set_status("watching", "over the server")
navi.set_status("playing", "with fire")
navi.set_status("listening", "your complaints")
navi.set_status("none", "")  -- clears the status
```

---

## 26. Data Type Reference

### Snowflake IDs

Discord uses 64-bit integer "snowflake" IDs for users, channels, guilds, messages, roles, etc. In Lua 5.4 these are handled as integers, but **always use `tostring()` before passing them to `navi.db.set` or string concatenation** to be safe.

```lua
-- Safe pattern:
local uid = tostring(msg.author_id)
navi.db.set("xp:" .. uid, tostring(new_xp))
```

### Discord Mentions

In `description` or message text, you can mention users and roles with Discord's mention syntax. Discord renders these as clickable mentions.

```lua
-- Mention a user
"<@" .. user_id .. ">"

-- Mention a role
"<@&" .. role_id .. ">"

-- Mention a channel
"<#" .. channel_id .. ">"
```

---

## 27. Patterns, Tips & Common Mistakes

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

Message and user IDs come from the engine as numbers. Store them as strings by calling `tostring()` right away:

```lua
navi.register(function(msg)
    local uid = tostring(msg.author_id)  -- do this once, at the top
    local bal = tonumber(navi.db.get("balance:" .. uid)) or 0
    navi.db.set("balance:" .. uid, tostring(bal + 5))
end)
```

### Check `author_bot` in message listeners

If you respond to every message and your bot also sends messages, you'll get an infinite loop. Always guard against it:

```lua
navi.register(function(msg)
    if msg.author_bot then return end
    -- safe to process now
end)
```

### Use `ctx.defer()` for slow commands

Any slash command that makes an HTTP request, does a heavy database query, or takes more than ~2 seconds to respond **must** call `ctx.defer()` first. Discord will time out the interaction otherwise.

```lua
navi.create_slash("weather", "Get current weather", {
    { name = "city", description = "City name", type = "string", required = true }
}, function(ctx)
    ctx.defer()  -- tell Discord to wait

    local url = "https://api.weather.example.com/city/" .. ctx.args.city
    local body = navi.http.get(url, nil)
    local data = body and navi.json.decode(body)

    if data then
        ctx.followup("🌤️ " .. ctx.args.city .. ": " .. data.description .. ", " .. data.temp .. "°C")
    else
        ctx.followup("❌ Couldn't fetch weather data.", true)
    end
end)
```

### Use the event bus for cross-plugin data

Don't read another plugin's database keys directly when the event bus is available. The economy plugin exposes clean events:

```lua
-- Preferred: use the public API
navi.emit("economy:add",    { user_id = uid, amount = 100 })
navi.emit("economy:remove", { user_id = uid, amount = 50  })

-- Acceptable: read economy's balance (read-only, no side effects)
local balance = tonumber(navi.db.get("economy:balance:" .. uid)) or 0
```

### Store complex data as JSON

The database stores strings only. If you need to save a table (arrays, nested objects), encode it with `navi.json.encode` first:

```lua
local poll_data = {
    title      = "Best pet?",
    options    = { "Dog", "Cat", "Fish" },
    expires_at = os.time() + 3600,
    closed     = false
}
navi.db.set("polls:data:42", navi.json.encode(poll_data))

-- Reading back:
local raw = navi.db.get("polls:data:42")
local poll = raw and navi.json.decode(raw)
if poll and not poll.closed then
    navi.log.info("Poll is still open: " .. poll.title)
end
```

### Use local functions to avoid global namespace pollution

Declare all helper functions as `local`. If two plugins both define a non-local function with the same name, the second one silently overwrites the first.

```lua
-- Bad — pollutes globals, may conflict with other plugins
function get_balance(uid)
    return tonumber(navi.db.get("balance:" .. uid)) or 0
end

-- Good — scoped to this plugin file
local function get_balance(uid)
    return tonumber(navi.db.get("balance:" .. uid)) or 0
end
```

### Reloading and re-registration

When you press `r` in the TUI, every plugin file is re-executed from scratch. This means:

- All `navi.register`, `navi.create_slash`, `navi.register_component`, and `navi.register_modal` calls run again — this is expected and correct.
- Any **global variables** your plugin set in the previous load are still there. If your plugin does something like `INITIALIZED = true` and checks it, that check will be `true` on the second load. Be careful.
- Intervals are cancelled automatically before reload. You don't need to track and cancel them yourself.

### Mentioning vs looking up roles/channels from config

Config values of type `channel` and `role` store the snowflake ID as a string. You can use them directly in Discord mention syntax:

```lua
local role_id = navi.db.get("config:my_plugin:mod_role")

-- Mention it in a message
navi.say(channel_id, "Paging <@&" .. role_id .. ">!")

-- Use it to assign a role
navi.add_role(guild_id, user_id, role_id)
```

No extra lookup is needed. The ID is the ID.

### The `"config:"` prefix

When `navi.register_config` writes defaults to the database, it stores them under the key `config:plugin_name:key`. You **must** use this exact prefix when reading config values manually — `navi.db.get` auto-namespacing does **not** apply to `config:` keys (because the `config:` prefix already makes the key explicit with a colon).

```lua
-- Reading a config value (always use the full key)
local reward = tonumber(navi.db.get("config:economy:message_reward")) or 5
```
