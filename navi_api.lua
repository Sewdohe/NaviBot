---@meta

-------------------------------------------------------------------------------
-- 📦 NAVI ENGINE DATA TYPES
-------------------------------------------------------------------------------

--- A user mentioned inside a message.
---@class NaviMentionedUser
---@field id number @The mentioned user's snowflake ID
---@field name string @The mentioned user's username
---@field avatar string @The mentioned user's avatar URL

--- The table passed to every `navi.register` / `navi.on("message", …)` listener.
---@class NaviMsg
---@field content string @The raw text of the message
---@field channel_id number @The channel's snowflake ID
---@field message_id number @The message's snowflake ID
---@field author string @The sender's username
---@field author_id number @The sender's snowflake ID
---@field author_avatar string @The sender's avatar URL
---@field mentions NaviMentionedUser[] @Array of users mentioned in the message
---@field attachments string[] @Array of attachment URLs

--- Context passed to a slash command callback registered with `navi.create_slash`.
---@class NaviSlashCtx
---@field user_id number @The invoking user's snowflake ID (integer)
---@field username string @The invoking user's username
---@field channel_id string @The channel's snowflake ID (string)
---@field guild_id string|nil @The guild's snowflake ID, or nil in DMs
---@field args table<string, string|number|boolean> @Named options passed to the command
---@field reply fun(message: string) @Sends a reply to the slash interaction

--- Context passed to a component (button / select menu) handler.
---@class NaviComponentCtx
---@field custom_id string @The `id` field set when the button or select was created
---@field user_id string @The clicking user's snowflake ID (string)
---@field username string @The clicking user's username
---@field channel_id string @The channel's snowflake ID
---@field guild_id string|nil @The guild's snowflake ID, or nil in DMs
---@field values string[] @Selected values (non-empty only for string-select menus)
---@field reply fun(message: string, ephemeral: boolean) @Sends a reply to the interaction

--- Context passed to `on_reaction_add` and `on_reaction_remove` global callbacks.
---@class NaviReactionCtx
---@field user_id string|nil @The reacting user's snowflake ID, or nil if unknown
---@field channel_id string @The channel's snowflake ID
---@field message_id string @The message's snowflake ID
---@field guild_id string|nil @The guild's snowflake ID, or nil in DMs
---@field emoji string @Unicode emoji or custom emoji string (e.g. `<:name:id>`)

--- Data table emitted on the `"member_join"` event bus event.
---@class NaviMemberJoinData
---@field user_id string @The new member's snowflake ID
---@field username string @The new member's username
---@field guild_id string @The guild's snowflake ID

-------------------------------------------------------------------------------
-- 🧩 UI COMPONENT TYPES  (used inside `NaviEmbed.components`)
-------------------------------------------------------------------------------

--- A clickable button. Up to 5 buttons share one action row automatically.
---@class NaviButton
---@field type "button"
---@field label string @Text displayed on the button
---@field id string @The `custom_id` sent to `navi.register_component` when clicked
---@field style "primary"|"secondary"|"success"|"danger"|nil @Defaults to `"primary"`

--- A link button that opens a URL instead of triggering an interaction.
---@class NaviLinkButton
---@field type "button"
---@field label string @Text displayed on the button
---@field style "link"|"url"
---@field url string @The URL to open when clicked

--- A single option inside a string-select menu.
---@class NaviSelectOption
---@field label string @Text shown in the dropdown
---@field value string|nil @Value sent to the handler; defaults to `label`
---@field description string|nil @Optional subtitle shown below the label
---@field emoji string|nil @Optional Unicode emoji shown before the label

--- A string-select (dropdown) menu component.
---@class NaviSelectMenu
---@field type "select"
---@field id string @The `custom_id` sent to `navi.register_component` on selection
---@field placeholder string|nil @Greyed-out hint text when nothing is selected
---@field options NaviSelectOption[] @The list of choices

--- A single field entry inside an embed.
---@class NaviEmbedField
---@field name string @Bold field title
---@field value string @Field body text
---@field inline boolean|nil @Whether this field sits side-by-side with the next one

--- The data table accepted by `navi.send_message`.
---@class NaviEmbed
---@field title string|nil @Embed title
---@field description string|nil @Embed body text
---@field color number|nil @Hex color integer (e.g. `0x3498DB`)
---@field image string|nil @URL of an image to display at the bottom of the embed
---@field fields NaviEmbedField[]|nil @Array of embed field objects
---@field components (NaviButton|NaviLinkButton|NaviSelectMenu)[]|nil @Buttons and/or select menus to attach

--- Options accepted by `navi.create_channel`.
---@class NaviChannelOptions
---@field category_id string|nil @Snowflake ID of the category to place the channel in
---@field user_id string|nil @Grant this user private View/Send permissions
---@field role_id string|nil @Grant this role private View/Send permissions
---@field welcome_message string|nil @Text to send immediately after the channel is created
---@field close_button boolean|nil @Attach a red "Close Ticket" button to the welcome message

--- Schema for one sub-field within a list config item.
---@class NaviListItemSchema
---@field key string @Key used inside each item table
---@field name string @Human-readable label shown in the TUI
---@field type "string"|"number"|"boolean"|"channel"|"role"|"category" @Controls the sub-field input widget

