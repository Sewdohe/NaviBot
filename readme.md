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