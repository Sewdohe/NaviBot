# Commands & Interactions

## Message Listeners

### `navi.register(callback)`

Registers a function that is called every time any message is sent in a channel the bot can see. You can register multiple listeners from multiple plugins — they all fire independently.

```lua
navi.register(function(msg)
    if msg.author_bot then return end

    if msg.content:lower():find("good bot") then
        navi.react(tostring(msg.channel_id), tostring(msg.message_id), "❤️")
    end
end)
```

### Message Object (`msg`)

| Field | Type | Description |
| --- | --- | --- |
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

## Slash Commands

### `navi.create_slash(name, description, options, callback)`

Registers a slash command. After registering, slash commands are automatically synced to Discord when plugins reload.

- `name` (string): The command name. Lowercase, no spaces (e.g. `"balance"`).
- `description` (string): The help text shown in Discord's command picker.
- `options` (table): A list of argument definitions. Pass `{}` if the command takes no arguments.
- `callback` (function): Called when the command is used. Receives a `ctx` object.

```lua
navi.create_slash("ping", "Check if the bot is alive", {}, function(ctx)
    ctx.reply("Pong! 🏓")
end)
```

```lua
navi.create_slash("greet", "Greet a user", {
    { name = "user",   description = "Who to greet", type = "user",    required = true  },
    { name = "shout",  description = "Shout it?",    type = "boolean", required = false }
}, function(ctx)
    local target = ctx.args.user
    local shout  = ctx.args.shout
    local message = "Hey, <@" .. target .. ">!"
    if shout then message = string.upper(message) end
    ctx.reply(message)
end)
```

### Command Grouping

If a single plugin file registers **two or more** slash commands, Discord automatically groups them under the plugin's filename as a **parent command**. For example, a plugin named `casino.lua` that calls `create_slash` for `coinflip`, `slots`, and `dice` will appear in Discord as `/casino coinflip`, `/casino slots`, and `/casino dice`.

If a plugin only registers **one** command, it stays flat. This is fully automatic — you don't need to change any Lua code.

### Slash Option Fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `name` | string | Yes | Argument name (lowercase, no spaces) |
| `description` | string | Yes | Help text in Discord |
| `type` | string | Yes | See option types below |
| `required` | boolean | No | Whether the user must provide this arg. Default: `false` |
| `autocomplete` | function | No | If set, Discord calls this as the user types. See below. |

### Option Types

| Type | Lua value in `ctx.args` | Notes |
| --- | --- | --- |
| `"string"` | `string` | Plain text |
| `"integer"` | `number` | Whole number; safe to use directly in math |
| `"number"` | `number` | Float/decimal |
| `"boolean"` | `boolean` | Real `true`/`false` |
| `"user"` | `string` | The selected user's snowflake ID |
| `"channel"` | `string` | The selected channel's snowflake ID |
| `"role"` | `string` | The selected role's snowflake ID |

> **Note:** `user`, `channel`, and `role` options give you an ID string, not the full object. Use `navi.get_member` if you need more info about a user.

### Autocomplete

If an option has a function in its `autocomplete` field, Discord calls it as the user types. Your function receives a context table and must return up to 25 `{name, value}` pairs.

```lua
navi.create_slash("give_item", "Give an item to a user", {
    {
        name = "item",
        description = "The item name",
        type = "string",
        required = true,
        autocomplete = function(ctx)
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
| --- | --- | --- |
| `ctx.current_value` | string | What the user has typed so far (may be empty) |
| `ctx.user_id` | string | The user's snowflake ID |
| `ctx.guild_id` | string\|nil | The guild's snowflake ID |

## Slash Command Context

The `ctx` object passed to a slash command callback.

| Field / Method | Type | Description |
| --- | --- | --- |
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

### `ctx.reply` and `ctx.reply_embed`

```lua
ctx.reply("Done!")
ctx.reply("This is a secret.", true)  -- only you can see this

