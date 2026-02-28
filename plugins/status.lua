print("--- Loading Status Manager ---")

-- 1. Register the settings in the TUI
navi.register_config("bot_status", {
    { key = "activity_type", name = "Activity Type", description = "playing, listening, watching, competing, or none", type = "enum", options = {"playing", "watching", "competing", "none" }, default = "playing" },
    { key = "status_text", name = "Status Text", description = "What is the bot doing?", type = "string", default = "with my TUI Dashboard" }
})

-- 2. Fetch from DB and set it immediately
local act_type = navi.db.get("config:bot_status:activity_type") or "playing"
local text = navi.db.get("config:bot_status:status_text") or "with my TUI Dashboard"

navi.set_status(act_type, text)