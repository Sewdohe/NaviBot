mod draw;
mod input;
mod state;

use state::AppState;
use crate::types::{AdminCommand, BotEvent, ConfigRegistry, SharedDiscordState};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut state = AppState::default();

    loop {
        while let Ok(event) = rx_from_bot.try_recv() {
            match event {
                BotEvent::Log(level, s) => state.logs.push((level, s)),
            }
        }
        if state.logs.len() > 200 { state.logs.remove(0); }

        terminal.draw(|f| draw::draw(f, &mut state, &config_registry, &discord_state))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if input::handle_input(key.code, &mut state, &tx_to_bot, &config_registry, &discord_state) {
                        break;
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
