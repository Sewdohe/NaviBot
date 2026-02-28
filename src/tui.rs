mod draw;
mod input;
mod state;

use state::AppState;
use crate::types::{AdminCommand, BotEvent, ConfigRegistry, SharedDiscordState};
use crossterm::{
    event::{self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub fn run(
    tx_to_bot: UnboundedSender<AdminCommand>,
    mut rx_from_bot: UnboundedReceiver<BotEvent>,
    config_registry: ConfigRegistry,
    discord_state: SharedDiscordState,
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut state = AppState::default();

    loop {
        while let Ok(event) = rx_from_bot.try_recv() {
            match event {
                BotEvent::Log(level, s) => state.logs.push((level, s)),
                BotEvent::PluginList(entries) => {
                    let count = entries.len();
                    state.plugin_browser_entries = entries;
                    state.plugin_browser_loading = false;
                    state.plugin_browser_status = format!("{} plugins available", count);
                }
                BotEvent::PluginInstalled(id) => {
                    state.plugin_browser_status = format!("✓ {} installed — press 'r' to reload", id);
                    state.plugin_browser_loading = false;
                }
            }
        }
        if state.logs.len() > 200 { state.logs.remove(0); }

        terminal.draw(|f| draw::draw(f, &mut state, &config_registry, &discord_state))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if input::handle_input(key.code, &mut state, &tx_to_bot, &config_registry, &discord_state) {
                        break;
                    }
                }
                Event::Paste(text) => {
                    if state.is_editing {
                        state.edit_buffer.push_str(&text);
                    } else if state.input_mode {
                        state.input_buffer.push_str(&text);
                    } else if state.item_subfield_editing {
                        state.item_subfield_buffer.push_str(&text);
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture, DisableBracketedPaste)?;
    terminal.show_cursor()?;
    Ok(())
}
