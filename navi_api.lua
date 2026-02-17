---@meta

---Registers a function to run whenever a message is received
---@param callback fun(msg: Message) The function to run
function navi.register(callback) end

---Registers a new command with the bot
---@param name string The command name (without prefix, e.g. "ping")
---@param callback fun(msg: Message, args: string[]) The function to run
function navi.create_command(name, callback) end

---@class Message
---@field content string The text content
---@field message_id string|number The unique ID of this message
---@field channel_id string|number The Channel ID
---@field author string The username (e.g. "Sewdohe")
---@field author_id string|number The user's unique ID
---@field author_avatar string URL to the user's profile picture
---@field attachments string[] List of URLs for any uploaded images/files
---@field mentions User[] A list of users tagged in the message

---@class User
---@field name string
---@field id string|number
---@field avatar string URL to avatar

---@class Navi
navi = {}

---Sends a message to a specific Discord channel.
---@param channel_id string|number The Channel ID
---@param text string The message to send
function navi.say(channel_id, text) end

---@type Navi
_G.navi = navi

---@class EmbedField
---@field name string The title of the field
---@field value string The content of the field
---@field inline? boolean Whether fields sit side-by-side (default: false)

---@class EmbedFooter
---@field text string The footer text
---@field icon_url? string Small icon next to footer text

---@class Embed
---@field title? string Big bold title
---@field description? string Main body text
---@field color? integer Hex color (e.g. 0xFF0000 for red)
---@field url? string Clicking the title links here
---@field image? string URL of big image at bottom
---@field thumbnail? string URL of small image at top-right
---@field footer? EmbedFooter
---@field fields? EmbedField[] List of fields

---Sends a rich embed card to a channel
---@param channel_id string|number
---@param embed Embed
function navi.send_embed(channel_id, embed) end