navi.register_config("Greeter", {
    { key = "channel_id", name = "Welcome Channel", description = "The ID of the channel to send welcomes in", type = "channel", default = "" },
    { key = "enabled", name = "Plugin Enabled", description = "Toggle the welcome messages on or off", type = "boolean", default = true }
})