use crate::types::{AdminCommand, BotEvent, ConfigRegistry, SharedDiscordState, ConfigType};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
    Terminal,
};
use std::{io, time::Duration};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(PartialEq)]
enum AppMode {
    Logs,
    Config,
}

#[derive(PartialEq)]
enum ConfigPane {
    PluginList,
    FieldList,
}

pub fn run(
    tx_to_bot: UnboundedSender<AdminCommand>,
    mut rx_from_bot: UnboundedReceiver<BotEvent>,
    config_registry: ConfigRegistry,
    discord_state: SharedDiscordState
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
    
    // New State for Config Dashboard
    let _mode = AppMode::Logs;
    let _selected_plugin_index = 0;

    // New State for Config Dashboard
    let mut mode = AppMode::Logs;
    let mut config_pane = ConfigPane::PluginList; // Left or Right side
    let mut selected_plugin_index = 0;
    let mut selected_field_index = 0;
    
    // Typing State
    let mut is_editing = false;
    let mut edit_buffer = String::new();

    // DROPDOWN STATE 
    let mut is_dropdown_open = false;
    let mut dropdown_selected_index = 0;

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
            let size = f.area();

            if mode == AppMode::Logs {
                // --- 1. LOGS VIEW (Original) ---
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Min(1),    // Logs (Grow to fill)
                        Constraint::Length(3), // Input Bar
                    ].as_ref())
                    .split(size);

                let log_items: Vec<ListItem> = logs
                    .iter()
                    .rev() // Show newest at bottom
                    .map(|msg| ListItem::new(Line::from(Span::raw(msg))))
                    .collect();
                
                let logs_widget = List::new(log_items)
                    .block(Block::default().borders(Borders::ALL).title(" Navi Logs "));
                f.render_widget(logs_widget, chunks[0]);

                let input_text = if input_mode {
                    format!("> {}_", input_buffer)
                } else {
                    String::from("Press 'q' to quit | 'r' to reload | 'i' to type | 'c' for Config")
                };

                let input_widget = Paragraph::new(input_text)
                    .style(if input_mode { Style::default().fg(Color::Yellow) } else { Style::default() })
                    .block(Block::default().borders(Borders::ALL).title(" Controls "));
                f.render_widget(input_widget, chunks[1]);

            } else if mode == AppMode::Config {
                // --- 2. CONFIG VIEW (New) ---
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                    .split(size);

                let registry = config_registry.lock().unwrap();
                let mut plugin_names: Vec<String> = registry.keys().cloned().collect();
                plugin_names.sort();

                if plugin_names.is_empty() {
                    let empty_msg = Paragraph::new("No plugins have registered configs yet.\nPress 'l' to return to logs.")
                        .block(Block::default().title("⚙️ Settings").borders(Borders::ALL));
                    f.render_widget(empty_msg, chunks[0]);
                    return;
                }
                
                if selected_plugin_index >= plugin_names.len() {
                    selected_plugin_index = plugin_names.len().saturating_sub(1);
                }

                // Left Pane: Plugin List
                let items: Vec<ListItem> = plugin_names
                    .iter()
                    .enumerate()
                    .map(|(i, name)| {
                        if i == selected_plugin_index {
                            ListItem::new(format!("> {}", name))
                                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                        } else {
                            ListItem::new(name.clone())
                        }
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default().title("🔌 Plugins (Up/Down) ").borders(Borders::ALL));
                f.render_widget(list, chunks[0]);


                // Right Pane: Config Fields
                let active_plugin = &plugin_names[selected_plugin_index];
                if let Some(schema) = registry.get(active_plugin) {
                    let mut text = vec![
                        Line::from(Span::styled(
                            format!("Configuration for: {}", active_plugin),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                    ];

                    // Prevent field index out of bounds
                    if selected_field_index >= schema.fields.len() {
                        selected_field_index = schema.fields.len().saturating_sub(1);
                    }

                    for (i, field) in schema.fields.iter().enumerate() {
                        let name_style = if config_pane == ConfigPane::FieldList && i == selected_field_index {
                            Style::default().fg(Color::Black).bg(Color::Yellow) 
                        } else {
                            Style::default().fg(Color::Yellow)
                        };
                        
                        text.push(Line::from(Span::styled(&field.name, name_style)));
                        text.push(Line::from(Span::styled(&field.description, Style::default().fg(Color::DarkGray))));
                        
                        // Show the typing buffer if editing, otherwise show the current value
                        if is_editing && i == selected_field_index {
                            text.push(Line::from(Span::styled(
                                format!(" [ {}_ ]", edit_buffer), 
                                Style::default().fg(Color::White).bg(Color::DarkGray)
                            )));
                        } else {
                            text.push(Line::from(format!(" [ {} ]", field.default_value)));
                        }
                        
                        text.push(Line::from("")); 
                    }

                    text.push(Line::from(Span::styled("Press 'l' to return to logs.", Style::default().fg(Color::DarkGray))));

                    // --- THIS IS WHERE `text` IS MOVED ---
                    let right_pane = Paragraph::new(text)
                        .block(Block::default()
                            .title(if config_pane == ConfigPane::FieldList { "⚙️ Settings (Editing)" } else { "⚙️ Settings" })
                            .borders(Borders::ALL)
                            .border_style(if config_pane == ConfigPane::FieldList { Style::default().fg(Color::Green) } else { Style::default() })
                        );
                    f.render_widget(right_pane, chunks[1]);

                    // --- DROPDOWN POPUP OVERLAY ---
                    if is_dropdown_open {
                        // 1. Calculate a centered box inside the right pane
                        let v_chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Percentage(20), Constraint::Percentage(60), Constraint::Percentage(20)])
                            .split(chunks[1]);
                        let popup_area = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(10), Constraint::Percentage(80), Constraint::Percentage(10)])
                            .split(v_chunks[1])[1];

                        // 2. Clear the background so text doesn't overlap
                        f.render_widget(Clear, popup_area);

                        // 3. Draw the list of channels
                        let state = discord_state.lock().unwrap();
                        
                        // Prevent index out of bounds
                        let safe_index = dropdown_selected_index.min(state.channels.len().saturating_sub(1));
                        
                        let items: Vec<ListItem> = state.channels.iter().enumerate().map(|(i, (id, name))| {
                            if i == safe_index {
                                ListItem::new(format!("> #{} ({})", name, id))
                                    .style(Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD))
                            } else {
                                ListItem::new(format!("  #{}", name))
                            }
                        }).collect();

                        let list = List::new(items)
                            .block(Block::default()
                                .title(" 📚 Select Discord Channel (Up/Down/Enter) ")
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Cyan)));
                        f.render_widget(list, popup_area);
                    }
                }
            }
        })?;

        // C. HANDLE KEYS
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if input_mode {
                    match key.code {
                        KeyCode::Enter => {
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
                        // --- GLOBAL HOTKEYS ---
                        KeyCode::Char('q') if !is_editing && !is_dropdown_open => {
                            let _ = tx_to_bot.send(AdminCommand::Shutdown);
                            break; 
                        }
                        KeyCode::Char('r') if !is_editing && !is_dropdown_open => {
                            let _ = tx_to_bot.send(AdminCommand::Reload);
                        }
                        KeyCode::Char('i') if !is_editing && !is_dropdown_open => {
                            if mode == AppMode::Logs { input_mode = true; }
                        }
                        KeyCode::Char('c') if !is_editing && !is_dropdown_open => mode = AppMode::Config,
                        KeyCode::Char('l') if !is_editing && !is_dropdown_open => mode = AppMode::Logs,

                        // --- DROPDOWN NAVIGATION ---
                        KeyCode::Up if is_dropdown_open => {
                            dropdown_selected_index = dropdown_selected_index.saturating_sub(1);
                        }
                        KeyCode::Down if is_dropdown_open => {
                            let max = discord_state.lock().unwrap().channels.len().saturating_sub(1);
                            if dropdown_selected_index < max { dropdown_selected_index += 1; }
                        }
                        KeyCode::Esc if is_dropdown_open => {
                            is_dropdown_open = false;
                        }

                        // --- CONFIG DASHBOARD NAVIGATION ---
                        KeyCode::Right | KeyCode::Enter if mode == AppMode::Config && config_pane == ConfigPane::PluginList && !is_dropdown_open => {
                            config_pane = ConfigPane::FieldList;
                            selected_field_index = 0;
                        }
                        KeyCode::Left | KeyCode::Esc if mode == AppMode::Config && config_pane == ConfigPane::FieldList && !is_editing && !is_dropdown_open => {
                            config_pane = ConfigPane::PluginList;
                        }
                        
                        // Navigating Left Pane
                        KeyCode::Up if mode == AppMode::Config && config_pane == ConfigPane::PluginList && !is_dropdown_open => {
                            selected_plugin_index = selected_plugin_index.saturating_sub(1);
                        }
                        KeyCode::Down if mode == AppMode::Config && config_pane == ConfigPane::PluginList && !is_dropdown_open => {
                            let max = config_registry.lock().unwrap().len().saturating_sub(1);
                            if selected_plugin_index < max { selected_plugin_index += 1; }
                        }

                        // Navigating Right Pane
                        KeyCode::Up if mode == AppMode::Config && config_pane == ConfigPane::FieldList && !is_editing && !is_dropdown_open => {
                            selected_field_index = selected_field_index.saturating_sub(1);
                        }
                        KeyCode::Down if mode == AppMode::Config && config_pane == ConfigPane::FieldList && !is_editing && !is_dropdown_open => {
                            let registry = config_registry.lock().unwrap();
                            let mut names: Vec<String> = registry.keys().cloned().collect();
                            names.sort();
                            if let Some(name) = names.get(selected_plugin_index) {
                                if let Some(schema) = registry.get(name) {
                                    let max = schema.fields.len().saturating_sub(1);
                                    if selected_field_index < max { selected_field_index += 1; }
                                }
                            }
                        }

                        // --- EDITING A FIELD OR SELECTING DROPDOWN ---
                        KeyCode::Enter if mode == AppMode::Config && config_pane == ConfigPane::FieldList => {
                            if is_dropdown_open {
                                // 1. Save Dropdown Selection
                                let state = discord_state.lock().unwrap();
                                if let Some((id, _name)) = state.channels.get(dropdown_selected_index) {
                                    let registry = config_registry.lock().unwrap();
                                    let mut names: Vec<String> = registry.keys().cloned().collect();
                                    names.sort();
                                    if let Some(plugin_name) = names.get(selected_plugin_index) {
                                        if let Some(schema) = registry.get(plugin_name) {
                                            if let Some(field) = schema.fields.get(selected_field_index) {
                                                let _ = tx_to_bot.send(AdminCommand::SaveConfig {
                                                    plugin: plugin_name.clone(),
                                                    key: field.key.clone(),
                                                    value: id.clone(),
                                                });
                                            }
                                        }
                                    }
                                }
                                is_dropdown_open = false;
                            } 
                            else if is_editing {
                                // 2. Save Text Box
                                let registry = config_registry.lock().unwrap();
                                let mut names: Vec<String> = registry.keys().cloned().collect();
                                names.sort();
                                if let Some(plugin_name) = names.get(selected_plugin_index) {
                                    if let Some(schema) = registry.get(plugin_name) {
                                        if let Some(field) = schema.fields.get(selected_field_index) {
                                            let _ = tx_to_bot.send(AdminCommand::SaveConfig {
                                                plugin: plugin_name.clone(),
                                                key: field.key.clone(),
                                                value: edit_buffer.clone(),
                                            });
                                        }
                                    }
                                }
                                is_editing = false;
                                edit_buffer.clear();
                            } 
                            else {
                                // 3. Open Editor or Dropdown
                                let registry = config_registry.lock().unwrap();
                                let mut names: Vec<String> = registry.keys().cloned().collect();
                                names.sort();
                                if let Some(plugin_name) = names.get(selected_plugin_index) {
                                    if let Some(schema) = registry.get(plugin_name) {
                                        if let Some(field) = schema.fields.get(selected_field_index) {
                                            if field.field_type == ConfigType::Channel {
                                                is_dropdown_open = true;
                                                dropdown_selected_index = 0;
                                            } else {
                                                is_editing = true;
                                                edit_buffer = field.default_value.clone();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // --- TYPING INSIDE THE TEXT BOX ---
                        KeyCode::Esc if is_editing => {
                            is_editing = false;
                            edit_buffer.clear();
                        }
                        KeyCode::Backspace if is_editing => { edit_buffer.pop(); }
                        KeyCode::Char(c) if is_editing => { edit_buffer.push(c); }
                        
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