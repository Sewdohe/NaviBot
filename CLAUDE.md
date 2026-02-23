# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Run the bot (requires .env with DISCORD_TOKEN)
cargo run

# Build release binary
cargo build --release

# Check for compile errors without building
cargo check
```

There is no test suite. The `.env` file must contain `DISCORD_TOKEN=your_token_here`.

## Architecture

NaviBot is a **Rust core + hot-reloadable Lua plugin system** for Discord. The Rust binary handles all Discord API I/O, database access, and async runtime. All bot features live in Lua plugins under `plugins/`.

### Key Source Files

| File | Purpose |
|---|---|
| `src/main.rs` | Entry point; spawns bot thread + TUI thread; sets up inter-thread channels |
| `src/engine.rs` | Loads Lua runtime, executes core files and plugins, binds all `navi.*` Rust functions |
| `src/events.rs` | Discord event handlers; bridges serenity events into Lua callbacks |
| `src/types.rs` | Shared types: `Data`, `BotEvent`, `AdminCommand`, `ConfigRegistry` |
| `src/commands.rs` | Prefix commands (`!reload`, `!sync`) |
| `src/tui.rs` | Ratatui TUI dashboard; settings editor, log viewer, key bindings |
| `core/events.lua` | Inter-plugin event bus (`navi.on`, `navi.emit`) |
| `core/components.lua` | Discord UI component (button/select) handler registration |
| `navi_api.lua` | EmmyLua `@meta` annotations for IDE autocomplete — **not loaded at runtime** |

### Thread Model

Two threads communicate via unbounded channels:

- **Bot thread** (tokio): runs the Discord client, Lua engine, and admin command loop
- **TUI thread** (main): renders the terminal dashboard, sends `AdminCommand` to bot thread

`AdminCommand` enum drives hot-reload (`Reload`), graceful shutdown (`Shutdown`), config persistence (`SaveConfig`), and Discord cache refresh (`RefreshCache`).

### Plugin System

Plugins are `.lua` files in `plugins/`. The engine loads them alphabetically after executing `core/events.lua` and `core/components.lua`. All `navi.*` APIs are injected into the Lua global before any plugin runs.

**Standard plugin structure:**

```lua
-- 1. Register TUI-editable config (persisted in SQLite)
navi.register_config("plugin_name", {
    { key = "some_key", name = "Display Name", type = "string", default = "value" }
})

-- 2. Declare slash commands
navi.create_slash("command", "Description", {options...}, function(ctx)
    local val = navi.db.get("some_key")  -- auto-namespaced to "plugin_name:some_key"
    ctx.reply("response", true)          -- true = ephemeral
end)

-- 3. Listen to message events
navi.register(function(msg)
    if msg.author_bot then return end
    -- msg.content, msg.channel_id, msg.author_id, msg.guild_id
end)

-- 4. Inter-plugin event bus
navi.on("plugin:event_name", function(data) ... end)
navi.emit("plugin:event_name", { key = value })
```

### Database

SQLite (`navi.db`) is a single key-value table (`kv_store`). The Lua `navi.db.get/set` wrappers **automatically prepend the calling plugin's filename** as a namespace (e.g., a `get("balance")` call from `economy.lua` reads key `economy:balance`). Use `navi.db.query(sql)` for raw SQL when needed (e.g., leaderboards with `ORDER BY`).

### Navi API Surface (Lua globals)

- `navi.create_slash(name, desc, options, fn)` — register a slash command
- `navi.register(fn)` — register a message listener
- `navi.on(event, fn)` / `navi.emit(event, data)` — inter-plugin event bus
- `navi.db.get(key)` / `navi.db.set(key, val)` / `navi.db.query(sql)` — database
- `navi.say(channel_id, text)` — send plain text message
- `navi.send_message(channel_id, embed_table)` — send embed
- `navi.add_role(guild_id, user_id, role_id)` / `navi.remove_role(...)` — role management
- `navi.register_config(plugin, schema)` — declare TUI-configurable settings
- `navi.get_roles(guild_id)` / `navi.get_channels(guild_id)` — Discord cache lookups

### TUI Key Bindings

| Key | Action |
|---|---|
| `q` | Graceful shutdown |
| `r` | Hot-reload all Lua plugins |
| `c` | Open settings/config dashboard |
| `l` | Return to live logs |
| `i` | Open terminal input |
| `u` | Refresh Discord roles/channels cache |
| `Up/Down/Enter` | Navigate and edit configs |

### Adding a New Plugin

1. Create `plugins/yourplugin.lua`
2. Use `navi.register_config`, `navi.create_slash`, `navi.register`, and/or `navi.on` as needed
3. Press `r` in the TUI to hot-reload — no Rust recompilation required
4. Run `!sync` (prefix command) in Discord to register any new slash commands with the API

### IDE Autocomplete

Place `navi_api.lua` in a non-`plugins/` directory (e.g., `.types/`). LuaLS/EmmyLua will pick up the `@meta` annotations and provide autocomplete for all `navi.*` APIs and context types (`NaviSlashCtx`, `NaviMsg`, `NaviComponentCtx`, etc.).
