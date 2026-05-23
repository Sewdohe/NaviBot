# Utilities

## HTTP Client

### `navi.http.get(url, headers?)`

Sends an HTTP GET request. **Blocks until complete.** Returns the response body as a string, or `nil` on error.

```lua
local body = navi.http.get("https://api.example.com/endpoint", nil)
if body then
    local data = navi.json.decode(body)
end
```

```lua
-- With custom headers
local body = navi.http.get("https://api.private.com/data", {
    ["Authorization"] = "Bearer " .. api_key,
    ["Accept"]        = "application/json"
})
```

### `navi.http.post(url, body, headers?)`

Sends an HTTP POST request. **Blocks until complete.** Returns the response body as a string, or `nil` on error.

```lua
local response = navi.http.post(
    "https://api.example.com/submit",
    navi.json.encode({ key = "value" }),
    { ["Content-Type"] = "application/json" }
)
```

> **Important:** Both `http.get` and `http.post` block the Lua thread while the request is in flight. For commands that make HTTP calls, always call `ctx.defer()` first.

## JSON

### `navi.json.encode(value)`

Serializes a Lua table or value to a JSON string.

```lua
local data = { title = "Best pet?", options = { "Dog", "Cat" }, closed = false }
navi.db.set("polls:data:1", navi.json.encode(data))
```

### `navi.json.decode(str)`

Parses a JSON string into a Lua value (table, number, string, boolean, or `nil`).

```lua
local raw  = navi.db.get("polls:data:1")
local poll = raw and navi.json.decode(raw)
if poll then
    navi.log.info("Poll title: " .. poll.title)
end
```

JSON and the database together are the standard pattern for storing **structured data** (arrays, nested tables) since `navi.db.set` only accepts flat strings.

## Permissions

The permissions system maps users and roles to a level hierarchy. Levels from lowest to highest: `"user"` → `"helper"` → `"moderator"` → `"admin"` → `"owner"`. Permissions are configured from the TUI.

### `navi.require_perm(ctx, level)`

Checks whether the user meets or exceeds the required level. If they **do not**, it automatically sends them an ephemeral denial message and returns `false`. If they **do**, it returns `true`.

```lua
navi.create_slash("nuke", "Delete all posts", {}, function(ctx)
    if not navi.require_perm(ctx, "moderator") then return end
    -- Only reaches here if the user is a moderator, admin, or owner
end)
```

### `navi.check_perm(ctx, level)`

Like `require_perm` but **silent** — returns `true` or `false` with no side effects. Use when you want to adjust behavior based on permissions without sending a denial.

```lua
if navi.check_perm(ctx, "moderator") then
    -- Show extra fields to mods
end
```

### `navi.get_perm_level(ctx)`

Returns the user's highest permission level as a string. Never returns `nil`; defaults to `"user"`.

```lua
local level = navi.get_perm_level(ctx)
ctx.reply("Your permission level is: " .. level)
```

---
