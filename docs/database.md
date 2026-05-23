# Database

The database is a single SQLite table (`kv_store`) with `key` and `value` columns. All values are stored as strings.

## Automatic Namespacing

`navi.db.get` and `navi.db.set` **automatically prepend the calling plugin's filename** as a namespace. A call to `navi.db.get("score")` from `trivia.lua` actually reads the key `trivia:score`.

**To bypass namespacing** and use an exact key, include a `:` anywhere in the key string:

```lua
-- From casino.lua — reads economy.lua's balance data directly
local balance = navi.db.get("economy:balance:" .. user_id)
```

This is intentional. Cross-plugin data access is fine as long as you know the key format.

## `navi.db.get(key)`

Reads a value from the database.

- **Returns:** `string | nil` — the value, or `nil` if the key doesn't exist.

```lua
local xp = navi.db.get("xp:" .. user_id)
if xp == nil then
    xp = "0"  -- default value
end
local xp_num = tonumber(xp) or 0
```

## `navi.db.set(key, value)`

Writes a value to the database. Creates the key if it doesn't exist; overwrites it if it does.

- `value` can be a string, number, or boolean. Numbers and booleans are automatically converted to strings.

```lua
navi.db.set("xp:" .. user_id, tostring(new_xp))
navi.db.set("score", 42)          -- stored as "42"
navi.db.set("enabled", true)      -- stored as "true"
```

> **Important:** Always use `tonumber()` when reading a value you intend to do math with. The database always gives you strings back.

## `navi.db.query(sql)`

Executes a raw SQL statement against the `kv_store` table. Returns an array of row tables.

- Each row has a `key` field and a `value` field.
- Use this for sorted queries, `LIKE` pattern searches, and leaderboards — things you can't do with simple get/set.

```lua
-- Top 10 XP earners, sorted descending
local rows = navi.db.query(
    "SELECT key, value FROM kv_store " ..
    "WHERE key LIKE 'leveling:xp:%' " ..
    "ORDER BY CAST(value AS INTEGER) DESC LIMIT 10"
)

for i, row in ipairs(rows) do
    local user_id = row.key:match("leveling:xp:(.+)")
    navi.log.info(i .. ". " .. user_id .. " — " .. row.value .. " XP")
end
```

## `navi.db.get_list(key)`

Reads a `list`-type config field (registered with `navi.register_config`) and returns it as an array of tables. Each table has the keys defined in `item_schema`.

```lua
local rewards = navi.db.get_list("config:leveling:role_rewards")
for _, r in ipairs(rewards) do
    print(r.level, r.role_id)
end
```

---
