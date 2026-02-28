use crate::types::{ConfigRegistry, ConfigType, LogLevel, SharedDiscordState};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use super::state::{AppMode, AppState, ConfigPane};

pub fn draw(
    f: &mut Frame,
    state: &mut AppState,
    config_registry: &ConfigRegistry,
    discord_state: &SharedDiscordState,
) {
    let size = f.area();

    // --- 1. THE GLOBAL HEADER ---
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
        ].as_ref())
        .split(size);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" 🤖 Navi Engine v1.0 ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" | "),
        Span::styled("🟢 System Online", Style::default().fg(Color::Green)),
    ]));
    f.render_widget(header, main_chunks[0]);

    // --- 2. ROUTE THE REST OF THE APP ---
    if state.mode == AppMode::Logs {
        // --- LOGS VIEW ---
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
            ].as_ref())
            .split(main_chunks[1]);

        let plugin_style = Style::default().fg(Color::Magenta);
        let log_lines: Vec<Line> = state.logs.iter().map(|(level, msg)| {
                let (prefix, style) = match level {
                    LogLevel::Error => ("[E]", Style::default().fg(Color::Red)),
                    LogLevel::Warn  => ("[W]", Style::default().fg(Color::Yellow)),
                    LogLevel::Info  => ("[I]", Style::default().fg(Color::Cyan)),
                };
                let spans = if msg.starts_with('[') {
                    if let Some(close) = msg.find(']') {
                        let tag  = &msg[..=close];
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

        let visible_height = chunks[0].height.saturating_sub(2);
        let max_scroll = (log_lines.len() as u16).saturating_sub(visible_height);
        state.log_scroll_max = max_scroll;
        if state.log_auto_scroll { state.log_scroll = max_scroll; }
        let scroll_offset = state.log_scroll.min(max_scroll);

        let logs_pane = Paragraph::new(log_lines)
                .block(Block::default()
                    .title(" Navi Logs ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White))
                )
                .wrap(ratatui::widgets::Wrap { trim: true })
                .scroll((scroll_offset, 0));

        f.render_widget(logs_pane, chunks[0]);

        let input_text = if state.input_mode {
            format!("> {}_", state.input_buffer)
        } else {
            String::from("Up/Down: scroll logs | 'q' quit | 'r' reload | 'u' re-cache | 'U' update | 'i' type | 'c' config")
        };

        let input_widget = Paragraph::new(input_text)
            .style(if state.input_mode { Style::default().fg(Color::Yellow) } else { Style::default() })
            .block(Block::default().borders(Borders::ALL).title(" Controls "));
        f.render_widget(input_widget, chunks[1]);

    } else if state.mode == AppMode::Config {
        // --- CONFIG VIEW ---
        let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(main_chunks[1]);

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

        if state.selected_plugin_index >= plugin_names.len() {
            state.selected_plugin_index = plugin_names.len().saturating_sub(1);
        }

        // Left Pane: Plugin List
        let items: Vec<ListItem> = plugin_names.iter().enumerate().map(|(i, name)| {
                if i == state.selected_plugin_index {
                    ListItem::new(format!("> {}", name)).style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                } else {
                    ListItem::new(format!("  {}", name))
                }
            }).collect();

        let left_border_style = if state.config_pane == ConfigPane::PluginList {
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
        let active_plugin = &plugin_names[state.selected_plugin_index];
        if let Some(schema) = registry.get(active_plugin) {
            if state.selected_field_index >= schema.fields.len() {
                state.selected_field_index = schema.fields.len().saturating_sub(1);
            }

            if state.config_pane == ConfigPane::ListManager {
                if let Some(list_field) = schema.fields.get(state.selected_field_index) {
                    if !list_field.list_items.is_empty()
                        && state.selected_list_item_index >= list_field.list_items.len()
                    {
                        state.selected_list_item_index =
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

                            let line = if i == state.selected_list_item_index {
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
            } else if state.config_pane == ConfigPane::ItemEditor {
                if let Some(list_field) = schema.fields.get(state.selected_field_index) {
                    if !list_field.item_schema.is_empty()
                        && state.item_edit_field_index >= list_field.item_schema.len()
                    {
                        state.item_edit_field_index =
                            list_field.item_schema.len().saturating_sub(1);
                    }

                    let title = match state.item_editing_index {
                        Some(i) => format!(" Editing item #{} ", i + 1),
                        None => " New Item ".to_string(),
                    };

                    let mut text = vec![Line::from("")];

                    for (i, sub_field) in list_field.item_schema.iter().enumerate() {
                        let name_style = if i == state.item_edit_field_index {
                            Style::default().fg(Color::Black).bg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::Yellow)
                        };

                        text.push(Line::from(Span::styled(&sub_field.name, name_style)));

                        let current_val = state.item_edit_buffer
                            .get(&sub_field.key)
                            .cloned()
                            .unwrap_or_default();

                        if state.item_subfield_editing && i == state.item_edit_field_index {
                            text.push(Line::from(Span::styled(
                                format!(" [ {}_ ]", state.item_subfield_buffer),
                                Style::default().fg(Color::White).bg(Color::DarkGray),
                            )));
                        } else {
                            match sub_field.field_type {
                                ConfigType::Enum => {
                                    let display = if current_val.is_empty() { "(not set)".to_string() } else { current_val.clone() };
                                    text.push(Line::from(Span::styled(
                                        format!(" [ {} ]", display),
                                        Style::default().fg(Color::Green),
                                    )));
                                }
                                _ => { text.push(Line::from(format!(" [ {} ]", current_val))); }
                            }
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
                    if state.item_dropdown_open {
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

                        let sub_enum_options = list_field.item_schema
                            .get(state.item_edit_field_index)
                            .map(|s| s.enum_options.clone())
                            .unwrap_or_default();
                        let ds = discord_state.lock().unwrap();
                        let sub_field_type = list_field
                            .item_schema
                            .get(state.item_edit_field_index)
                            .map(|s| s.field_type.clone())
                            .unwrap_or(ConfigType::String);

                        let (list_title, list_len) = match sub_field_type {
                            ConfigType::Channel => {
                                (" 📚 Select Channel ", ds.channels.len())
                            }
                            ConfigType::Category => {
                                (" 📁 Select Category ", ds.categories.len())
                            }
                            ConfigType::Role => (" 🎭 Select Role ", ds.roles.len()),
                            ConfigType::Enum => (" 📋 Select Option ", sub_enum_options.len()),
                            _ => (" Select ", 0),
                        };

                        let safe_idx =
                            state.item_dropdown_index.min(list_len.saturating_sub(1));

                        let popup_items: Vec<ListItem> = match sub_field_type {
                            ConfigType::Channel => ds
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
                            ConfigType::Category => ds
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
                            ConfigType::Role => ds
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
                            ConfigType::Enum => sub_enum_options.iter().enumerate().map(|(i, opt)| {
                                if i == safe_idx {
                                    ListItem::new(format!("> {}", opt))
                                        .style(Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD))
                                } else {
                                    ListItem::new(format!("  {}", opt))
                                }
                            }).collect(),
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

                let ds = discord_state.lock().unwrap();

                for (i, field) in schema.fields.iter().enumerate() {
                    let name_style = if state.config_pane == ConfigPane::FieldList
                        && i == state.selected_field_index
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
                    } else if state.is_editing && i == state.selected_field_index {
                        text.push(Line::from(Span::styled(
                            format!(" [ {}_ ]", state.edit_buffer),
                            Style::default().fg(Color::White).bg(Color::DarkGray),
                        )));
                    } else {
                        match field.field_type {
                            ConfigType::Channel => {
                                let display = ds.channels.iter()
                                    .find(|(id, _)| id == &field.default_value)
                                    .map(|(_, name)| format!("#{}", name))
                                    .unwrap_or_else(|| if field.default_value.is_empty() {
                                        "(not set)".to_string()
                                    } else {
                                        field.default_value.clone()
                                    });
                                text.push(Line::from(Span::styled(
                                    format!(" [ {} ]", display),
                                    Style::default().fg(Color::Cyan),
                                )));
                            }
                            ConfigType::Category => {
                                let display = ds.categories.iter()
                                    .find(|(id, _)| id == &field.default_value)
                                    .map(|(_, name)| name.clone())
                                    .unwrap_or_else(|| if field.default_value.is_empty() {
                                        "(not set)".to_string()
                                    } else {
                                        field.default_value.clone()
                                    });
                                text.push(Line::from(Span::styled(
                                    format!(" [ {} ]", display),
                                    Style::default().fg(Color::Yellow),
                                )));
                            }
                            ConfigType::Role => {
                                let (display, color) = ds.roles.iter()
                                    .find(|r| r.id == field.default_value)
                                    .map(|r| {
                                        let c = if r.color == (0, 0, 0) {
                                            Color::White
                                        } else {
                                            Color::Rgb(r.color.0, r.color.1, r.color.2)
                                        };
                                        (format!("@{}", r.name), c)
                                    })
                                    .unwrap_or_else(|| {
                                        let label = if field.default_value.is_empty() {
                                            "(not set)".to_string()
                                        } else {
                                            field.default_value.clone()
                                        };
                                        (label, Color::White)
                                    });
                                text.push(Line::from(Span::styled(
                                    format!(" [ {} ]", display),
                                    Style::default().fg(color),
                                )));
                            }
                            ConfigType::Enum => {
                                let display = if field.default_value.is_empty() {
                                    "(not set)".to_string()
                                } else {
                                    field.default_value.clone()
                                };
                                text.push(Line::from(Span::styled(
                                    format!(" [ {} ]", display),
                                    Style::default().fg(Color::Green),
                                )));
                            }
                            _ => {
                                text.push(Line::from(format!(" [ {} ]", field.default_value)));
                            }
                        }
                    }

                    text.push(Line::from(""));
                }
                drop(ds);

                text.push(Line::from(Span::styled(
                    "Press 'l' to return to logs.",
                    Style::default().fg(Color::DarkGray),
                )));

                let right_pane = Paragraph::new(text).block(
                    Block::default()
                        .title(if state.config_pane == ConfigPane::FieldList {
                            "⚙️ Settings (Editing)"
                        } else {
                            "⚙️ Settings"
                        })
                        .borders(Borders::ALL)
                        .border_style(if state.config_pane == ConfigPane::FieldList {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default()
                        }),
                );
                f.render_widget(right_pane, chunks[1]);

                // --- DROPDOWN POPUP OVERLAY ---
                if state.is_dropdown_open {
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

                    let active_enum_options = schema.fields[state.selected_field_index].enum_options.clone();
                    let ds = discord_state.lock().unwrap();
                    let active_field_type =
                        schema.fields[state.selected_field_index].field_type.clone();

                    let (list_title, list_len) = match active_field_type {
                        ConfigType::Channel => {
                            (" 📚 Select Channel ", ds.channels.len())
                        }
                        ConfigType::Category => {
                            (" 📁 Select Category ", ds.categories.len())
                        }
                        ConfigType::Role => (" 🎭 Select Role ", ds.roles.len()),
                        ConfigType::Enum => (" 📋 Select Option ", active_enum_options.len()),
                        _ => (" Select ", 0),
                    };

                    let safe_index =
                        state.dropdown_selected_index.min(list_len.saturating_sub(1));

                    let items: Vec<ListItem> = match active_field_type {
                        ConfigType::Channel => ds
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
                        ConfigType::Category => ds
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
                        ConfigType::Role => ds
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
                        ConfigType::Enum => active_enum_options.iter().enumerate().map(|(i, opt)| {
                            if i == safe_index {
                                ListItem::new(format!("> {}", opt))
                                    .style(Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD))
                            } else {
                                ListItem::new(format!("  {}", opt))
                            }
                        }).collect(),
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
        let help_text = if state.item_dropdown_open {
            "Up/Down to scroll | Enter to confirm | Esc to cancel"
        } else if state.item_subfield_editing {
            "Type value | Enter to confirm | Esc to cancel"
        } else if state.config_pane == ConfigPane::ItemEditor {
            "Up/Down to select sub-field | Enter to edit | s=save  Esc=back"
        } else if state.config_pane == ConfigPane::ListManager {
            "Up/Down to select item | Enter=edit | n=add | d=delete | Esc=back"
        } else if state.is_dropdown_open {
            "Press 'Up/Down' to scroll | 'Enter' to confirm selection | 'Esc' to cancel"
        } else if state.is_editing {
            "Type your value | Press 'Enter' to save | 'Esc' to cancel"
        } else if state.config_pane == ConfigPane::PluginList {
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
}
