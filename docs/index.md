# Navi Bot — Lua Plugin API Reference

Complete guide for writing Lua plugins for the Navi Bot engine. Covers every API function, every data structure, and the patterns used in the real plugins that ship with the bot.

---

## Sections

- [Getting Started](getting-started.md) — How plugins work, plugin anatomy, load order, logging
- [Configuration](configuration.md) — `register_config`, config types, lists, enums, reading values
- [Database](database.md) — Key-value store, namespacing, get/set/query
- [Commands & Interactions](commands.md) — Slash commands, message listeners, buttons, select menus, modals
- [Messaging & Discord API](messaging.md) — Sending messages, embeds, member/role/channel management, bot status
- [Events & Timers](events.md) — Inter-plugin event bus, Discord event callbacks, timed intervals
- [Utilities](utilities.md) — HTTP client, JSON, permissions
- [Reference](reference.md) — Data types, common patterns, and pitfalls to avoid
