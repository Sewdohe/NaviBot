# Messaging & Discord API

## Sending Messages

### `navi.say(channel_id, text)`

Sends a plain-text message. Fire-and-forget (non-blocking).

```lua
navi.say(msg.channel_id, "Hello world!")
```

### `navi.say_sync(channel_id, text)`

Like `navi.say` but **blocks until the message is sent** and returns the new message's snowflake ID as a string, or `nil` on error.

```lua
local msg_id = navi.say_sync(channel_id, "This message was sent synchronously")
```

### `navi.send_message(channel_id, embed_table)`

Sends a rich embed with optional buttons/selects. Fire-and-forget.

```lua
navi.send_message(channel_id, {
    title = "Welcome!",
    description = "Glad to have you here.",
    color = 0x2ECC71
})
```

### `navi.send_message_sync(channel_id, embed_table)`

Like `navi.send_message` but **blocks until sent** and returns the message ID as a string, or `nil` on error. Useful when you need to store the message ID for later editing.

```lua
local message_id = navi.send_message_sync(channel_id, build_embed())
if message_id then
    navi.db.set("stats:live_message_id", message_id)
end
```

### `navi.dm(user_id, text)`

Sends a plain-text direct message to a user. If the user has DMs disabled, the error is logged but does not crash the plugin.

```lua
navi.dm(ctx.user_id, "Hey, check this out!")
```

### `navi.react(channel_id, message_id, emoji)`

Adds a reaction to a message.

- `emoji`: A Unicode emoji (`"❤️"`, `"⭐"`) or a custom emoji string in the format `"name:id"`.

```lua
navi.react(tostring(msg.channel_id), tostring(msg.message_id), "⭐")
```

### `navi.edit_message(channel_id, message_id, content)`

Edits the text content of a message the bot previously sent.

```lua
navi.edit_message(channel_id, message_id, "Updated content")
```

### `navi.edit_embed(channel_id, message_id, embed_table)`

Replaces the embed on a message the bot previously sent. Useful for live-updating stat embeds.

```lua
navi.edit_embed(channel_id, message_id, build_embed())
```

### `navi.delete_message(channel_id, message_id)`

Deletes a message.

```lua
navi.delete_message(tostring(msg.channel_id), tostring(msg.message_id))
```

### `navi.fetch_message(channel_id, message_id)`

Fetches a message from Discord by ID. **Blocks until complete.** Returns a table or `nil` if not found.

```lua
local fetched = navi.fetch_message(channel_id, message_id)
if fetched then
    navi.log.info("Message content: " .. fetched.content)
end
```

| Field | Type | Description |
| --- | --- | --- |
| `fetched.message_id` | string | The message's snowflake ID |
| `fetched.channel_id` | string | The channel's snowflake ID |
| `fetched.guild_id` | string\|nil | The guild's snowflake ID |
| `fetched.content` | string | The message text |
| `fetched.author_id` | string | The author's snowflake ID |
| `fetched.author` | string | The author's username |
| `fetched.attachments` | string[] | Array of attachment URLs |

## Embed Structure

An embed is a rich message card. You pass the same embed table structure to `navi.send_message`, `ctx.reply_embed`, and `ctx.followup_embed`.

```lua
{
    title       = "Optional title",
    description = "Main body text. Supports **markdown**.",
    color       = 0x3498DB,
    image       = "https://example.com/image.png",
    fields = {
        { name = "Field 1", value = "Some text",  inline = true  },
        { name = "Field 2", value = "More text",  inline = true  },
        { name = "Long one", value = "Full width", inline = false },
    },
    components = {
        -- Buttons and/or a select menu
    }
}
```

| Field | Type | Description |
| --- | --- | --- |
| `title` | string | Bold title at the top |
| `description` | string | Main body text (supports Discord markdown) |
| `color` | number | Hex color as an integer, e.g. `0xFF0000` for red |
| `image` | string | URL of a large image to display at the bottom |
| `fields` | table | Array of `{name, value, inline?}` objects |
| `components` | table | Array of buttons and/or one select menu |

When `inline = true`, up to 3 fields display side-by-side. Use `inline = false` (or omit it) for full-width fields.

### Common Colors

```lua
0x2ECC71  -- Emerald green  (success)
0xE74C3C  -- Alizarin red   (error / danger)
0xF1C40F  -- Sunflower gold (warning / economy)
0x3498DB  -- Peter River blue (info)
0x5865F2  -- Discord blurple
0x99AAB5  -- Grey/neutral
0x000000  -- Black (removes the color bar)
```

## Member & Role Management

### `navi.add_role(guild_id, user_id, role_id)`

Assigns a role to a member. All arguments are strings.

```lua
navi.add_role(ctx.guild_id, ctx.user_id, verified_role_id)
```

### `navi.remove_role(guild_id, user_id, role_id)`