--- A single entry in a plugin's configuration schema.
---@class NaviConfigItem
---@field key string @Database key used to store the value (e.g. `"log_channel"`)
---@field name string @Human-readable label shown in the TUI
---@field description string @Help text shown in the TUI
---@field type "string"|"number"|"boolean"|"channel"|"role"|"category"|"list" @Controls the TUI input widget
---@field default string|number|boolean|nil @Value written to the DB if the user has not configured it yet; omit for list fields
---@field item_schema NaviListItemSchema[]|nil @Required when type = "list"; defines the sub-fields of each item

--- A Discord role from the cached guild state.
---@class NaviRole
---@field id string @The role's snowflake ID
---@field name string @The role's display name
---@field color integer[] @RGB tuple: {r, g, b}

--- A Discord text channel from the cached guild state.
---@class NaviChannel
---@field id string @The channel's snowflake ID
---@field name string @The channel's display name

-------------------------------------------------------------------------------
-- 🗄️ DATABASE API
-------------------------------------------------------------------------------

--- A single row returned by `navi.db.query`. Column 0 maps to `key`, column 1 to `value`.
---@class NaviDBRow
---@field key string
---@field value string

---@class NaviDB
---@field get fun(key: string): string|nil @Reads a value; the key is auto-namespaced to the calling plugin
---@field set fun(key: string, value: string|number|boolean) @Writes a value; the key is auto-namespaced to the calling plugin
---@field query fun(sql: string): NaviDBRow[] @Executes raw SQL and returns an array of `{key, value}` rows
---@field get_list fun(key: string): table[] @Returns all items of a list config field as an array of tables

-------------------------------------------------------------------------------
-- 🚀 THE GLOBAL NAVI API
-------------------------------------------------------------------------------

---@class NaviLogger
---@field info fun(msg: string) @Sends an INFO-level message to the TUI log pane
---@field warn fun(msg: string) @Sends a WARN-level message (yellow) to the TUI log pane
---@field error fun(msg: string) @Sends an ERROR-level message (red) to the TUI log pane

--- HTTP client for making outbound requests.
---@class NaviHTTP
---@field get fun(url: string, headers: table<string,string>|nil): string|nil @Sends a GET request; returns body string or nil on error
---@field post fun(url: string, body: string, headers: table<string,string>|nil): string|nil @Sends a POST request; returns body string or nil on error

--- JSON encoder/decoder.
---@class NaviJSON
---@field decode fun(str: string): any @Parses a JSON string into a Lua table/value
---@field encode fun(val: any): string @Serializes a Lua table/value into a JSON string

---@class NaviCore
---@field db NaviDB @The SQLite key-value database interface
---@field log NaviLogger @Structured logger; use .info(), .warn(), .error()
---@field http NaviHTTP @Outbound HTTP client; blocks until the request completes
---@field json NaviJSON @JSON encode/decode utilities
---@field register fun(callback: fun(msg: NaviMsg)) @Registers a listener for every incoming chat message
---@field register_component fun(custom_id: string, callback: fun(ctx: NaviComponentCtx)) @Registers a handler for a button or select-menu interaction
---@field register_config fun(plugin_name: string, schema: NaviConfigItem[]) @Declares TUI-editable settings for a plugin; defaults are persisted to the DB automatically
---@field create_slash fun(name: string, description: string, options: table, callback: fun(ctx: NaviSlashCtx)) @Declares a slash command (run `!sync` to push it to Discord)
---@field on fun(event_name: string, callback: fun(data: any)) @Subscribes to an inter-plugin event bus event
---@field emit fun(event_name: string, data: any) @Publishes an event to all subscribers on the inter-plugin event bus
---@field say fun(channel_id: string|number, text: string) @Sends plain text to a channel
---@field send_message fun(channel_id: string|number, data: NaviEmbed) @Sends an embed (with optional buttons/selects) to a channel
---@field react fun(channel_id: string, message_id: string, emoji: string) @Adds a reaction emoji to a message
---@field create_channel fun(guild_id: string, name: string, options: NaviChannelOptions) @Creates a new text channel, optionally private and with a welcome message
---@field delete_channel fun(channel_id: string|number) @Permanently deletes a channel
---@field add_role fun(guild_id: string, user_id: string, role_id: string) @Assigns a role to a member
---@field remove_role fun(guild_id: string, user_id: string, role_id: string) @Removes a role from a member
---@field set_status fun(activity_type: "playing"|"listening"|"watching"|"competing"|"custom"|"none", text: string) @Changes the bot's Discord presence
---@field get_roles fun(guild_id: string|nil): NaviRole[] @Returns cached roles (call navi.refresh_cache or press u first)
---@field get_channels fun(guild_id: string|nil): NaviChannel[] @Returns cached text channels

---@type NaviCore
---@diagnostic disable-next-line: missing-fields
navi = {}

-------------------------------------------------------------------------------
-- 🌐 GLOBAL CALLBACKS  (define these in your plugin to handle Discord events)
-------------------------------------------------------------------------------

--- Called by the engine whenever a reaction is added to any message.
--- Define this function in your plugin to handle reaction-add events.
---@type fun(ctx: NaviReactionCtx)
on_reaction_add = nil

--- Called by the engine whenever a reaction is removed from any message.
--- Define this function in your plugin to handle reaction-remove events.
---@type fun(ctx: NaviReactionCtx)
on_reaction_remove = nil
