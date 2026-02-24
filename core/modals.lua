navi.log.info("Loading Modal Manager")

navi._modals = {}

navi.register_modal = function(custom_id, callback)
    navi._modals[custom_id] = callback
end

function on_modal_submit(ctx)
    local handler = navi._modals[ctx.custom_id]
    if handler then
        local success, err = pcall(handler, ctx)
        if not success then
            ctx.reply("❌ Modal Error: " .. tostring(err), true)
            navi.log.error("Modal Error (" .. ctx.custom_id .. "): " .. tostring(err))
        end
    else
        navi.log.warn("Unhandled modal submit: " .. ctx.custom_id)
    end
end
