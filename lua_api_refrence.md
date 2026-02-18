# Navi Bot Lua API Reference 🧚

This document details the complete Lua API available within the Navi Bot engine. All functions are accessed through the global `navi` object.

## 📚 Table of Contents
1.  [Core Events](#core-events)
2.  [Messaging](#messaging)
3.  [Database (Persistence)](#database)
4.  [Data Structures](#data-structures)

---

## 1. Core Events

### `navi.register(callback)`
Registers a function to be executed whenever a new message is received in a channel the bot can see.

* **Parameters:**
    * `callback` (function): A function that takes a single argument `msg` (see [Message Structure](#message-structure)).

* **Example:**
    ```lua
    navi.register(function(msg)
        if msg.content == "!ping" then
            navi.say(msg.channel_id, "Pong!")
        end
    end)
    ```

---

## 2. Messaging

### `navi.say(channel_id, text)`
Sends a standard plain-text message to the specified channel.

* **Parameters:**
    * `channel_id` (string|number): The Discord Channel ID.
    * `text` (string): The content of the message.

### `navi.send_embed(channel_id, embed_table)`
Sends a rich Discord embed to the specified channel.

* **Parameters:**
    * `channel_id` (string|number): The Discord Channel ID.
    * `embed_table` (table): A table defining the embed structure (see [Embed Structure](#embed-structure)).

* **Example:**
    ```lua
    navi.send_embed(msg.channel_id, {
        title = "Level Up!",
        description = "You reached level 5.",
        color = 0x00FF00,
        fields = {
            { name = "XP Gained", value = "500", inline = true }
        }
    })
    ```

---

## 3. Database

The database module allows you to persist data across bot restarts. It uses a simple Key-Value store backed by SQLite.

### `navi.db.set(key, value)`
Saves a value to the database. If the key already exists, it is overwritten.

* **Parameters:**
    * `key` (string): The unique identifier for this data (e.g., `"user_123_xp"`).
    * `value` (string): The data to save. **Note:** Currently only supports strings. Use `tostring()` for numbers.

### `navi.db.get(key)`
Retrieves a value from the database.

* **Parameters:**
    * `key` (string): The unique identifier to look up.
* **Returns:**
    * (string | nil): The saved value, or `nil` if the key does not exist.

* **Example:**
    ```lua
    -- Save
    navi.db.set("gold_user_123", "50")

    -- Load
    local gold = tonumber(navi.db.get("gold_user_123")) or 0
    ```

---

## 4. Data Structures

### <a name="message-structure"></a> `Message` Object
Passed to the `register` callback.

| Field | Type | Description |
| :--- | :--- | :--- |
| `msg.content` | `string` | The actual text content of the message. |
| `msg.message_id` | `string` | The unique ID of the message itself. |
| `msg.channel_id` | `string` | The ID of the channel where the message was sent. |
| `msg.author` | `string` | The username of the sender. |
| `msg.author_id` | `string` | The unique user ID of the sender. |
| `msg.author_avatar` | `string` | URL to the sender's avatar (or default avatar). |
| `msg.mentions` | `table` | List of `User` objects mentioned in the message. |
| `msg.attachments` | `table` | List of URL strings for any attached files. |

### <a name="embed-structure"></a> `Embed` Table
Used in `navi.send_embed`.

| Field | Type | Description |
| :--- | :--- | :--- |
| `title` | `string` | (Optional) The bold title at the top. |
| `description` | `string` | (Optional) The main body text. |
| `color` | `number` | (Optional) Hex color code (e.g., `0xFF0000`). |
| `url` | `string` | (Optional) Makes the title a clickable link. |
| `image` | `string` | (Optional) URL of a large image at the bottom. |
| `thumbnail` | `string` | (Optional) URL of a small image at the top-right. |
| `footer` | `table` | (Optional) `{ text = "...", icon_url = "..." }` |
| `fields` | `table` | (Optional) List of field objects. |

#### Embed Field Object
| Field | Type | Description |
| :--- | :--- | :--- |
| `name` | `string` | The field title. |
| `value` | `string` | The field text. |
| `inline` | `boolean` | (Optional) If true, fields display side-by-side. |

### `User` Object
Found inside the `msg.mentions` list.

| Field | Type | Description |
| :--- | :--- | :--- |
| `user.name` | `string` | The username. |
| `user.id` | `string` | The unique user ID. |
| `user.avatar` | `string` | URL to the user's avatar. |

---

## 5. Slash Command Registration

### `navi.create_slash(name, description, options, callback)`
Registers a slash command with Discord. These commands appear in the `/` menu.

* **Parameters:**
    * `name` (string): The command name (lowercase, no spaces).
    * `description` (string): The help text shown in Discord.
    * `options` (table): A list of arguments the command accepts (see [Option Structure](#option-structure)). **Pass `{}` if no arguments are needed.**
    * `callback` (function): The function to run. Receives a `ctx` object.

* **Example (No Arguments):**
    ```lua
    navi.create_slash("hello", "Says hi", {}, function(ctx)
        ctx.reply("Hello " .. ctx.username)
    end)
    ```

* **Example (With Arguments):**
    ```lua
    navi.create_slash("greet", "Greets a user", {
        { name = "user", description = "Who to greet", type = "user", required = true },
        { name = "shout", description = "Yell it?", type = "boolean", required = false }
    }, function(ctx)
        local target = ctx.args.user -- Returns User ID
        local shout = ctx.args.shout -- Returns "true" or "false" string
        
        ctx.reply("Target ID: " .. target)
    end)
    ```

#### <a name="option-structure"></a> Option Structure
Each option in the list must be a table with these fields:
| Field | Type | Description |
| :--- | :--- | :--- |
| `name` | `string` | The argument name (e.g. "amount"). |
| `description` | `string` | Help text for this argument. |
| `type` | `string` | One of: `"string"`, `"integer"`, `"boolean"`, `"user"`, `"channel"`, `"role"`. |
| `required` | `boolean` | (Optional) Is this argument mandatory? Default: `false`. |

---

### <a name="slash-context"></a> Slash `Context` Object
Passed to the `create_slash` callback.

| Field | Type | Description |
| :--- | :--- | :--- |
| `ctx.user_id` | `string` | The ID of the user who ran the command. |
| `ctx.username` | `string` | The username of the command runner. |
| `ctx.args` | `table` | **NEW:** A Key-Value table of arguments provided by the user. |
| `ctx.reply(msg)` | `function` | Sends a response message to the channel. |

#### Argument Type Mapping
The values in `ctx.args` now preserve their native types from Discord:

| Discord Option Type | Lua Type | Notes |
| :--- | :--- | :--- |
| `STRING` | `string` | Standard text. |
| `INTEGER` | `number` | A whole number (e.g. `42`). |
| `NUMBER` | `number` | A float/decimal (e.g. `3.14`). |
| `BOOLEAN` | `boolean` | Real `true` / `false`. |
| `USER` / `ROLE` / `CHANNEL` | `string` | **IDs are strings.** We keep them as strings to prevent precision loss with large IDs. |

* **Example Usage:**
    ```lua
    local amount = ctx.args.amount -- This is a number! You can do math immediately.
    local is_vip = ctx.args.vip    -- This is a boolean! You can do 'if is_vip then'.
    ```

## 🐛 Troubleshooting

* **Nil Errors:** Always check if a value exists before using it. `navi.db.get` returns `nil` if the key is missing. Use `or` to provide defaults:
    `local xp = navi.db.get("xp") or "0"`
* **Types:** The database stores everything as strings. Remember to use `tonumber()` when reading math values and `tostring()` when saving them.
* **Reloading:** Use the `!reload` command in Discord to apply changes to your Lua scripts instantly.