use crate::types::{AdminCommand, BotEvent};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{io, time::Duration};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub fn run(
    tx_to_bot: UnboundedSender<AdminCommand>,
    mut rx_from_bot: UnboundedReceiver<BotEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    
    // 1. SETUP TERMINAL
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. STATE
    let mut logs: Vec<String> = Vec::new();
    let mut input_mode = false;
    let mut input_buffer = String::new();

    // 3. MAIN LOOP
    loop {
        // A. Handle Bot Events (Non-blocking)
        while let Ok(event) = rx_from_bot.try_recv() {
            match event {
                BotEvent::Log(s) => logs.push(s),
                BotEvent::UserJoined(u) => logs.push(format!("👋 User Joined: {}", u)),
                _ => {}
            }
        }
        // Keep log size manageable
        if logs.len() > 50 { logs.remove(0); }

        // B. DRAW UI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Min(1),    // Logs (Grow to fill)
                    Constraint::Length(3), // Input Bar
                ].as_ref())
                .split(f.size());

            // 1. Logs Window
            let log_items: Vec<ListItem> = logs
                .iter()
                .rev() // Show newest at bottom (standard terminal feel)
                .map(|msg| ListItem::new(Line::from(Span::raw(msg))))
                .collect();
            
            let logs_widget = List::new(log_items)
                .block(Block::default().borders(Borders::ALL).title(" Navi Logs "));
            f.render_widget(logs_widget, chunks[0]);

            // 2. Input/Status Window
            let input_text = if input_mode {
                format!("> {}_", input_buffer)
            } else {
                String::from("Press 'q' to quit, 'r' to reload, 'i' to type command")
            };

            let input_widget = Paragraph::new(input_text)
                .style(if input_mode { Style::default().fg(Color::Yellow) } else { Style::default() })
                .block(Block::default().borders(Borders::ALL).title(" Controls "));
            f.render_widget(input_widget, chunks[1]);
        })?;

        // C. HANDLE KEYS
        // Poll for 100ms so we don't burn 100% CPU
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if input_mode {
                    match key.code {
                        KeyCode::Enter => {
                            // Example: Send raw input as log for now (or implement chat command)
                            logs.push(format!("You typed: {}", input_buffer));
                            input_buffer.clear();
                            input_mode = false;
                        }
                        KeyCode::Char(c) => input_buffer.push(c),
                        KeyCode::Backspace => { input_buffer.pop(); },
                        KeyCode::Esc => input_mode = false,
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => {
                            // Quit
                            let _ = tx_to_bot.send(AdminCommand::Shutdown);
                            break; 
                        }
                        KeyCode::Char('r') => {
                            let _ = tx_to_bot.send(AdminCommand::Reload);
                        }
                        KeyCode::Char('i') => input_mode = true,
                        _ => {}
                    }
                }
            }
        }
    }

    // 4. CLEANUP
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}