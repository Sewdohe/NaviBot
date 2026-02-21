# 🤖 Navi Engine

![[navibot-preview.png]]

Navi Engine is a high-performance, enterprise-grade Discord bot framework. It pairs a lightning-fast Rust core with a hot-reloadable Lua plugin system and a built-in Terminal UI (TUI) dashboard. 

With Navi, you never have to recompile your Rust binary to add new bot features. Simply write a Lua script, drop it in the `plugins/` directory, and hit reload.

---

## ✨ Core Features

* **⚡ Rust + Lua Architecture:** Uses `poise`/`serenity` for rock-solid Discord API interactions, and `mlua` to execute Lua scripts at blazing speeds.
* **🖥️ Interactive TUI Dashboard:** A built-in terminal interface using `ratatui`. Features real-time color-coded logs, auto-scrolling, and a dynamic configuration menu with interactive dropdowns.
* **🔥 Hot-Reloading:** Press `r` in the TUI to instantly reload all Lua plugins without disconnecting the bot.
* **🗄️ Smart SQLite Database:** Includes a built-in, thread-safe SQLite key-value store (`navi.db`). Database keys are automatically sandboxed and namespaced to the specific plugin requesting them to prevent data collisions.
* **📡 Decoupled Event Bus:** Plugins communicate via an inter-plugin event bus (`navi.on` and `navi.emit`), meaning plugins never directly depend on each other.
* **🎛️ Native UI Components:** Full support for spawning and handling Discord buttons and select menus directly from Lua.

---

## 🚀 Getting Started

### Prerequisites
* Rust and Cargo installed.
* A Discord Bot Token.

### Installation
1. Clone the repository.
2. Create a `.env` file in the root directory and add your bot token: `DISCORD_TOKEN=your_token_here`
3. Run the engine:
```bash
cargo run
```
4. The TUI will launch, and the engine will automatically execute any `.lua` files found in the `plugins/` directory.

---

## 📂 Writing Plugins

Every feature in Navi is a self-contained plugin. A plugin is simply a `.lua` file placed inside the `plugins/` folder. The engine reads these alphabetically and bakes them into the core at runtime.

Here is an example of a simple auto-responder plugin:

```lua
print("--- Loading Greeter Plugin ---")

-- 1. Register Configuration for the TUI Dashboard
navi.register_config("greeter", {
    { key = "welcome_message", name = "Welcome Message", description = "What the bot says", type = "string", default = "Hello there!" },
    { key = "log_channel", name = "Log Channel", description = "Where to send the logs", type = "channel", default = "" }
})

-- 2. Create a Slash Command
navi.create_slash("hello", "Says hello to the user", {}, 
---@param ctx NaviSlashCtx
function(ctx)
    local text = navi.db.get("welcome_message") -- Auto-namespaced to 'greeter:welcome_message'!
    ctx.reply(text)
end)

-- 3. Listen to Discord Events
navi.register(function(msg)
    if msg.author_bot then return end
    
    if msg.content == "ping" then
        navi.say(msg.channel_id, "pong!")
    end
end)
```

---

## 🛠️ Workspace & IDE Setup (Autocomplete)

To get full autocomplete, hover documentation, and type-checking in your editor (especially for Neovim/LuaLS or VSCode), you need the engine's EmmyLua annotations.

1. Grab the `navi_api.lua` file (contains all `---@meta` tags for the engine).
2. Place it anywhere in your project workspace **EXCEPT** the `plugins/` folder (e.g., in a `.types/` or `docs/` folder).
3. Your LSP will automatically read the global `navi` object and provide perfect autocomplete for things like `ctx.reply`, `navi.send_message`, and UI component tables!

---

## ⌨️ TUI Controls

* `q` - Shutdown the engine safely.
* `r` - Reload all Lua plugins.
* `c` - Open the Settings & Configuration Dashboard.
* `l` - Return to the Live Logs.
* `i` - Open the terminal input buffer.
* `Up/Down/Enter` - Navigate and edit plugin configurations.

## 📜 Documentation

[Click here to view the Wiki Page](https://www.divnectar.com/docs/navibot-plugin-development-guide/)