# Navi Bot Engine 🧚

**Navi** is a high-performance, modular Discord bot engine built in **Rust**. It features a hot-reloadable **Lua** plugin system, allowing you to write, update, and fix bot logic instantly without restarting the core process.

It combines the safety and concurrency of Rust with the ease of use of Lua.

## 🚀 Features

* **⚡ Rust Core:** Built on `poise` and `serenity` for maximum performance and stability.
* **🧠 Lua Scripting:** Write plugins in standard Lua 5.4.
* **🔥 Hot Reloading:** Update commands on the fly with `!reload`.
* **💾 Integrated Database:** Zero-config persistence using SQLite (`navi.db`).
* **🔌 Event Bus:** Multiple plugins can listen to chat events simultaneously.
* **🎨 Rich Embeds:** Full support for Discord embeds, images, and avatars via Lua tables.

---

## 🛠️ Installation & Setup

### Prerequisites
* Rust (Cargo) installed.
* A Discord Bot Token.

### Quick Start
1.  **Clone the repo:**
    ```bash
    git clone [https://github.com/yourname/navi_bot](https://github.com/yourname/navi_bot)
    cd navi_bot
    ```

2.  **Environment Setup:**
    Create a `.env` file in the root directory:
    ```env
    DISCORD_TOKEN=your_token_here_do_not_share
    ```

3.  **Run the Engine:**
    ```bash
    cargo run
    ```
    *The bot will automatically create `navi.db` and a `/plugins` folder if they don't exist.*

---

## 🧩 Writing Plugins

Plugins live in the `/plugins` directory. You can create as many `.lua` files as you want.

### The "Hello World" Plugin
Create `plugins/hello.lua`:

```lua
navi.register(function(msg)
    if msg.content == "!ping" then
        navi.say(msg.channel_id, "Pong! 🏓")
    end
end)

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

## 🐛 Troubleshooting

* **Nil Errors:** Always check if a value exists before using it. `navi.db.get` returns `nil` if the key is missing. Use `or` to provide defaults:
    `local xp = navi.db.get("xp") or "0"`
* **Types:** The database stores everything as strings. Remember to use `tonumber()` when reading math values and `tostring()` when saving them.
* **Reloading:** Use the `!reload` command in Discord to apply changes to your Lua scripts instantly.