Removes a role from a member.

```lua
navi.remove_role(ctx.guild_id, ctx.user_id, unverified_role_id)
```

### `navi.get_member(guild_id, user_id)`

Fetches live member info from Discord. **Blocks until complete.** Returns a table or `nil`.

```lua
local member = navi.get_member(ctx.guild_id, ctx.user_id)
if member then
    navi.log.info("Display name: " .. member.display_name)
end
```

| Field | Type | Description |
| --- | --- | --- |
| `member.user_id` | string | The member's snowflake ID |
| `member.username` | string | The member's username |
| `member.display_name` | string | Nickname if set, otherwise username |
| `member.nickname` | string\|nil | Server-specific nickname, or `nil` |
| `member.joined_at` | string\|nil | ISO 8601 timestamp of when they joined |
| `member.roles` | string[] | Array of role snowflake IDs |

### `navi.kick(guild_id, user_id, reason?)`

Kicks a member from the guild. The reason appears in the audit log.

```lua
navi.kick(ctx.guild_id, target_user_id, "Spam")
```

### `navi.ban(guild_id, user_id, delete_message_days, reason?)`

Bans a member. `delete_message_days` (0–7) controls how many days of their recent messages to delete.

```lua
navi.ban(ctx.guild_id, target_user_id, 1, "Harassment")
```

### `navi.unban(guild_id, user_id)`

Removes a ban.

```lua
navi.unban(ctx.guild_id, target_user_id)
```

### `navi.timeout(guild_id, user_id, duration_seconds)`

Times out a member (they cannot send messages or join voice). Pass `0` to remove an existing timeout.

```lua
navi.timeout(ctx.guild_id, target_user_id, 600)  -- 10 minutes
navi.timeout(ctx.guild_id, target_user_id, 0)    -- remove timeout
```

## Channel Management

### `navi.create_channel(guild_id, name, options)`

Creates a new text channel in the guild.

```lua
navi.create_channel(ctx.guild_id, "ticket-username", {
    category_id     = category_id,
    user_id         = ctx.user_id,
    role_id         = support_role_id,
    welcome_message = "Welcome! Staff will be with you shortly.",
    close_button    = true
})
```

| Option | Type | Description |
| --- | --- | --- |
| `category_id` | string\|nil | Snowflake ID of the parent category |
| `user_id` | string\|nil | Grant this user private View + Send permissions |
| `role_id` | string\|nil | Grant this role private View + Send permissions |
| `welcome_message` | string\|nil | Text sent immediately after the channel is created |
| `close_button` | boolean\|nil | If `true`, attaches a red "Close Ticket" button to the welcome message |

### `navi.delete_channel(channel_id)`

Permanently deletes a channel. This is instant and irreversible.

```lua
navi.delete_channel(ctx.channel_id)
```

### `navi.create_thread(channel_id, name, options?)`

Creates a thread and returns its channel ID as a string, or `nil` on failure. **Blocks until complete.**

```lua
local thread_id = navi.create_thread(ctx.channel_id, "Discussion", {
    message_id   = ctx.message_id,
    private      = true,
    auto_archive = 60
})

if thread_id then
    navi.say(thread_id, "Thread is open!")
end
```

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| `message_id` | string\|nil | nil | If set, creates the thread attached to this message |
| `private` | boolean | false | Create a private thread (invite-only) |
| `auto_archive` | number | 1440 | Minutes until auto-archive: `60`, `1440`, `4320`, `10080` |

## Discord Cache

These functions return data from the bot's **cached** guild state. Press `u` in the TUI to refresh the cache.

### `navi.get_roles(guild_id?)`

Returns an array of role objects for the guild.

```lua
local roles = navi.get_roles(ctx.guild_id)
for _, role in ipairs(roles) do
    navi.log.info(role.id .. " — " .. role.name)
end
```

| Field | Type | Description |
| --- | --- | --- |
| `role.id` | string | The role's snowflake ID |
| `role.name` | string | The role's display name |
| `role.color` | integer[] | RGB tuple: `{r, g, b}` |

### `navi.get_channels(guild_id?)`

Returns an array of text channel objects for the guild.

```lua
local channels = navi.get_channels(ctx.guild_id)
for _, ch in ipairs(channels) do
    navi.log.info(ch.id .. " — #" .. ch.name)
end
```

| Field | Type | Description |
| --- | --- | --- |
| `ch.id` | string | The channel's snowflake ID |
| `ch.name` | string | The channel's display name |

## Bot Status

### `navi.set_status(activity_type, text)`

Changes the bot's Discord presence (the "Playing X" message shown in the member list).

- `activity_type`: `"playing"`, `"listening"`, `"watching"`, `"competing"`, `"custom"`, or `"none"`

```lua
navi.set_status("watching", "over the server")
navi.set_status("none", "")  -- clears the status
```

---
