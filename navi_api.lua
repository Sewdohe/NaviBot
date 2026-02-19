---@meta

-- GLOBAL NAMESPACE
navi = {}
navi.db = {}

-- TYPES

---@class EmbedField
---@field name string The field title
---@field value string The field text
---@field inline boolean? Whether it sits inline

---@class Component
---@field type "button"
---@field label string Text on the button
---@field id string? Custom ID (Required for normal buttons)
---@field url string? URL (Required for 'link' style buttons)
---@field style "primary"|"secondary"|"success"|"danger"|"link"

---@class MessageData
---@field title string?
---@field description string?
---@field color integer? Hex color (0xFF0000)
---@field fields EmbedField[]?
---@field components Component[]?

---@alias ReplyFn fun(message: string, ephemeral: boolean)

---@class ComponentContext
---@field custom_id string The ID of the clicked button
---@field user_id string Who clicked it
---@field channel_id string Where they clicked it
---@field guild_id string? The server ID (nil if DM)
---@field values string[]? Selected values (for dropdowns)
---@field reply fun(message: string, ephemeral: boolean) Send a response

-- FUNCTIONS

--- Send a message with optional embed and buttons
---@param channel_id string
---@param data MessageData
function navi.send_message(channel_id, data) end

--- Send a plain text message
---@param channel_id string
---@param content string
function navi.say(channel_id, content) end

--- Add a role to a user
---@param guild_id string
---@param user_id string
---@param role_id string
function navi.add_role(guild_id, user_id, role_id) end

--- Remove a role from a user
---@param guild_id string
---@param user_id string
---@param role_id string
function navi.remove_role(guild_id, user_id, role_id) end

--- Get a value from the KV store
---@param key string
---@return string?
function navi.db.get(key) end

--- Set a value in the KV store
---@param key string
---@param value string
function navi.db.set(key, value) end

--- Register a chat listener
---@param callback fun(msg: Message)
function navi.register(callback) end

--- Create a Slash Command
---@param name string Command name
---@param desc string Description
---@param options table[] Arguments
---@param callback fun(ctx: SlashContext)
function navi.create_slash(name, desc, options, callback) end

--- (Global Callback) Handle button/dropdown interactions
--- Define this in your plugin to catch clicks!
---@param ctx ComponentContext
function on_component(ctx) end