ctx.reply_embed({
    title = "Your Stats",
    color = 0x3498DB,
    fields = {
        { name = "Level", value = "12", inline = true },
        { name = "XP",    value = "840", inline = true }
    }
}, true)
```

> **One response per interaction.** Discord only allows one direct response per command invocation. If you want to send a public embed as well as a silent acknowledgement, use `navi.send_message` for the embed and `ctx.reply("...", true)` for the hidden confirmation.

### `ctx.defer()` and `ctx.followup()`

For commands that take more than ~3 seconds, you must **defer** first to prevent the "This application did not respond" error.

```lua
navi.create_slash("slow_command", "Fetches data from the web", {}, function(ctx)
    ctx.defer()

    local body = navi.http.get("https://api.example.com/data", nil)
    local data = body and navi.json.decode(body)

    if data then
        ctx.followup("Got it: " .. tostring(data.result))
    else
        ctx.followup("Request failed.", true)
    end
end)
```

`ctx.followup_embed(data, ephemeral?)` works the same way but sends an embed.

### Opening a Modal from a Command

Instead of replying, you can respond with a modal dialog (a popup form):

```lua
navi.create_slash("feedback", "Submit feedback", {}, function(ctx)
    ctx.modal("feedback_form", "Share Your Feedback", {
        { id = "subject", label = "Subject",       style = "short",     placeholder = "Brief topic",   required = true  },
        { id = "body",    label = "Your feedback", style = "paragraph", placeholder = "Tell us more…", required = true  },
        { id = "rating",  label = "Rating (1-10)", style = "short",     placeholder = "e.g. 8",        required = false },
    })
end)
```

> **Note:** A modal is a response to the interaction — you cannot also call `ctx.reply` in the same handler.

## Buttons and Select Menus

Buttons and select menus are attached to messages via the `components` field in an embed table. When clicked or selected, they fire a `register_component` handler.

### `navi.register_component(custom_id, callback)`

Registers a handler function that fires when a button with the matching `custom_id` is clicked, or when a select menu with the matching `id` has its value changed.

```lua
navi.send_message(channel_id, {
    description = "Click the button!",
    components = {
        { type = "button", id = "my_button", label = "Click Me", style = "primary" }
    }
})

navi.register_component("my_button", function(ctx)
    ctx.reply("You clicked it, " .. ctx.username .. "!", true)
end)
```

### Button Styles

| Style | Color | Use for |
| --- | --- | --- |
| `"primary"` | Blue | Main action |
| `"secondary"` | Grey | Secondary/neutral action |
| `"success"` | Green | Confirming or positive action |
| `"danger"` | Red | Destructive or risky action |
| `"link"` / `"url"` | Grey (opens URL) | External links — requires a `url` field instead of `id` |

Link buttons open a URL and do not need a `register_component` handler:

```lua
components = {
    { type = "button", style = "link", label = "Visit Website", url = "https://example.com" }
}
```

### Select Menus

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

## Component Context

The `ctx` object passed to a `register_component` handler.

| Field / Method | Type | Description |
| --- | --- | --- |
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

## Modal Dialogs

### `navi.register_modal(custom_id, callback)`

Registers a handler for when a user submits a modal form. The `custom_id` must match what was passed to `ctx.modal(...)`.

```lua
navi.register_modal("feedback_form", function(ctx)
    local subject = ctx.values.subject
    local body    = ctx.values.body
    local rating  = ctx.values.rating or "not given"

    navi.say(FEEDBACK_CHANNEL, "**" .. subject .. "**\n" .. body .. "\nRating: " .. rating)
    ctx.reply("Thanks for your feedback!", true)
end)
```

### Modal Field Definition

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `id` | string | Yes | Key used to read the value in `ctx.values` |
| `label` | string | Yes | The label shown above the input |
| `style` | `"short"` \| `"paragraph"` | No | Single-line vs multi-line input. Default: `"short"` |
| `placeholder` | string | No | Greyed-out hint text |
| `required` | boolean | No | Whether the user must fill this in. Default: `true` |

### Modal Context

| Field / Method | Type | Description |
| --- | --- | --- |
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
