navi.log.info("Loading Permissions Plugin")

navi.register_config("permissions", {
    {
        key = "level_order",
        name = "Permission Levels (low → high)",
        description = "Define levels from lowest to highest. Higher index = higher rank.",
        type = "list",
        item_schema = {
            { key = "name", name = "Level Name", type = "string" }
        }
    },
    {
        key = "mappings",
        name = "Role → Level Mappings",
        description = "Assign a Discord role to a named permission level.",
        type = "list",
        item_schema = {
            { key = "role_id", name = "Role",             type = "role"   },
            { key = "level",   name = "Permission Level", type = "string" }
        }
    }
})

-- Build level-name → numeric rank table (1 = lowest). Called fresh each time.
local function build_rank_table()
    local order = navi.db.get_list("config:permissions:level_order")
    local ranks = {}
    for i, entry in ipairs(order) do
        if entry.name and entry.name ~= "" then
            ranks[entry.name] = i
        end
    end
    return ranks
end

-- Returns the highest rank (number) the user holds, based on their roles.
local function user_rank(member_roles)
    local mappings = navi.db.get_list("config:permissions:mappings")
    local ranks    = build_rank_table()

    local role_set = {}
    for _, rid in ipairs(member_roles or {}) do
        role_set[rid] = true
    end

    local best = 0
    for _, m in ipairs(mappings) do
        if m.role_id and m.level and role_set[m.role_id] then
            local r = ranks[m.level] or 0
            if r > best then best = r end
        end
    end
    return best
end

perms = {}

-- Returns true if user meets or exceeds required_level. No side effects.
function perms.check(ctx, required_level)
    if not ctx.guild_id then return false end
    local ranks = build_rank_table()
    local req   = ranks[required_level]
    if not req then
        navi.log.warn("[permissions] Unknown level: '" .. tostring(required_level) .. "'")
        return false
    end
    return user_rank(ctx.member_roles) >= req
end

-- Like check(), but sends an ephemeral denial message if denied.
-- Use as: if not perms.require(ctx, "admin") then return end
function perms.require(ctx, required_level)
    if perms.check(ctx, required_level) then return true end
    ctx.reply("❌ You need the **" .. required_level .. "** permission to use this.", true)
    return false
end

-- Returns the user's highest permission level name, or nil.
function perms.level(ctx)
    if not ctx.guild_id or not ctx.member_roles then return nil end
    local ranks = build_rank_table()
    local role_set = {}
    for _, rid in ipairs(ctx.member_roles) do role_set[rid] = true end
    local best_rank, best_name = 0, nil
    for _, m in ipairs(navi.db.get_list("config:permissions:mappings")) do
        if m.role_id and m.level and role_set[m.role_id] then
            local r = ranks[m.level] or 0
            if r > best_rank then best_rank, best_name = r, m.level end
        end
    end
    return best_name
end

navi.log.info("Permissions ready. Use: if not perms.require(ctx, 'level') then return end")
