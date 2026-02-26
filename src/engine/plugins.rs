use mlua::prelude::*;
use poise::serenity_prelude as serenity;
use serenity::{CreateCommand, CreateCommandOption};
use std::collections::HashMap;
use std::path::Path;
use crate::types::{Error, LogLevel, IntervalRegistry};

pub fn load_plugins(lua: &Lua, interval_registry: &IntervalRegistry) -> (LogLevel, String) {
    // 0. Cancel all running intervals before reloading
    {
        let mut reg = interval_registry.lock().unwrap();
        for (_, cancel_tx) in reg.drain() {
            let _ = cancel_tx.send(true);
        }
    }

    // 1. Reset Event Bus
    let result: LuaResult<()> = (|| {
        let navi: LuaTable = lua.globals().get("navi")?;
        navi.set("listeners", lua.create_table()?)?;
        Ok(())
    })();

    if let Err(e) = result {
        return (LogLevel::Error, format!("Failed to reset listeners: {}", e));
    }

    // 2. Read Files
    let mut count = 0;
    let mut details = String::new();
    let mut error_msg = None;

    if !Path::new("plugins").exists() {
        return (LogLevel::Warn, "'plugins/' folder not found!".to_string());
    }

    // --- BAKE IN THE CORE ENGINE API ---
    let core_events = include_str!("../../core/events.lua");
    let core_components = include_str!("../../core/components.lua");
    let core_modals = include_str!("../../core/modals.lua");
    // let core_dispatcher = include_str!("../../core/dispatcher.lua");

    // Execute them in exact order
    let _ = lua.load(core_events).set_name("core:events").exec();
    let _ = lua.load(core_components).set_name("core:components").exec();
    let _ = lua.load(core_modals).set_name("core:modals").exec();
    // let _ = lua.load(core_dispatcher).set_name("core:dispatcher").exec();

    if let Ok(entries) = std::fs::read_dir("plugins") {
        // 1. Collect all valid .lua files into a Vector first
        let mut lua_files = Vec::new();
        for entry in entries {
            if let Ok(file) = entry {
                let p = file.path();
                if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("lua") {
                    lua_files.push(p);
                }
            }
        }

        // 2. Sort plugin loading alphabetically
        lua_files.sort();

        // 3. Now execute them in guaranteed order
        for p in lua_files {
            // Expose the current plugin name so create_slash can tag commands
            let stem = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            if let Ok(navi) = lua.globals().get::<_, LuaTable>("navi") {
                let _ = navi.set("_current_plugin", stem);
            }

            if let Ok(code) = std::fs::read_to_string(&p) {
                let chunk = lua.load(&code).set_name(p.to_string_lossy());
                if let Err(e) = chunk.exec() {
                    error_msg = Some(format!("Error in {:?}: \n{}", p, e));
                    break;
                }
                count += 1;
                details.push_str(&format!("Loaded: {:?}\n", p.file_name().unwrap()));
            }
        }
    }

    if let Some(err) = error_msg {
        (LogLevel::Error, err)
    } else {
        (LogLevel::Info, format!("Loaded {} plugins.", count))
    }
}

fn option_type_from_str(s: &str) -> serenity::CommandOptionType {
    match s {
        "string" => serenity::CommandOptionType::String,
        "integer" => serenity::CommandOptionType::Integer,
        "boolean" => serenity::CommandOptionType::Boolean,
        "user" => serenity::CommandOptionType::User,
        "channel" => serenity::CommandOptionType::Channel,
        "role" => serenity::CommandOptionType::Role,
        "number" => serenity::CommandOptionType::Number,
        _ => serenity::CommandOptionType::String,
    }
}

pub fn read_slash_commands(lua: &Lua) -> Result<Vec<CreateCommand>, Error> {
    let navi: LuaTable = lua.globals().get("navi")?;
    let slash_cmds: LuaTable = match navi.get("slash_commands") {
        Ok(t) => t,
        Err(_) => return Ok(Vec::new()),
    };

    // Group commands by their originating plugin
    let mut plugin_groups: HashMap<String, Vec<(String, LuaTable)>> = HashMap::new();

    for pair in slash_cmds.pairs::<String, LuaTable>() {
        let (name, data) = pair?;
        let plugin: String = data
            .get("_plugin")
            .unwrap_or_else(|_| "unknown".to_string());
        plugin_groups.entry(plugin).or_default().push((name, data));
    }

    let mut commands = Vec::new();

    for (plugin_name, cmds) in plugin_groups {
        if cmds.len() == 1 {
            // Single command — register as a flat top-level command
            let (name, data) = cmds.into_iter().next().unwrap();
            let desc: String = data.get("description")?;
            let mut command = CreateCommand::new(name).description(desc);

            if let Ok(options) = data.get::<_, Vec<LuaTable>>("options") {
                for opt in options {
                    let oname: String = opt.get("name")?;
                    let odesc: String = opt.get("description")?;
                    let type_str: String = opt.get("type")?;
                    let required: bool = opt.get("required").unwrap_or(false);
                    let autocomplete: bool = opt.get("autocomplete").unwrap_or(false);
                    let kind = option_type_from_str(&type_str);
                    command = command.add_option(
                        CreateCommandOption::new(kind, oname, odesc)
                            .required(required)
                            .set_autocomplete(autocomplete),
                    );
                }
            }
            commands.push(command);
        } else {
            // Multiple commands — group them under the plugin name as subcommands
            let group_desc = format!(
                "{} commands",
                plugin_name
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_default()
                    + &plugin_name[plugin_name.char_indices().nth(1).map_or(0, |(i, _)| i)..]
            );
            let mut group_cmd = CreateCommand::new(&plugin_name).description(group_desc);

            for (name, data) in cmds {
                let desc: String = data.get("description")?;
                let mut subcmd = CreateCommandOption::new(
                    serenity::CommandOptionType::SubCommand,
                    name,
                    desc,
                );

                if let Ok(options) = data.get::<_, Vec<LuaTable>>("options") {
                    for opt in options {
                        let oname: String = opt.get("name")?;
                        let odesc: String = opt.get("description")?;
                        let type_str: String = opt.get("type")?;
                        let required: bool = opt.get("required").unwrap_or(false);
                        let autocomplete: bool = opt.get("autocomplete").unwrap_or(false);
                        let kind = option_type_from_str(&type_str);
                        subcmd = subcmd.add_sub_option(
                            CreateCommandOption::new(kind, oname, odesc)
                                .required(required)
                                .set_autocomplete(autocomplete),
                        );
                    }
                }

                group_cmd = group_cmd.add_option(subcmd);
            }

            commands.push(group_cmd);
        }
    }

    Ok(commands)
}

pub async fn upload_slash_commands(
    http: &serenity::Http,
    commands: Vec<CreateCommand>,
) -> Result<String, Error> {
    let guilds = http.get_guilds(None, None).await?;
    let count = guilds.len();

    for guild in guilds {
        guild.id.set_commands(http, commands.clone()).await?;
    }

    Ok(format!("✅ Synced slash commands to {} guilds.", count))
}
