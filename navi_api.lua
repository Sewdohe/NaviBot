---@meta

-------------------------------------------------------------------------------
-- 📦 NAVI ENGINE DATA TYPES
-------------------------------------------------------------------------------

---@class NaviAuthor
---@field id number|string @The user's ID
---@field name string @The user's display name
---@field bot boolean|nil @True if the user is a bot
---@field avatar string|nil @The user's avatar hash

---@class NaviMsg
---@field content string
---@field channel_id number|string
---@field id number|string @The message ID from Serenity
---@field author NaviAuthor @The nested Serenity user object
---@field mentions table[] @Array of mentioned users
---@field attachments table[] @Array of attachment objects

---@class NaviSlashCtx
---@field user_id string
---@field username string
---@field channel_id string
---@field guild_id string|nil
---@field options table<string, string|number|boolean> @The options passed to the slash command
---@field reply fun(message: string, ephemeral: boolean|nil) @Replies directly to the interaction

---@class NaviComponentCtx
---@field custom_id string @The ID of the clicked button or dropdown
---@field user_id string
---@field username string
---@field channel_id string
---@field guild_id string|nil
---@field values string[] @An array of selected values (only populated for dropdowns)
---@field reply fun(message: string, ephemeral: boolean|nil) @Replies directly to the interaction

---@class NaviReactionCtx
---@field user_id string|nil
---@field channel_id string
---@field message_id string
---@field guild_id string|nil
---@field emoji string

---@class NaviEmbedField
---@field name string
---@field value string
---@field inline boolean|nil

---@class NaviEmbed
---@field title string|nil
---@field description string|nil
---@field color number|nil @Hex color code (e.g., 0x3498DB)
---@field fields NaviEmbedField[]|nil
---@field components table[]|nil @Array of UI components (buttons, dropdowns)

---@class NaviChannelOptions
---@field category_id string|nil @Optional category to spawn the channel inside
---@field user_id string|nil @Optional user to grant private View/Send permissions
---@field role_id string|nil @Optional role to grant private View/Send permissions
---@field welcome_message string|nil @Optional text to send immediately upon creation
---@field close_button boolean|nil @If true, attaches a red 'Close Ticket' button to the welcome message

---@class NaviConfigItem
---@field key string @The database key (e.g., "channel_id")
---@field name string @The display name in the TUI
---@field description string @Help text shown in the TUI
---@field type "string"|"number"|"boolean"|"channel"|"role"|"category" @Tells the TUI how to render the input
---@field default string|number|boolean @Fallback value if not configured

-------------------------------------------------------------------------------
-- 🗄️ DATABASE API
-------------------------------------------------------------------------------

---@class NaviDB
---@field get fun(key: string): string|nil @Fetches a string from the SQLite database
---@field set fun(key: string, value: string) @Saves a string to the SQLite database
---@field get_list fun(key: string): string[] @Fetches a comma-separated string and returns a Lua table array

---@class NaviDBRow
---@field key string
---@field value string

---@class NaviDB
---@field get fun(key: string): string|nil
---@field set fun(key: string, value: string)
---@field query fun(sql: string): NaviDBRow[] @Executes raw SQL and returns an array of rows

-------------------------------------------------------------------------------
-- 🚀 THE GLOBAL NAVI API
-------------------------------------------------------------------------------

---@class NaviCore
---@field db NaviDB @The SQLite Database interface
---@field log fun(msg: string) @Sends a raw log string directly to the TUI
---@field register_config fun(plugin_name: string, schema: NaviConfigItem[]) @Registers TUI settings
---@field register fun(callback: fun(msg: NaviMsg)) @Registers a listener for all raw chat messages
---@field create_slash fun(name: string, description: string, options: table, callback: fun(ctx: NaviSlashCtx)) @Registers a Discord slash command
---@field send_message fun(channel_id: string|number, data: table) @Sends text or a rich embed to a channel
---@field say fun(channel_id: string|number, text: string) @Quickly sends plain text to a channel
---@field react fun(channel_id: string, message_id: string, emoji: string) @Adds a reaction to a specific message
---@field register_component fun(custom_id: string, callback: fun(ctx: NaviComponentCtx)) @Registers a button or dropdown click handler
---@field create_channel fun(guild_id: string, name: string, options: NaviChannelOptions) @Spawns a new text channel dynamically
---@field delete_channel fun(channel_id: string|number) @Deletes a Discord channel
---@field add_role fun(guild_id: string, user_id: string, role_id: string) @Assigns a role to a user
---@field remove_role fun(guild_id: string, user_id: string, role_id: string) @Removes a role from a user
---@field set_status fun(activity_type: "playing"|"listening"|"watching"|"competing"|"custom"|"none", text: string) @Changes the bot's Discord presence
---@field on fun(event_name: string, callback: fun(data: any)) @Listens for an event on the inter-plugin Event Bus
---@field emit fun(event_name: string, data: any) @Broadcasts an event across the inter-plugin Event Bus

---@type NaviCore
navi = {}

