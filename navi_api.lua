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
---@field guild_id string|nil @The guild's snowflake ID, or nil in DMs

--- Context passed to a slash command callback registered with `navi.create_slash`.
---@class NaviSlashCtx
---@field user_id number @The invoking user's snowflake ID (integer)
---@field username string @The invoking user's username
---@field channel_id string @The channel's snowflake ID (string)
---@field guild_id string|nil @The guild's snowflake ID, or nil in DMs
---@field member_roles string[] @Role IDs of the invoking member. Empty array in DMs.
---@field args table<string, string|number|boolean> @Named options passed to the command
---@field reply fun(message: string, ephemeral: boolean|nil) @Sends a plain-text reply. Pass true as the second argument for an ephemeral (only-you-can-see) response.
---@field reply_embed fun(data: NaviEmbed, ephemeral: boolean|nil) @Sends an embed reply. Supports the same fields as NaviEmbed (title, description, color, fields, image, components). Pass true as the second argument for ephemeral.
---@field defer fun(ephemeral: boolean|nil) @Acknowledges the interaction immediately (shows "App is thinking…"). Call this for commands that take more than 3 seconds, then respond with ctx.followup() or ctx.followup_embed(). Blocks until Discord confirms the ACK.
---@field followup fun(message: string, ephemeral: boolean|nil) @Sends a follow-up message after ctx.defer(). Can be called multiple times.
---@field followup_embed fun(data: NaviEmbed, ephemeral: boolean|nil) @Sends an embed as a follow-up message after ctx.defer().
---@field modal fun(custom_id: string, title: string, fields: {id:string, label:string, style:"short"|"paragraph"|nil, placeholder:string|nil, required:boolean|nil}[]) @Responds with a modal dialog instead of a message. Cannot be combined with reply.

--- Context passed to a component (button / select menu) handler.
---@class NaviComponentCtx
---@field custom_id string @The `id` field set when the button or select was created
---@field user_id string @The clicking user's snowflake ID (string)
---@field username string @The clicking user's username
---@field channel_id string @The channel's snowflake ID
---@field guild_id string|nil @The guild's snowflake ID, or nil in DMs
---@field member_roles string[] @Role IDs of the clicking member. Empty array in DMs.
---@field values string[] @Selected values (non-empty only for string-select menus)
---@field reply fun(message: string, ephemeral: boolean) @Sends a reply to the interaction
---@field reply_embed fun(data: NaviEmbed, ephemeral: boolean|nil) @Sends an embed reply to the interaction
---@field modal fun(custom_id: string, title: string, fields: {id:string, label:string, style:"short"|"paragraph"|nil, placeholder:string|nil, required:boolean|nil}[]) @Responds with a modal dialog instead of a message. Cannot be combined with reply.

--- Context passed to a modal submit handler registered with `navi.register_modal`.
---@class NaviModalCtx
---@field custom_id string @The modal's custom_id (set when it was created)
---@field user_id string @The submitting user's snowflake ID
---@field username string @The submitting user's username
---@field channel_id string @Channel snowflake ID
---@field guild_id string|nil @Guild snowflake ID, nil in DMs
---@field member_roles string[] @Role IDs of the submitting member. Empty array in DMs.
---@field values table<string, string> @Map of field custom_id → submitted value
---@field reply fun(message: string, ephemeral: boolean) @Sends a reply to the modal submission
---@field reply_embed fun(data: NaviEmbed, ephemeral: boolean|nil) @Sends an embed reply to the modal submission

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

--- Data table passed to `on_member_leave`.
---@class NaviMemberLeaveData
---@field user_id string @The leaving member's snowflake ID
---@field username string @The leaving member's username
---@field guild_id string @The guild's snowflake ID

--- Data table passed to `on_message_edit`.
---@class NaviMessageEditData
---@field message_id string @The edited message's snowflake ID
---@field channel_id string @The channel's snowflake ID
---@field guild_id string|nil @The guild's snowflake ID, or nil in DMs
---@field new_content string|nil @The updated content (nil if Discord did not include it)

