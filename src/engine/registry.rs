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

    // navi.autocomplete_handlers  { [cmd_name] = { [option_name] = fn } }
    let ac_handlers = lua.create_table()?;
    navi.set("autocomplete_handlers", ac_handlers)?;

    // navi.create_slash
    navi.set(
        "create_slash",
        lua.create_function(
            |lua, (name, desc, options, func): (String, String, LuaValue, Function)| {
                let navi: LuaTable = lua.globals().get("navi")?;
                let slash_cmds: LuaTable = navi.get("slash_commands")?;

                let cmd_data = lua.create_table()?;
                cmd_data.set("description", desc.clone())?;
                cmd_data.set("callback", func)?;

                let plugin_name: String = navi
                    .get("_current_plugin")
                    .unwrap_or_else(|_| "unknown".to_string());
                cmd_data.set("_plugin", plugin_name)?;

                // Extract autocomplete handlers from options before storing.
                // If an option has autocomplete = <function>, pull it out into
                // navi.autocomplete_handlers[cmd][option] and replace the value
                // with `true` so read_slash_commands can set the Discord flag.
                if let LuaValue::Table(ref opts) = options {
                    let ac_handlers: LuaTable = navi.get("autocomplete_handlers")?;
                    let cmd_ac: LuaTable = match ac_handlers.get::<_, LuaTable>(name.clone()) {
                        Ok(t) => t,
                        Err(_) => {
                            let t = lua.create_table()?;
                            ac_handlers.set(name.clone(), t.clone())?;
                            t
                        }
                    };
                    for pair in opts.clone().pairs::<LuaInteger, LuaTable>().flatten() {
                        let opt = pair.1;
                        let opt_name: String = opt.get("name").unwrap_or_default();
                        if let Ok(LuaValue::Function(handler)) = opt.get("autocomplete") {
                            cmd_ac.set(opt_name, handler)?;
                            opt.set("autocomplete", true)?; // flag for read_slash_commands
                        }
                    }
                }

                cmd_data.set("options", options)?;
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
