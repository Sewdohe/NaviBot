use mlua::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Duration;
use crate::types::{BotEvent, LogLevel, IntervalRegistry};

pub(super) static NEXT_INTERVAL_ID: AtomicU64 = AtomicU64::new(1);

pub fn register(
    lua: &Lua,
    navi: &LuaTable,
    lua_holder: Arc<OnceLock<Arc<Mutex<Lua>>>>,
    interval_registry: IntervalRegistry,
    tui_tx: UnboundedSender<BotEvent>,
) -> LuaResult<()> {
    let lua_holder_si = lua_holder.clone();
    let interval_reg_si = interval_registry.clone();
    let tui_tx_si = tui_tx.clone();

    navi.set("set_interval", lua.create_function(move |lua_ctx, (func, amount, unit): (mlua::Function, u64, Option<String>)| {
        let ms: u64 = match unit.as_deref().unwrap_or("ms") {
            "s" | "seconds" => amount * 1_000,
            "m" | "minutes" => amount * 60_000,
            "h" | "hours"   => amount * 3_600_000,
            "d" | "days"    => amount * 86_400_000,
            _               => amount, // "ms" or unrecognized → raw ms
        };

        let key = Arc::new(lua_ctx.create_registry_value(func)?);
        let id = NEXT_INTERVAL_ID.fetch_add(1, Ordering::SeqCst);
        let (cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);
        interval_reg_si.lock().unwrap().insert(id, cancel_tx);

        let holder = lua_holder_si.clone();
        let reg    = interval_reg_si.clone();
        let tx     = tui_tx_si.clone();

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(ms.max(1)));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            ticker.tick().await; // skip immediate first tick so init() always finishes first

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if *cancel_rx.borrow() { break; }
                        if let Some(lua_arc) = holder.get() {
                            let lua_arc = lua_arc.clone();
                            let key2   = key.clone();
                            let tx2    = tx.clone();
                            let still_valid = tokio::task::block_in_place(move || {
                                let lua = lua_arc.lock().unwrap();
                                let result = match lua.registry_value::<mlua::Function>(key2.as_ref()) {
                                    Ok(f) => {
                                        if let Err(e) = f.call::<_, ()>(()) {
                                            let _ = tx2.send(BotEvent::Log(LogLevel::Error,
                                                format!("[interval {}] {}", id, e)));
                                        }
                                        true
                                    }
                                    Err(_) => false, // key invalidated after reload
                                };
                                result
                            });
                            if !still_valid { break; }
                        }
                    }
                    result = cancel_rx.changed() => {
                        if result.is_err() || *cancel_rx.borrow() { break; }
                    }
                }
            }
            reg.lock().unwrap().remove(&id);
        });

        Ok(id)
    })?)?;

    let interval_reg_ci = interval_registry.clone();
    navi.set("clear_interval", lua.create_function(move |_, id: u64| {
        if let Some(cancel_tx) = interval_reg_ci.lock().unwrap().remove(&id) {
            let _ = cancel_tx.send(true);
        }
        Ok(())
    })?)?;

    Ok(())
}