--- Data table passed to `on_message_delete`.
---@class NaviMessageDeleteData
---@field message_id string @The deleted message's snowflake ID
---@field channel_id string @The channel's snowflake ID
---@field guild_id string|nil @The guild's snowflake ID, or nil in DMs

--- Data table passed to `on_voice_state_update`.
---@class NaviVoiceStateData
---@field user_id string @The user's snowflake ID
---@field guild_id string|nil @The guild's snowflake ID
---@field channel_id string|nil @The channel they joined, or nil if they disconnected
---@field self_mute boolean @Whether the user has muted themselves
---@field self_deaf boolean @Whether the user has deafened themselves
---@field self_stream boolean @Whether the user is streaming (Go Live)
---@field self_video boolean @Whether the user has their camera on

--- Member info returned by `navi.get_member`.
---@class NaviMember
---@field user_id string @The member's snowflake ID
---@field username string @The member's username
---@field display_name string @Nickname if set, otherwise username
---@field nickname string|nil @Server nickname, or nil if not set
---@field joined_at string|nil @ISO 8601 timestamp of when they joined
---@field roles string[] @Array of role snowflake IDs

--- Message data returned by `navi.fetch_message`.
---@class NaviFetchedMessage
---@field message_id string @The message's snowflake ID
---@field channel_id string @The channel's snowflake ID
---@field guild_id string|nil @The guild's snowflake ID, or nil in DMs
---@field content string @The message text
---@field author_id string @The author's snowflake ID
---@field author string @The author's username
---@field attachments string[] @Array of attachment URLs

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

--- A single option passed in the options table of `navi.create_slash`.
---@class NaviSlashOption
---@field name string @Option name shown in Discord
---@field description string @Help text shown in Discord
---@field type "string"|"integer"|"number"|"boolean"|"user"|"channel"|"role" @Discord option type
---@field required boolean|nil @Whether the option must be provided (default false)
---@field autocomplete (fun(ctx: {current_value: string, user_id: string, guild_id: string|nil}): {name: string, value: string}[])|nil @If set, Discord will call this as the user types. Return up to 25 {name, value} pairs.

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
---@field type "string"|"number"|"boolean"|"channel"|"role"|"category"|"enum" @Controls the sub-field input widget
---@field options string[]|nil @Required when type = "enum"; list of valid string values

