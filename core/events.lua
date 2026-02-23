navi.log.info("Loading Core: Event Bus")

navi._events = {}

-- 1. Listen for an event
navi.on = function(event_name, callback)
  if not navi._events[event_name] then
    navi._events[event_name] = {}
  end
  table.insert(navi._events[event_name], callback)
end

-- 2. Broadcast an event to anyone listening
navi.emit = function(event_name, data)
  local listeners = navi._events[event_name]
  if listeners then
    for _, callback in ipairs(listeners) do
      -- Run safely so one bad plugin doesn't crash the bus
      local success, err = pcall(callback, data)
      if not success then
        navi.log.warn("Event Bus Error (" .. event_name .. "): " .. tostring(err))
      end
    end
  end
end

-- Rust calls this global function when a user joins
function on_member_join(user_id, username, guild_id)
  -- Package the data into a clean table and broadcast it to all plugins
  navi.emit("member_join", {
    user_id = user_id,
    username = username,
    guild_id = guild_id
  })
end

-- The bridge for new messages
function on_message(msg)
  -- Support navi.register() listeners
  if navi.listeners then
    for _, listener in ipairs(navi.listeners) do
      pcall(listener, msg)
    end
  end
  -- Support navi.on("message", ...) event bus subscribers
  navi.emit("message", msg)
end
