use mlua::prelude::*;
use tokio::sync::mpsc::UnboundedSender;
use crate::types::{BotEvent, LogLevel};

pub fn register(lua: &Lua, navi: &LuaTable, tui_tx: UnboundedSender<BotEvent>) -> LuaResult<()> {
    let navi_log = lua.create_table()?;

    let tx_info = tui_tx.clone();
    navi_log.set("_info", lua.create_function(move |_, msg: String| {
        let _ = tx_info.send(BotEvent::Log(LogLevel::Info, msg));
        Ok(())
    })?)?;

    let tx_warn = tui_tx.clone();
    navi_log.set("_warn", lua.create_function(move |_, msg: String| {
        let _ = tx_warn.send(BotEvent::Log(LogLevel::Warn, msg));
        Ok(())
    })?)?;

    let tx_err = tui_tx.clone();
    navi_log.set("_error", lua.create_function(move |_, msg: String| {
        let _ = tx_err.send(BotEvent::Log(LogLevel::Error, msg));
        Ok(())
    })?)?;

    navi.set("log", navi_log)?;

    lua.load(r#"
        do
            local function source()
                local info = debug.getinfo(3, "S")
                local src = info and info.short_src or "unknown"
                -- short_src for string-loaded chunks looks like [string "plugins/foo.lua"]
                -- extract the inner path if wrapped in quotes
                src = src:match('"([^"]+)"') or src
                src = src:gsub("plugins[/\\]", "")
                return "[" .. src .. "]"
            end
            navi.log.info  = function(msg) navi.log._info(source()  .. tostring(msg)) end
            navi.log.warn  = function(msg) navi.log._warn(source()  .. tostring(msg)) end
            navi.log.error = function(msg) navi.log._error(source() .. tostring(msg)) end
        end
    "#).exec()?;

    lua.load(r#"
        local old_print = print
        _G.print = function(...)
            local args = {...}
            local parts = {}
            for i, v in ipairs(args) do
                table.insert(parts, tostring(v))
            end
            local msg = table.concat(parts, "\t")
            local info = debug.getinfo(2, "S")
            local source = info and info.short_src or "unknown"
            source = source:match('"([^"]+)"') or source
            source = source:gsub("plugins[/\\]", "")
            navi.log._info(string.format("[%s] %s", source, msg))
        end
    "#).exec()?;

    Ok(())
}
