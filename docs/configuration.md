# Configuration

## Registering Config

### `navi.register_config(plugin_name, schema)`

Declares settings that can be edited from the TUI (`c` key) and are persisted in SQLite. Call this **once per plugin**, at the top of the file.

- `plugin_name` (string): The unique namespace for this plugin's config (usually the filename without `.lua`).
- `schema` (table): A list of setting definitions.

```lua
navi.register_config("my_plugin", {
    { key = "welcome_channel", name = "Welcome Channel", description = "Where to greet new users.", type = "channel", default = "" },
    { key = "max_points",      name = "Max Points",      description = "Point cap per user.",       type = "number",  default = 1000 },
    { key = "enabled",         name = "Enabled",         description = "Turn the plugin on/off.",   type = "boolean", default = true },
})
```

### Config Item Fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `key` | string | Yes | The DB key used to store the value. Read back as `config:plugin_name:key`. |
| `name` | string | Yes | Human-readable label shown in the TUI. |
| `description` | string | Yes | Help text shown below the field in the TUI. |
| `type` | string | Yes | Controls the input widget. See types below. |
| `default` | any | No | Value written to DB if the user hasn't configured it yet. Omit for `list` fields. |
| `item_schema` | table | Only for `list` | Defines the sub-fields of each list item. |
| `options` | string[] | Only for `enum` | The allowed values shown in the TUI dropdown. |

### Config Types

| Type | TUI Widget | Notes |
| --- | --- | --- |
| `"string"` | Text input | Free-form text |
| `"number"` | Number input | Stored as a string; use `tonumber()` when reading |
| `"boolean"` | Toggle | `true` / `false` |
| `"channel"` | Channel picker | Stores a channel snowflake ID string |
| `"role"` | Role picker | Stores a role snowflake ID string |
| `"category"` | Category picker | Stores a category (channel group) snowflake ID string |
| `"list"` | Expandable list | Each item is a sub-table; requires `item_schema` |
| `"enum"` | Option dropdown | Stores one of the strings declared in `options`; requires `options` |

## Reading Config Values

Config values are stored with the key format `config:plugin_name:key`. You read them with `navi.db.get`:

```lua
local channel = navi.db.get("config:my_plugin:welcome_channel")
local max     = tonumber(navi.db.get("config:my_plugin:max_points")) or 1000
local enabled = navi.db.get("config:my_plugin:enabled") == "true"
```

> **The `"config:"` prefix:** `navi.db.get` auto-namespacing does **not** apply to `config:` keys (the `config:` prefix already makes the key explicit). Always use the full key path when reading config values manually.

## List Config

A `list` config field stores multiple structured items — for example, a list of level-up role rewards. Define the sub-fields with `item_schema` and read them back with `navi.db.get_list`.

```lua
navi.register_config("leveling", {
    { key = "role_rewards", name = "Role Rewards", description = "Roles granted at specific levels.", type = "list",
      item_schema = {
          { key = "level",   name = "Level", type = "number" },
          { key = "role_id", name = "Role",  type = "role"   }
      }
    }
})

-- Reading the list back:
local rewards = navi.db.get_list("config:leveling:role_rewards")
for _, item in ipairs(rewards) do
    navi.log.info("Level " .. item.level .. " → Role " .. item.role_id)
end
```

Each entry in `item_schema` supports these fields:

| Field | Required | Description |
| --- | --- | --- |
| `key` | Yes | The key used inside each item table |
| `name` | Yes | Human-readable label shown in the TUI |
| `type` | Yes | Sub-field input type (see below) |
| `options` | Only for `enum` | List of valid string values shown in the dropdown |

**Sub-field types** (all types except `"list"` are allowed):

| Type | TUI Widget |
| --- | --- |
| `"string"` | Text input |
| `"number"` | Text input (use `tonumber()` when reading) |
| `"boolean"` | Toggle |
| `"channel"` | Channel picker |
| `"role"` | Role picker |
| `"category"` | Category picker |
| `"enum"` | Option dropdown — requires `options` |

## Enum Config

Use `type = "enum"` when a field must be one of a fixed set of strings. The TUI shows a dropdown instead of a free-text box, preventing invalid input.

```lua
-- Top-level enum field
navi.register_config("myplugin", {
    {
        key = "mode",
        name = "Operating Mode",
        description = "Controls how the plugin behaves.",
        type = "enum",
        options = { "strict", "lenient", "disabled" },
        default = "lenient"
    }
})

local mode = navi.db.get("config:myplugin:mode")  -- "strict", "lenient", or "disabled"
```

```lua
-- Enum sub-field inside a list
navi.register_config("permissions", {
    { key = "mappings", name = "Role → Permission Level", description = "Map roles to levels.", type = "list",
      item_schema = {
          { key = "role_id", name = "Discord Role",     type = "role" },
          { key = "level",   name = "Permission Level", type = "enum",
            options = { "helper", "moderator", "admin" } }
      }
    }
})
```

---
