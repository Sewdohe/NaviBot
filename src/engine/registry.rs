use mlua::prelude::*;
use mlua::Function;

pub fn register(lua: &Lua, navi: &LuaTable) -> LuaResult<()> {
    // navi.listeners
    let listeners = lua.create_table()?;
    navi.set("listeners", listeners)?;

    // navi.register
    navi.set(
        "register",
        lua.create_function(|lua, func: Function| {
            let navi: LuaTable = lua.globals().get("navi")?;
            let listeners: LuaTable = navi.get("listeners")?;
            listeners.set(listeners.len()? + 1, func)?;
            Ok(())
        })?,
    )?;

    // navi.slash_commands
    let slash_cmds = lua.create_table()?;
    navi.set("slash_commands", slash_cmds)?;

    // navi.create_slash
    navi.set(
        "create_slash",
        lua.create_function(
            |lua, (name, desc, options, func): (String, String, LuaValue, Function)| {
                let navi: LuaTable = lua.globals().get("navi")?;
                let slash_cmds: LuaTable = navi.get("slash_commands")?;

                let cmd_data = lua.create_table()?;
                cmd_data.set("description", desc)?;
                cmd_data.set("options", options)?;
                cmd_data.set("callback", func)?;

                slash_cmds.set(name, cmd_data)?;
                Ok(())
            },
        )?,
    )?;

    // on_message conductor
    lua.load(
        r#"
        function on_message(msg)
            if navi.listeners then
                for i, listener in ipairs(navi.listeners) do
                    pcall(listener, msg)
                end
            end
        end
    "#,
    )
    .exec()?;

    Ok(())
}
