navi.log.info("Loading Component Manager")

navi._components = {}

navi.register_component = function(custom_id, callback)
    navi._components[custom_id] = callback
end

function on_component(ctx)
    local handler = navi._components[ctx.custom_id]
    
    if handler then
        local success, err = pcall(handler, ctx)
        if not success then
            ctx.reply("❌ Component Error: " .. tostring(err), true)
            navi.log.error("Component Error (" .. ctx.custom_id .. "): " .. tostring(err))
        end
    else
        navi.log.warn("Unhandled component interaction: " .. ctx.custom_id)
    end
end