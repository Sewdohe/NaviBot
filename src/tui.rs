use crate::types::{AdminCommand, BotEvent, ConfigRegistry, ConfigType, LogLevel, SharedDiscordState};
use std::collections::HashMap;
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
    ListManager,
    ItemEditor,
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
    let mut logs: Vec<(LogLevel, String)> = Vec::new();
    let mut log_scroll: u16 = 0;
    let mut log_auto_scroll = true;  // pinned to bottom until user scrolls up
    let mut log_scroll_max: u16 = 0; // updated each frame, used by key handler
    let mut input_mode = false;
    let mut input_buffer = String::new();

    // State for Config Dashboard
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

    // LIST MANAGER STATE
    let mut selected_list_item_index: usize = 0;

    // ITEM EDITOR STATE
    let mut item_edit_buffer: HashMap<String, String> = HashMap::new();
    let mut item_edit_field_index: usize = 0;
    let mut item_editing_index: Option<usize> = None; // None = new, Some(i) = editing existing
    let mut item_subfield_editing = false;
    let mut item_subfield_buffer = String::new();
    let mut item_dropdown_open = false;
    let mut item_dropdown_index: usize = 0;

    // 3. MAIN LOOP
    loop {
        // A. Handle Bot Events (Non-blocking)
        while let Ok(event) = rx_from_bot.try_recv() {
            match event {
                BotEvent::Log(level, s) => logs.push((level, s)),
            }
        }
        // Keep log size manageable
        if logs.len() > 200 { logs.remove(0); }

        // B. DRAW UI
        terminal.draw(|f| {
            let size = f.area();

            // --- 1. THE GLOBAL HEADER ---
            // Slice off the top 1 row of the entire screen
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Top row for header
                    Constraint::Min(0),    // The rest for the app
                ].as_ref())
                .split(size);

            // Draw the slick header bar into that top row
            let header = Paragraph::new(Line::from(vec![
                Span::styled(" 🤖 Navi Engine v1.0 ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" | "),
                Span::styled("🟢 System Online", Style::default().fg(Color::Green)),
            ]));
            f.render_widget(header, main_chunks[0]);

            // --- 2. ROUTE THE REST OF THE APP ---
            if mode == AppMode::Logs {
                // --- LOGS VIEW ---
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Min(1),    // Logs (Grow to fill)
                        Constraint::Length(3), // Input Bar
                    ].as_ref())
                    .split(main_chunks[1]); // Uses the space below the header

                // 1. Map them directly to `Line` objects instead of `ListItem`
                let plugin_style = Style::default().fg(Color::Magenta);
                let log_lines: Vec<Line> = logs.iter().map(|(level, msg)| {
                        let (prefix, style) = match level {
                            LogLevel::Error => ("[E]", Style::default().fg(Color::Red)),
                            LogLevel::Warn  => ("[W]", Style::default().fg(Color::Yellow)),
                            LogLevel::Info  => ("[I]", Style::default().fg(Color::Cyan)),
                        };
                        // If the message starts with [plugin-name], colour that part purple
                        let spans = if msg.starts_with('[') {
                            if let Some(close) = msg.find(']') {
                                let tag  = &msg[..=close];           // "[counting.lua]"
                                let rest = msg[close+1..].trim_start();
                                vec![
                                    Span::styled(prefix, style),
                                    Span::raw(" "),
                                    Span::styled(tag.to_string(), plugin_style),
                                    Span::raw(" "),
                                    Span::styled(rest.to_string(), style),
                                ]
                            } else {
                                vec![Span::styled(prefix, style), Span::raw(" "), Span::styled(msg.clone(), style)]
                            }
                        } else {
                            vec![Span::styled(prefix, style), Span::raw(" "), Span::styled(msg.clone(), style)]
                        };
                        Line::from(spans)
                    }).collect();
                
                // Calculate scroll bounds
                let visible_height = chunks[0].height.saturating_sub(2);
                let max_scroll = (log_lines.len() as u16).saturating_sub(visible_height);
                log_scroll_max = max_scroll;
                // If pinned to bottom, track new messages automatically
                if log_auto_scroll { log_scroll = max_scroll; }
                let scroll_offset = log_scroll.min(max_scroll);

                // 2. Use a Paragraph instead of a List
                let logs_pane = Paragraph::new(log_lines)
                        .block(Block::default()
                            .title(" Navi Logs ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::White))
                        )
                        // 3. Turn on word wrapping!
                        .wrap(ratatui::widgets::Wrap { trim: true }) 
                        // 4. Safely scroll using our math above
                        .scroll((scroll_offset, 0)); 
                        
                f.render_widget(logs_pane, chunks[0]);

                let input_text = if input_mode {
                    format!("> {}_", input_buffer)
                } else {
                    String::from("Up/Down: scroll logs | 'q' quit | 'r' reload | 'u' re-cache | 'i' type | 'c' config")
                };

                let input_widget = Paragraph::new(input_text)
                    .style(if input_mode { Style::default().fg(Color::Yellow) } else { Style::default() })
                    .block(Block::default().borders(Borders::ALL).title(" Controls "));
                f.render_widget(input_widget, chunks[1]);

            } else if mode == AppMode::Config {
                // --- CONFIG VIEW ---
                let vertical_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(3)])
                        .split(main_chunks[1]); // Uses the space below the header
                
                let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                        .split(vertical_chunks[0]); 

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
                let items: Vec<ListItem> = plugin_names.iter().enumerate().map(|(i, name)| {
                        if i == selected_plugin_index {
                            ListItem::new(format!("> {}", name)).style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                        } else {
                            ListItem::new(format!("  {}", name))
                        }
                    }).collect();

                // --- CHECK IF LEFT PANE IS ACTIVE ---
                let left_border_style = if config_pane == ConfigPane::PluginList { 
                    Style::default().fg(Color::Green) 
                } else { 
                    Style::default() 
                };

                let left_pane = List::new(items)
                        .block(Block::default()
                            .title(" 🔌 Plugins (Up/Down) ")
                            .borders(Borders::ALL)
                            .border_style(left_border_style)
                        );
                    f.render_widget(left_pane, chunks[0]);


                // Right Pane: Config Fields / List Manager / Item Editor
                let active_plugin = &plugin_names[selected_plugin_index];
                if let Some(schema) = registry.get(active_plugin) {
                    // Prevent field index out of bounds
                    if selected_field_index >= schema.fields.len() {
                        selected_field_index = schema.fields.len().saturating_sub(1);
                    }

                    if config_pane == ConfigPane::ListManager {
                        if let Some(list_field) = schema.fields.get(selected_field_index) {
                            // Clamp list item selection
                            if !list_field.list_items.is_empty()
                                && selected_list_item_index >= list_field.list_items.len()
                            {
                                selected_list_item_index =
                                    list_field.list_items.len().saturating_sub(1);
                            }

                            let mut text = vec![Line::from("")];

                            if list_field.list_items.is_empty() {
                                text.push(Line::from(Span::styled(
                                    "  (no items yet — press 'n' to add one)",
                                    Style::default().fg(Color::DarkGray),
                                )));
                            } else {
                                for (i, item) in list_field.list_items.iter().enumerate() {
                                    let summary: String = list_field
                                        .item_schema
                                        .iter()
                                        .map(|s| {
                                            format!(
                                                "{}={}",
                                                s.key,
                                                item.get(&s.key).map(|v| v.as_str()).unwrap_or("")
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                        .join("  ");

                                    let line = if i == selected_list_item_index {
                                        Line::from(Span::styled(
                                            format!(" > #{} {}", i + 1, summary),
                                            Style::default()
                                                .fg(Color::Black)
                                                .bg(Color::Yellow)
                                                .add_modifier(Modifier::BOLD),
                                        ))
                                    } else {
                                        Line::from(format!("   #{} {}", i + 1, summary))
                                    };
                                    text.push(line);
                                }
                            }

                            text.push(Line::from(""));
                            text.push(Line::from(Span::styled(
                                "n=add  d=delete  Enter=edit  Esc=back",
                                Style::default().fg(Color::DarkGray),
                            )));

                            let right_pane = Paragraph::new(text).block(
                                Block::default()
                                    .title(format!(
                                        " {} ({} items) ",
                                        list_field.name,
                                        list_field.list_items.len()
                                    ))
                                    .borders(Borders::ALL)
                                    .border_style(Style::default().fg(Color::Green)),
                            );
                            f.render_widget(right_pane, chunks[1]);
                        }
                    } else if config_pane == ConfigPane::ItemEditor {
                        if let Some(list_field) = schema.fields.get(selected_field_index) {
                            // Clamp sub-field selection
                            if !list_field.item_schema.is_empty()
                                && item_edit_field_index >= list_field.item_schema.len()
                            {
                                item_edit_field_index =
                                    list_field.item_schema.len().saturating_sub(1);
                            }

                            let title = match item_editing_index {
                                Some(i) => format!(" Editing item #{} ", i + 1),
                                None => " New Item ".to_string(),
                            };

                            let mut text = vec![Line::from("")];

                            for (i, sub_field) in list_field.item_schema.iter().enumerate() {
                                let name_style = if i == item_edit_field_index {
                                    Style::default().fg(Color::Black).bg(Color::Yellow)
                                } else {
                                    Style::default().fg(Color::Yellow)
                                };

                                text.push(Line::from(Span::styled(&sub_field.name, name_style)));

                                let current_val = item_edit_buffer
                                    .get(&sub_field.key)
                                    .cloned()
                                    .unwrap_or_default();

                                if item_subfield_editing && i == item_edit_field_index {
                                    text.push(Line::from(Span::styled(
                                        format!(" [ {}_ ]", item_subfield_buffer),
                                        Style::default().fg(Color::White).bg(Color::DarkGray),
                                    )));
                                } else {
                                    text.push(Line::from(format!(" [ {} ]", current_val)));
                                }

                                text.push(Line::from(""));
                            }

                            text.push(Line::from(Span::styled(
                                "s=save  Esc=cancel",
                                Style::default().fg(Color::DarkGray),
                            )));

                            let right_pane = Paragraph::new(text).block(
                                Block::default()
                                    .title(title)
                                    .borders(Borders::ALL)
                                    .border_style(Style::default().fg(Color::Cyan)),
                            );
                            f.render_widget(right_pane, chunks[1]);

                            // Item sub-field dropdown popup
                            if item_dropdown_open {
                                let v_chunks = Layout::default()
                                    .direction(Direction::Vertical)
                                    .constraints([
                                        Constraint::Percentage(20),
                                        Constraint::Percentage(60),
                                        Constraint::Percentage(20),
                                    ])
                                    .split(chunks[1]);
                                let popup_area = Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([
                                        Constraint::Percentage(10),
                                        Constraint::Percentage(80),
                                        Constraint::Percentage(10),
                                    ])
                                    .split(v_chunks[1])[1];

                                f.render_widget(Clear, popup_area);

                                let state = discord_state.lock().unwrap();
                                let sub_field_type = list_field
                                    .item_schema
                                    .get(item_edit_field_index)
                                    .map(|s| s.field_type.clone())
                                    .unwrap_or(ConfigType::String);

                                let (list_title, list_len) = match sub_field_type {
                                    ConfigType::Channel => {
                                        (" 📚 Select Channel ", state.channels.len())
                                    }
                                    ConfigType::Category => {
                                        (" 📁 Select Category ", state.categories.len())
                                    }
                                    ConfigType::Role => (" 🎭 Select Role ", state.roles.len()),
                                    _ => (" Select ", 0),
                                };

                                let safe_idx =
                                    item_dropdown_index.min(list_len.saturating_sub(1));

                                let popup_items: Vec<ListItem> = match sub_field_type {
                                    ConfigType::Channel => state
                                        .channels
                                        .iter()
                                        .enumerate()
                                        .map(|(i, (id, name))| {
                                            if i == safe_idx {
                                                ListItem::new(format!("> #{} ({})", name, id))
                                                    .style(Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD))
                                            } else {
                                                ListItem::new(format!("  #{}", name))
                                            }
                                        })
                                        .collect(),
                                    ConfigType::Category => state
                                        .categories
                                        .iter()
                                        .enumerate()
                                        .map(|(i, (id, name))| {
                                            if i == safe_idx {
                                                ListItem::new(format!("> 📁 {} ({})", name, id))
                                                    .style(Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD))
                                            } else {
                                                ListItem::new(format!("  📁 {}", name))
                                            }
                                        })
                                        .collect(),
                                    ConfigType::Role => state
                                        .roles
                                        .iter()
                                        .enumerate()
                                        .map(|(i, role)| {
                                            let dc = if role.color == (0, 0, 0) {
                                                Color::White
                                            } else {
                                                Color::Rgb(role.color.0, role.color.1, role.color.2)
                                            };
                                            let mut style = Style::default().fg(dc);
                                            if i == safe_idx {
                                                style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                                                ListItem::new(format!("> @{} ({})", role.name, role.id)).style(style)
                                            } else {
                                                ListItem::new(format!("  @{}", role.name)).style(style)
                                            }
                                        })
                                        .collect(),
                                    _ => vec![],
                                };

                                let list = List::new(popup_items).block(
                                    Block::default()
                                        .title(format!("{} (Up/Down/Enter) ", list_title))
                                        .borders(Borders::ALL)
                                        .border_style(Style::default().fg(Color::Cyan)),
                                );
                                f.render_widget(list, popup_area);
                            }
                        }
                    } else {
                        // --- FIELD LIST RENDERING ---
                        let mut text = vec![
                            Line::from(Span::styled(
                                format!("Configuration for: {}", active_plugin),
                                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                            )),
                            Line::from(""),
                        ];

                        for (i, field) in schema.fields.iter().enumerate() {
                            let name_style = if config_pane == ConfigPane::FieldList
                                && i == selected_field_index
                            {
                                Style::default().fg(Color::Black).bg(Color::Yellow)
                            } else {
                                Style::default().fg(Color::Yellow)
                            };

                            text.push(Line::from(Span::styled(&field.name, name_style)));
                            text.push(Line::from(Span::styled(
                                &field.description,
                                Style::default().fg(Color::DarkGray),
                            )));

                            if field.field_type == ConfigType::List {
                                text.push(Line::from(format!(
                                    " [ {} items ]",
                                    field.list_items.len()
                                )));
                            } else if is_editing && i == selected_field_index {
                                text.push(Line::from(Span::styled(
                                    format!(" [ {}_ ]", edit_buffer),
                                    Style::default().fg(Color::White).bg(Color::DarkGray),
                                )));
                            } else {
                                text.push(Line::from(format!(" [ {} ]", field.default_value)));
                            }

                            text.push(Line::from(""));
                        }

                        text.push(Line::from(Span::styled(
                            "Press 'l' to return to logs.",
                            Style::default().fg(Color::DarkGray),
                        )));

                        let right_pane = Paragraph::new(text).block(
                            Block::default()
                                .title(if config_pane == ConfigPane::FieldList {
                                    "⚙️ Settings (Editing)"
                                } else {
                                    "⚙️ Settings"
                                })
                                .borders(Borders::ALL)
                                .border_style(if config_pane == ConfigPane::FieldList {
                                    Style::default().fg(Color::Green)
                                } else {
                                    Style::default()
                                }),
                        );
                        f.render_widget(right_pane, chunks[1]);

                        // --- DROPDOWN POPUP OVERLAY ---
                        if is_dropdown_open {
                            let v_chunks = Layout::default()
                                .direction(Direction::Vertical)
                                .constraints([
                                    Constraint::Percentage(20),
                                    Constraint::Percentage(60),
                                    Constraint::Percentage(20),
                                ])
                                .split(chunks[1]);
                            let popup_area = Layout::default()
                                .direction(Direction::Horizontal)
                                .constraints([
                                    Constraint::Percentage(10),
                                    Constraint::Percentage(80),
                                    Constraint::Percentage(10),
                                ])
                                .split(v_chunks[1])[1];

                            f.render_widget(Clear, popup_area);

                            let state = discord_state.lock().unwrap();
                            let active_field_type =
                                schema.fields[selected_field_index].field_type.clone();

                            let (list_title, list_len) = match active_field_type {
                                ConfigType::Channel => {
                                    (" 📚 Select Channel ", state.channels.len())
                                }
                                ConfigType::Category => {
                                    (" 📁 Select Category ", state.categories.len())
                                }
                                ConfigType::Role => (" 🎭 Select Role ", state.roles.len()),
                                _ => (" Select ", 0),
                            };

                            let safe_index =
                                dropdown_selected_index.min(list_len.saturating_sub(1));

                            let items: Vec<ListItem> = match active_field_type {
                                ConfigType::Channel => state
                                    .channels
                                    .iter()
                                    .enumerate()
                                    .map(|(i, (id, name))| {
                                        if i == safe_index {
                                            ListItem::new(format!("> #{} ({})", name, id)).style(
                                                Style::default()
                                                    .fg(Color::Black)
                                                    .bg(Color::Cyan)
                                                    .add_modifier(Modifier::BOLD),
                                            )
                                        } else {
                                            ListItem::new(format!("  #{}", name))
                                        }
                                    })
                                    .collect(),
                                ConfigType::Category => state
                                    .categories
                                    .iter()
                                    .enumerate()
                                    .map(|(i, (id, name))| {
                                        if i == safe_index {
                                            ListItem::new(format!("> 📁 {} ({})", name, id))
                                                .style(
                                                    Style::default()
                                                        .fg(Color::Black)
                                                        .bg(Color::Yellow)
                                                        .add_modifier(Modifier::BOLD),
                                                )
                                        } else {
                                            ListItem::new(format!("  📁 {}", name))
                                        }
                                    })
                                    .collect(),
                                ConfigType::Role => state
                                    .roles
                                    .iter()
                                    .enumerate()
                                    .map(|(i, role)| {
                                        let display_color = if role.color == (0, 0, 0) {
                                            Color::White
                                        } else {
                                            Color::Rgb(role.color.0, role.color.1, role.color.2)
                                        };

                                        let mut style = Style::default().fg(display_color);
                                        if i == safe_index {
                                            style = style
                                                .bg(Color::DarkGray)
                                                .add_modifier(Modifier::BOLD);
                                            ListItem::new(format!(
                                                "> @{} ({})",
                                                role.name, role.id
                                            ))
                                            .style(style)
                                        } else {
                                            ListItem::new(format!("  @{}", role.name)).style(style)
                                        }
                                    })
                                    .collect(),
                                _ => vec![],
                            };

                            let list = List::new(items).block(
                                Block::default()
                                    .title(format!("{} (Up/Down/Enter) ", list_title))
                                    .borders(Borders::ALL)
                                    .border_style(Style::default().fg(Color::Cyan)),
                            );
                            f.render_widget(list, popup_area);
                        }
                    }
                }

                // --- DYNAMIC CONTROLS FOOTER ---
                    let help_text = if item_dropdown_open {
                        "Up/Down to scroll | Enter to confirm | Esc to cancel"
                    } else if item_subfield_editing {
                        "Type value | Enter to confirm | Esc to cancel"
                    } else if config_pane == ConfigPane::ItemEditor {
                        "Up/Down to select sub-field | Enter to edit | s=save  Esc=back"
                    } else if config_pane == ConfigPane::ListManager {
                        "Up/Down to select item | Enter=edit | n=add | d=delete | Esc=back"
                    } else if is_dropdown_open {
                        "Press 'Up/Down' to scroll | 'Enter' to confirm selection | 'Esc' to cancel"
                    } else if is_editing {
                        "Type your value | Press 'Enter' to save | 'Esc' to cancel"
                    } else if config_pane == ConfigPane::PluginList {
                        "Press 'Up/Down' to select plugin | 'Right' or 'Enter' to configure | 'l' for Logs | 'q' to quit"
                    } else {
                        "Press 'Up/Down' to select field | 'Enter' to edit | 'Left' or 'Esc' for plugins | 'l' for Logs"
                    };

                    let controls = Paragraph::new(help_text)
                        .style(Style::default().fg(Color::DarkGray))
                        .block(Block::default()
                            .title(" Controls ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::White))
                        );
                    
                    f.render_widget(controls, vertical_chunks[1]);
            }
        })?;

        // C. HANDLE KEYS
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if input_mode {
                    match key.code {
                        KeyCode::Enter => {
                            logs.push((LogLevel::Info, format!("You typed: {}", input_buffer)));
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
                        KeyCode::Char('q') if !is_editing && !is_dropdown_open && !item_subfield_editing && !item_dropdown_open => {
                            let _ = tx_to_bot.send(AdminCommand::Shutdown);
                            break;
                        }
                        KeyCode::Char('r') if !is_editing && !is_dropdown_open && !item_subfield_editing && !item_dropdown_open => {
                            let _ = tx_to_bot.send(AdminCommand::Reload);
                        }
                        KeyCode::Char('u') if !is_editing && !is_dropdown_open && !item_subfield_editing && !item_dropdown_open => {
                            let _ = tx_to_bot.send(AdminCommand::RefreshCache);
                        }
                        KeyCode::Char('i') if !is_editing && !is_dropdown_open && !item_subfield_editing && !item_dropdown_open => {
                            if mode == AppMode::Logs { input_mode = true; }
                        }
                        KeyCode::Char('c') if !is_editing && !is_dropdown_open && !item_subfield_editing && !item_dropdown_open => mode = AppMode::Config,
                        KeyCode::Char('l') if !is_editing && !is_dropdown_open && !item_subfield_editing && !item_dropdown_open => mode = AppMode::Logs,

                        // --- LOG SCROLLING ---
                        KeyCode::Up if mode == AppMode::Logs => {
                            log_auto_scroll = false;
                            log_scroll = log_scroll.saturating_sub(1);
                        }
                        KeyCode::Down if mode == AppMode::Logs => {
                            log_scroll = (log_scroll + 1).min(log_scroll_max);
                            if log_scroll >= log_scroll_max { log_auto_scroll = true; }
                        }

                        // --- DROPDOWN NAVIGATION ---
                        KeyCode::Up if is_dropdown_open => {
                            dropdown_selected_index = dropdown_selected_index.saturating_sub(1);
                        }
                        KeyCode::Down if is_dropdown_open => {
                            let max = {
                                let registry = config_registry.lock().unwrap();
                                let mut names: Vec<String> = registry.keys().cloned().collect();
                                names.sort();
                                let f_type = names.get(selected_plugin_index)
                                    .and_then(|n| registry.get(n))
                                    .and_then(|s| s.fields.get(selected_field_index))
                                    .map(|f| f.field_type.clone())
                                    .unwrap_or(ConfigType::String);
                                    
                                let state = discord_state.lock().unwrap();
                                match f_type {
                                    ConfigType::Channel => state.channels.len(),
                                    ConfigType::Category => state.categories.len(),
                                    ConfigType::Role => state.roles.len(),
                                    _ => 0
                                }.saturating_sub(1)
                            };
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
                        KeyCode::Left | KeyCode::Esc if mode == AppMode::Config && config_pane == ConfigPane::ListManager => {
                            config_pane = ConfigPane::FieldList;
                        }
                        KeyCode::Esc if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && !item_subfield_editing && !item_dropdown_open => {
                            config_pane = ConfigPane::ListManager;
                            item_edit_buffer.clear();
                            item_editing_index = None;
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

                        // Navigating ListManager
                        KeyCode::Up if mode == AppMode::Config && config_pane == ConfigPane::ListManager => {
                            selected_list_item_index = selected_list_item_index.saturating_sub(1);
                        }
                        KeyCode::Down if mode == AppMode::Config && config_pane == ConfigPane::ListManager => {
                            let registry = config_registry.lock().unwrap();
                            let mut names: Vec<String> = registry.keys().cloned().collect();
                            names.sort();
                            if let Some(name) = names.get(selected_plugin_index) {
                                if let Some(schema) = registry.get(name) {
                                    if let Some(field) = schema.fields.get(selected_field_index) {
                                        let max = field.list_items.len().saturating_sub(1);
                                        if selected_list_item_index < max { selected_list_item_index += 1; }
                                    }
                                }
                            }
                        }

                        // Navigating ItemEditor sub-fields
                        KeyCode::Up if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && !item_subfield_editing && !item_dropdown_open => {
                            item_edit_field_index = item_edit_field_index.saturating_sub(1);
                        }
                        KeyCode::Down if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && !item_subfield_editing && !item_dropdown_open => {
                            let registry = config_registry.lock().unwrap();
                            let mut names: Vec<String> = registry.keys().cloned().collect();
                            names.sort();
                            if let Some(name) = names.get(selected_plugin_index) {
                                if let Some(schema) = registry.get(name) {
                                    if let Some(field) = schema.fields.get(selected_field_index) {
                                        let max = field.item_schema.len().saturating_sub(1);
                                        if item_edit_field_index < max { item_edit_field_index += 1; }
                                    }
                                }
                            }
                        }

                        // --- LIST MANAGER KEYS ---
                        KeyCode::Char('n') if mode == AppMode::Config && config_pane == ConfigPane::ListManager => {
                            // Open ItemEditor for a new item
                            item_edit_buffer.clear();
                            item_edit_field_index = 0;
                            item_editing_index = None;
                            item_subfield_editing = false;
                            item_dropdown_open = false;
                            config_pane = ConfigPane::ItemEditor;
                        }
                        KeyCode::Enter if mode == AppMode::Config && config_pane == ConfigPane::ListManager => {
                            // Open ItemEditor for existing item
                            let registry = config_registry.lock().unwrap();
                            let mut names: Vec<String> = registry.keys().cloned().collect();
                            names.sort();
                            if let Some(name) = names.get(selected_plugin_index) {
                                if let Some(schema) = registry.get(name) {
                                    if let Some(field) = schema.fields.get(selected_field_index) {
                                        if let Some(item) = field.list_items.get(selected_list_item_index) {
                                            item_edit_buffer = item.clone();
                                            item_edit_field_index = 0;
                                            item_editing_index = Some(selected_list_item_index);
                                            item_subfield_editing = false;
                                            item_dropdown_open = false;
                                            config_pane = ConfigPane::ItemEditor;
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('d') if mode == AppMode::Config && config_pane == ConfigPane::ListManager => {
                            let plugin_name = {
                                let registry = config_registry.lock().unwrap();
                                let mut names: Vec<String> = registry.keys().cloned().collect();
                                names.sort();
                                names.get(selected_plugin_index).cloned()
                            };
                            if let Some(plugin) = plugin_name {
                                let field_key = {
                                    let registry = config_registry.lock().unwrap();
                                    registry.get(&plugin).and_then(|s| s.fields.get(selected_field_index)).map(|f| f.key.clone())
                                };
                                if let Some(key) = field_key {
                                    let _ = tx_to_bot.send(AdminCommand::DeleteListItem {
                                        plugin,
                                        key,
                                        index: selected_list_item_index,
                                    });
                                    selected_list_item_index = selected_list_item_index.saturating_sub(1);
                                }
                            }
                        }

                        // --- ITEM EDITOR KEYS ---
                        // item dropdown navigation
                        KeyCode::Up if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && item_dropdown_open => {
                            item_dropdown_index = item_dropdown_index.saturating_sub(1);
                        }
                        KeyCode::Down if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && item_dropdown_open => {
                            let registry = config_registry.lock().unwrap();
                            let mut names: Vec<String> = registry.keys().cloned().collect();
                            names.sort();
                            let sub_type = names.get(selected_plugin_index)
                                .and_then(|n| registry.get(n))
                                .and_then(|s| s.fields.get(selected_field_index))
                                .and_then(|f| f.item_schema.get(item_edit_field_index))
                                .map(|sf| sf.field_type.clone())
                                .unwrap_or(ConfigType::String);
                            let state = discord_state.lock().unwrap();
                            let max = match sub_type {
                                ConfigType::Channel => state.channels.len(),
                                ConfigType::Category => state.categories.len(),
                                ConfigType::Role => state.roles.len(),
                                _ => 0,
                            }.saturating_sub(1);
                            if item_dropdown_index < max { item_dropdown_index += 1; }
                        }
                        KeyCode::Esc if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && item_dropdown_open => {
                            item_dropdown_open = false;
                        }
                        KeyCode::Enter if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && item_dropdown_open => {
                            // Save dropdown selection into item_edit_buffer
                            let registry = config_registry.lock().unwrap();
                            let mut names: Vec<String> = registry.keys().cloned().collect();
                            names.sort();
                            if let Some(name) = names.get(selected_plugin_index) {
                                if let Some(schema) = registry.get(name) {
                                    if let Some(field) = schema.fields.get(selected_field_index) {
                                        if let Some(sub_field) = field.item_schema.get(item_edit_field_index) {
                                            let state = discord_state.lock().unwrap();
                                            let selected_id = match sub_field.field_type {
                                                ConfigType::Channel => state.channels.get(item_dropdown_index).map(|(id, _)| id.clone()),
                                                ConfigType::Category => state.categories.get(item_dropdown_index).map(|(id, _)| id.clone()),
                                                ConfigType::Role => state.roles.get(item_dropdown_index).map(|r| r.id.clone()),
                                                _ => None,
                                            };
                                            if let Some(id) = selected_id {
                                                item_edit_buffer.insert(sub_field.key.clone(), id);
                                            }
                                        }
                                    }
                                }
                            }
                            item_dropdown_open = false;
                        }
                        // typing inside item sub-field text box
                        KeyCode::Esc if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && item_subfield_editing => {
                            item_subfield_editing = false;
                            item_subfield_buffer.clear();
                        }
                        KeyCode::Backspace if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && item_subfield_editing => {
                            item_subfield_buffer.pop();
                        }
                        KeyCode::Char(c) if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && item_subfield_editing => {
                            item_subfield_buffer.push(c);
                        }
                        KeyCode::Enter if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && item_subfield_editing => {
                            // Save sub-field text box value
                            let registry = config_registry.lock().unwrap();
                            let mut names: Vec<String> = registry.keys().cloned().collect();
                            names.sort();
                            if let Some(name) = names.get(selected_plugin_index) {
                                if let Some(schema) = registry.get(name) {
                                    if let Some(field) = schema.fields.get(selected_field_index) {
                                        if let Some(sub_field) = field.item_schema.get(item_edit_field_index) {
                                            item_edit_buffer.insert(sub_field.key.clone(), item_subfield_buffer.clone());
                                        }
                                    }
                                }
                            }
                            item_subfield_editing = false;
                            item_subfield_buffer.clear();
                        }
                        // open editor/dropdown for selected sub-field
                        KeyCode::Enter if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && !item_subfield_editing && !item_dropdown_open => {
                            let registry = config_registry.lock().unwrap();
                            let mut names: Vec<String> = registry.keys().cloned().collect();
                            names.sort();
                            if let Some(name) = names.get(selected_plugin_index) {
                                if let Some(schema) = registry.get(name) {
                                    if let Some(field) = schema.fields.get(selected_field_index) {
                                        if let Some(sub_field) = field.item_schema.get(item_edit_field_index) {
                                            match sub_field.field_type {
                                                ConfigType::Channel | ConfigType::Category | ConfigType::Role => {
                                                    item_dropdown_open = true;
                                                    item_dropdown_index = 0;
                                                }
                                                ConfigType::Boolean => {
                                                    let current = item_edit_buffer.get(&sub_field.key).map(|v| v.as_str()).unwrap_or("false");
                                                    let new_val = if current == "true" { "false" } else { "true" };
                                                    item_edit_buffer.insert(sub_field.key.clone(), new_val.to_string());
                                                }
                                                _ => {
                                                    item_subfield_buffer = item_edit_buffer.get(&sub_field.key).cloned().unwrap_or_default();
                                                    item_subfield_editing = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // save the item
                        KeyCode::Char('s') if mode == AppMode::Config && config_pane == ConfigPane::ItemEditor && !item_subfield_editing => {
                            let plugin_name = {
                                let registry = config_registry.lock().unwrap();
                                let mut names: Vec<String> = registry.keys().cloned().collect();
                                names.sort();
                                names.get(selected_plugin_index).cloned()
                            };
                            if let Some(plugin) = plugin_name {
                                let field_key = {
                                    let registry = config_registry.lock().unwrap();
                                    registry.get(&plugin).and_then(|s| s.fields.get(selected_field_index)).map(|f| f.key.clone())
                                };
                                if let Some(key) = field_key {
                                    let item = item_edit_buffer.clone();
                                    match item_editing_index {
                                        None => {
                                            let _ = tx_to_bot.send(AdminCommand::AppendListItem { plugin, key, item });
                                        }
                                        Some(i) => {
                                            let _ = tx_to_bot.send(AdminCommand::UpdateListItem { plugin, key, index: i, item });
                                        }
                                    }
                                }
                            }
                            item_edit_buffer.clear();
                            item_editing_index = None;
                            config_pane = ConfigPane::ListManager;
                        }

                        // --- EDITING A FIELD OR SELECTING DROPDOWN ---
                        KeyCode::Enter if mode == AppMode::Config && config_pane == ConfigPane::FieldList => {
                            if is_dropdown_open {
                                // 1. Save Dropdown Selection (Optimistic UI Update)
                                let mut registry = config_registry.lock().unwrap();
                                let mut names: Vec<String> = registry.keys().cloned().collect();
                                names.sort();
                                
                                if let Some(plugin_name) = names.get(selected_plugin_index).cloned() {
                                    if let Some(schema) = registry.get_mut(&plugin_name) {
                                        if let Some(field) = schema.fields.get_mut(selected_field_index) {
                                            
                                            let mut selected_id = None;
                                            let state = discord_state.lock().unwrap();
                                            match field.field_type {
                                                ConfigType::Channel => { if let Some((id, _)) = state.channels.get(dropdown_selected_index) { selected_id = Some(id.clone()); } },
                                                ConfigType::Category => { if let Some((id, _)) = state.categories.get(dropdown_selected_index) { selected_id = Some(id.clone()); } },
                                                ConfigType::Role => { if let Some(role) = state.roles.get(dropdown_selected_index) { selected_id = Some(role.id.clone()); } },
                                                _ => {}
                                            }

                                            if let Some(id) = selected_id {
                                                field.default_value = id.clone(); 
                                                
                                                let _ = tx_to_bot.send(AdminCommand::SaveConfig {
                                                    plugin: plugin_name.clone(),
                                                    key: field.key.clone(),
                                                    value: id,
                                                });
                                            }
                                        }
                                    }
                                }
                                is_dropdown_open = false;
                            } 
                            else if is_editing {
                                // 2. Save Text Box (Optimistic UI Update)
                                let mut registry = config_registry.lock().unwrap();
                                let mut names: Vec<String> = registry.keys().cloned().collect();
                                names.sort();
                                
                                if let Some(plugin_name) = names.get(selected_plugin_index).cloned() {
                                    if let Some(schema) = registry.get_mut(&plugin_name) {
                                        if let Some(field) = schema.fields.get_mut(selected_field_index) {
                                            
                                            field.default_value = edit_buffer.clone(); 
                                            
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
                                // 3. Open Editor, Dropdown, List Manager, or Toggle Boolean
                                let mut registry = config_registry.lock().unwrap();
                                let mut names: Vec<String> = registry.keys().cloned().collect();
                                names.sort();

                                if let Some(plugin_name) = names.get(selected_plugin_index).cloned() {
                                    if let Some(schema) = registry.get_mut(&plugin_name) {
                                        if let Some(field) = schema.fields.get_mut(selected_field_index) {

                                            if field.field_type == ConfigType::List {
                                                // Navigate into List Manager
                                                config_pane = ConfigPane::ListManager;
                                                selected_list_item_index = 0;
                                            } else if field.field_type == ConfigType::Channel || field.field_type == ConfigType::Role || field.field_type == ConfigType::Category {
                                                is_dropdown_open = true;
                                                dropdown_selected_index = 0;
                                            } else if field.field_type == ConfigType::Boolean {
                                                // --- INSTANT BOOLEAN TOGGLE ---
                                                let new_val = if field.default_value == "true" { "false".to_string() } else { "true".to_string() };

                                                field.default_value = new_val.clone(); // Optimistic UI update

                                                let _ = tx_to_bot.send(AdminCommand::SaveConfig {
                                                    plugin: plugin_name.clone(),
                                                    key: field.key.clone(),
                                                    value: new_val,
                                                });
                                            } else {
                                                // Normal Text Box
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