--- A single entry in a plugin's configuration schema.
---@class NaviConfigItem
---@field key string @Database key used to store the value (e.g. `"log_channel"`)
---@field name string @Human-readable label shown in the TUI
---@field description string @Help text shown in the TUI
---@field type "string"|"number"|"boolean"|"channel"|"role"|"category"|"list"|"enum" @Controls the TUI input widget
---@field default string|number|boolean|nil @Value written to the DB if the user has not configured it yet; omit for list fields
---@field item_schema NaviListItemSchema[]|nil @Required when type = "list"; defines the sub-fields of each item
---@field options string[]|nil @Required when type = "enum"; list of valid string values

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
---@field register_modal fun(custom_id: string, callback: fun(ctx: NaviModalCtx)) @Registers a handler for modal submissions matching the given custom_id
---@field register_config fun(plugin_name: string, schema: NaviConfigItem[]) @Declares TUI-editable settings for a plugin; defaults are persisted to the DB automatically
---@field create_slash fun(name: string, description: string, options: NaviSlashOption[], callback: fun(ctx: NaviSlashCtx)) @Declares a slash command. Options may include an `autocomplete` function for live suggestions. Run `!sync` or press `r` to push to Discord.
---@field on fun(event_name: string, callback: fun(data: any)) @Subscribes to an inter-plugin event bus event
---@field emit fun(event_name: string, data: any) @Publishes an event to all subscribers on the inter-plugin event bus
---@field say fun(channel_id: string|number, text: string) @Sends plain text to a channel
---@field say_sync fun(channel_id: string|number, text: string): string|nil @Sends plain text and returns the message ID (blocks until sent), or nil on error
---@field send_message fun(channel_id: string|number, data: NaviEmbed) @Sends an embed (with optional buttons/selects) to a channel
---@field react fun(channel_id: string, message_id: string, emoji: string) @Adds a reaction emoji to a message
---@field edit_message fun(channel_id: string, message_id: string, content: string) @Edits the text content of an existing message
---@field delete_message fun(channel_id: string, message_id: string) @Deletes a message by ID
---@field kick fun(guild_id: string, user_id: string, reason: string|nil) @Kicks a member from the guild
---@field ban fun(guild_id: string, user_id: string, delete_message_days: number, reason: string|nil) @Bans a member. delete_message_days (0–7) controls how many days of messages to delete
---@field unban fun(guild_id: string, user_id: string) @Removes a ban
---@field timeout fun(guild_id: string, user_id: string, duration_seconds: number) @Times out a member. Pass 0 to clear an existing timeout
---@field create_channel fun(guild_id: string, name: string, options: NaviChannelOptions) @Creates a new text channel, optionally private and with a welcome message
---@field delete_channel fun(channel_id: string|number) @Permanently deletes a channel
---@field add_role fun(guild_id: string, user_id: string, role_id: string) @Assigns a role to a member
---@field remove_role fun(guild_id: string, user_id: string, role_id: string) @Removes a role from a member
---@field set_status fun(activity_type: "playing"|"listening"|"watching"|"competing"|"custom"|"none", text: string) @Changes the bot's Discord presence
---@field get_roles fun(guild_id: string|nil): NaviRole[] @Returns cached roles (call navi.refresh_cache or press u first)
---@field get_channels fun(guild_id: string|nil): NaviChannel[] @Returns cached text channels
---@field set_interval fun(callback: fun(), amount: number, unit: "ms"|"s"|"seconds"|"m"|"minutes"|"h"|"hours"|"d"|"days"|nil): number @Schedules `callback` to run every `amount` `unit`s. Unit defaults to `"ms"`. Returns an interval ID; all intervals are cancelled on plugin reload.
---@field clear_interval fun(id: number) @Cancels a running interval by its ID. No-op if the ID is not found.
---@field check_perm fun(ctx: NaviSlashCtx|NaviComponentCtx|NaviModalCtx, level: "user"|"helper"|"moderator"|"admin"|"owner"): boolean @Returns true if the user meets or exceeds the required level. No side effects.
---@field require_perm fun(ctx: NaviSlashCtx|NaviComponentCtx|NaviModalCtx, level: "user"|"helper"|"moderator"|"admin"|"owner"): boolean @Like check_perm(), but sends an ephemeral denial if the user lacks the level. Use as: if not navi.require_perm(ctx, "admin") then return end
---@field get_perm_level fun(ctx: NaviSlashCtx|NaviComponentCtx|NaviModalCtx): "user"|"helper"|"moderator"|"admin"|"owner" @Returns the user's highest permission level. Never nil; defaults to "user".
---@field depends_on fun(plugin_name: string) @Declares a load-order dependency on another plugin. Call this at the top of your plugin file. The Rust loader scans for these declarations and ensures dependencies execute first. No-op at runtime.
---@field dm fun(user_id: string, text: string) @Sends a plain-text DM to a user. Fire-and-forget; logs an error if the user has DMs disabled.
---@field get_member fun(guild_id: string, user_id: string): NaviMember|nil @Fetches live member info from Discord. Returns nil if the member is not found. Blocks until complete.
---@field fetch_message fun(channel_id: string, message_id: string): NaviFetchedMessage|nil @Fetches a message by ID. Returns nil if not found or inaccessible. Blocks until complete.

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

--- Called when a member leaves or is removed from a guild.
---@type fun(data: NaviMemberLeaveData)
on_member_leave = nil

--- Called when a message is edited. `new_content` may be nil if Discord omitted it.
---@type fun(data: NaviMessageEditData)
on_message_edit = nil

--- Called when a message is deleted.
---@type fun(data: NaviMessageDeleteData)
on_message_delete = nil

--- Called whenever a user's voice state changes (join, leave, mute, deafen, stream, etc.).
--- `channel_id` is nil when the user disconnected from voice entirely.
---@type fun(data: NaviVoiceStateData)
on_voice_state_update = nil

