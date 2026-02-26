mod config;
mod db;
mod discord;
mod intervals;
mod logging;
mod messaging;
mod perms;
mod plugins;
mod registry;
mod util;

pub use plugins::{load_plugins, read_slash_commands, upload_slash_commands};

use crate::types::{BotEvent, Data, Error, LogLevel};
use mlua::prelude::*;
use mlua::{LuaOptions, StdLib};
use poise::serenity_prelude as serenity;
use rusqlite::Connection;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::mpsc::UnboundedSender;

pub async fn init(
    ctx: &serenity::Context,
    tui_tx: UnboundedSender<BotEvent>,
    config_registry: crate::types::ConfigRegistry,
    discord_state: crate::types::SharedDiscordState,
    interval_registry: crate::types::IntervalRegistry,
) -> Result<Data, Error> {
    let _ = tui_tx.send(BotEvent::Log(LogLevel::Info, "--- Engine Initialization ---".into()));

    // Shared handle that lets interval tasks acquire Arc<Mutex<Lua>> once init() is done
    let lua_holder: Arc<OnceLock<Arc<Mutex<Lua>>>> = Arc::new(OnceLock::new());

    let libs = StdLib::ALL_SAFE | StdLib::DEBUG;
    let lua = unsafe { Lua::unsafe_new_with(libs, LuaOptions::default()) };

    // DATABASE
    let conn = Connection::open("navi.db").expect("Failed to open DB");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT)",
        (),
    )
    .expect("Failed to create DB table");
    let db = Arc::new(Mutex::new(conn));

    // CREATE 'navi' TABLE
    let navi = lua.create_table()?;
    lua.globals().set("navi", navi.clone())?;

    logging::register(&lua, &navi, tui_tx.clone())?;
    db::register(&lua, &navi, db.clone())?;
    messaging::register(&lua, &navi, ctx, tui_tx.clone())?;
    discord::register(&lua, &navi, ctx, tui_tx.clone(), discord_state.clone())?;
    config::register(&lua, &navi, db.clone(), config_registry.clone(), tui_tx.clone())?;
    intervals::register(&lua, &navi, lua_holder.clone(), interval_registry.clone(), tui_tx.clone())?;
    registry::register(&lua, &navi)?;
    perms::init(&lua, &navi, db.clone(), config_registry.clone(), discord_state.clone(), tui_tx.clone())?;

    let (load_level, load_report) = load_plugins(&lua, &interval_registry);
    let _ = tui_tx.send(BotEvent::Log(load_level, load_report));

    drop(navi); // VERY IMPORTANT: drop before creating Data to avoid deadlock
    let lua_arc = Arc::new(Mutex::new(lua));
    let _ = lua_holder.set(lua_arc.clone()); // unblocks any interval tasks that have already ticked

    Ok(Data {
        lua: lua_arc,
        tui_tx,
        discord_state,
        interval_registry,
    })
}
