use crate::types::{AdminCommand, ConfigRegistry, ConfigType, LogLevel, SharedDiscordState};
use crossterm::event::KeyCode;
use tokio::sync::mpsc::UnboundedSender;
use super::state::{AppMode, AppState, ConfigPane};


/// Returns `true` if the caller should break the main loop (quit requested).
pub fn handle_input(
    key: KeyCode,
    state: &mut AppState,
    tx: &UnboundedSender<AdminCommand>,
    config_registry: &ConfigRegistry,
    discord_state: &SharedDiscordState,
) -> bool {
    if state.input_mode {
        match key {
            KeyCode::Enter => {
                state.logs.push((LogLevel::Info, format!("You typed: {}", state.input_buffer)));
                state.input_buffer.clear();
                state.input_mode = false;
            }
            KeyCode::Char(c) => state.input_buffer.push(c),
            KeyCode::Backspace => { state.input_buffer.pop(); }
            KeyCode::Esc => state.input_mode = false,
            _ => {}
        }
    } else {
        match key {
            // --- GLOBAL HOTKEYS ---
            KeyCode::Char('q') if !state.is_editing && !state.is_dropdown_open && !state.item_subfield_editing && !state.item_dropdown_open => {
                let _ = tx.send(AdminCommand::Shutdown);
                return true;
            }
            KeyCode::Char('r') if !state.is_editing && !state.is_dropdown_open && !state.item_subfield_editing && !state.item_dropdown_open => {
                let _ = tx.send(AdminCommand::Reload);
            }
            KeyCode::Char('u') if !state.is_editing && !state.is_dropdown_open && !state.item_subfield_editing && !state.item_dropdown_open => {
                let _ = tx.send(AdminCommand::RefreshCache);
            }
            KeyCode::Char('U') if !state.is_editing && !state.is_dropdown_open && !state.item_subfield_editing && !state.item_dropdown_open => {
                let _ = tx.send(AdminCommand::CheckUpdate);
            }
            KeyCode::Char('i') if !state.is_editing && !state.is_dropdown_open && !state.item_subfield_editing && !state.item_dropdown_open => {
                if state.mode == AppMode::Logs { state.input_mode = true; }
            }
            KeyCode::Char('c') if !state.is_editing && !state.is_dropdown_open && !state.item_subfield_editing && !state.item_dropdown_open => {
                state.mode = AppMode::Config;
            }
            KeyCode::Char('l') if !state.is_editing && !state.is_dropdown_open && !state.item_subfield_editing && !state.item_dropdown_open => {
                state.mode = AppMode::Logs;
            }
            KeyCode::Char('p') if !state.is_editing && !state.is_dropdown_open && !state.item_subfield_editing && !state.item_dropdown_open => {
                state.mode = AppMode::PluginBrowser;
                if state.plugin_browser_entries.is_empty() && !state.plugin_browser_loading {
                    state.plugin_browser_loading = true;
                    state.plugin_browser_status = "Fetching…".to_string();
                    let _ = tx.send(AdminCommand::FetchPluginList);
                }
            }

            // --- LOG SCROLLING ---
            KeyCode::Up if state.mode == AppMode::Logs => {
                state.log_auto_scroll = false;
                state.log_scroll = state.log_scroll.saturating_sub(1);
            }
            KeyCode::Down if state.mode == AppMode::Logs => {
                state.log_scroll = (state.log_scroll + 1).min(state.log_scroll_max);
                if state.log_scroll >= state.log_scroll_max { state.log_auto_scroll = true; }
            }

            // --- PLUGIN BROWSER ---
            KeyCode::Up if state.mode == AppMode::PluginBrowser => {
                state.plugin_browser_index = state.plugin_browser_index.saturating_sub(1);
            }
            KeyCode::Down if state.mode == AppMode::PluginBrowser => {
                let max = state.plugin_browser_entries.len().saturating_sub(1);
                if state.plugin_browser_index < max {
                    state.plugin_browser_index += 1;
                }
            }
            KeyCode::Enter if state.mode == AppMode::PluginBrowser => {
                if let Some(entry) = state.plugin_browser_entries.get(state.plugin_browser_index) {
                    state.plugin_browser_loading = true;
                    state.plugin_browser_status = format!("Installing {}…", entry.id);
                    let _ = tx.send(AdminCommand::InstallPlugin {
                        id: entry.id.clone(),
                        url: entry.url.clone(),
                        version: entry.version.clone(),
                    });
                }
            }
            KeyCode::Char('f') if state.mode == AppMode::PluginBrowser => {
                state.plugin_browser_loading = true;
                state.plugin_browser_status = "Fetching…".to_string();
                let _ = tx.send(AdminCommand::FetchPluginList);
            }
            KeyCode::Char('d') if state.mode == AppMode::PluginBrowser => {
                if let Some(entry) = state.plugin_browser_entries.get(state.plugin_browser_index) {
                    let path = format!("plugins/{}.lua", entry.id);
                    if std::fs::remove_file(&path).is_ok() {
                        // Remove the version entry from the sidecar file
                        let versions_path = "plugins/.versions.json";
                        if let Ok(s) = std::fs::read_to_string(versions_path) {
                            if let Ok(mut versions) = serde_json::from_str::<std::collections::HashMap<String, String>>(&s) {
                                versions.remove(&entry.id);
                                let _ = std::fs::write(
                                    versions_path,
                                    serde_json::to_string_pretty(&versions).unwrap_or_default(),
                                );
                            }
                        }
                        state.plugin_browser_status = format!("Removed {} — press 'r' to reload", entry.id);
                    }
                }
            }

            // --- DROPDOWN NAVIGATION ---
            KeyCode::Up if state.is_dropdown_open => {
                state.dropdown_selected_index = state.dropdown_selected_index.saturating_sub(1);
            }
            KeyCode::Down if state.is_dropdown_open => {
                let max = {
                    let registry = config_registry.lock().unwrap();
                    let mut names: Vec<String> = registry.keys().cloned().collect();
                    names.sort();
                    let (f_type, enum_len) = names.get(state.selected_plugin_index)
                        .and_then(|n| registry.get(n))
                        .and_then(|s| s.fields.get(state.selected_field_index))
                        .map(|f| (f.field_type.clone(), f.enum_options.len()))
                        .unwrap_or((ConfigType::String, 0));

                    let ds = discord_state.lock().unwrap();
                    match f_type {
                        ConfigType::Channel => ds.channels.len(),
                        ConfigType::Category => ds.categories.len(),
                        ConfigType::Role => ds.roles.len(),
                        ConfigType::Enum => enum_len,
                        _ => 0
                    }.saturating_sub(1)
                };
                if state.dropdown_selected_index < max { state.dropdown_selected_index += 1; }
            }
            KeyCode::Esc if state.is_dropdown_open => {
                state.is_dropdown_open = false;
            }

            // --- CONFIG DASHBOARD NAVIGATION ---
            KeyCode::Right | KeyCode::Enter if state.mode == AppMode::Config && state.config_pane == ConfigPane::PluginList && !state.is_dropdown_open => {
                state.config_pane = ConfigPane::FieldList;
                state.selected_field_index = 0;
            }
            KeyCode::Left | KeyCode::Esc if state.mode == AppMode::Config && state.config_pane == ConfigPane::FieldList && !state.is_editing && !state.is_dropdown_open => {
                state.config_pane = ConfigPane::PluginList;
            }
            KeyCode::Left | KeyCode::Esc if state.mode == AppMode::Config && state.config_pane == ConfigPane::ListManager => {
                state.config_pane = ConfigPane::FieldList;
            }
            KeyCode::Esc if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && !state.item_subfield_editing && !state.item_dropdown_open => {
                state.config_pane = ConfigPane::ListManager;
                state.item_edit_buffer.clear();
                state.item_editing_index = None;
            }

            // Navigating Left Pane
            KeyCode::Up if state.mode == AppMode::Config && state.config_pane == ConfigPane::PluginList && !state.is_dropdown_open => {
                state.selected_plugin_index = state.selected_plugin_index.saturating_sub(1);
            }
            KeyCode::Down if state.mode == AppMode::Config && state.config_pane == ConfigPane::PluginList && !state.is_dropdown_open => {
                let max = config_registry.lock().unwrap().len().saturating_sub(1);
                if state.selected_plugin_index < max { state.selected_plugin_index += 1; }
            }

            // Navigating Right Pane
            KeyCode::Up if state.mode == AppMode::Config && state.config_pane == ConfigPane::FieldList && !state.is_editing && !state.is_dropdown_open => {
                state.selected_field_index = state.selected_field_index.saturating_sub(1);
            }
            KeyCode::Down if state.mode == AppMode::Config && state.config_pane == ConfigPane::FieldList && !state.is_editing && !state.is_dropdown_open => {
                let registry = config_registry.lock().unwrap();
                let mut names: Vec<String> = registry.keys().cloned().collect();
                names.sort();
                if let Some(name) = names.get(state.selected_plugin_index) {
                    if let Some(schema) = registry.get(name) {
                        let max = schema.fields.len().saturating_sub(1);
                        if state.selected_field_index < max { state.selected_field_index += 1; }
                    }
                }
            }

            // Navigating ListManager
            KeyCode::Up if state.mode == AppMode::Config && state.config_pane == ConfigPane::ListManager => {
                state.selected_list_item_index = state.selected_list_item_index.saturating_sub(1);
            }
            KeyCode::Down if state.mode == AppMode::Config && state.config_pane == ConfigPane::ListManager => {
                let registry = config_registry.lock().unwrap();
                let mut names: Vec<String> = registry.keys().cloned().collect();
                names.sort();
                if let Some(name) = names.get(state.selected_plugin_index) {
                    if let Some(schema) = registry.get(name) {
                        if let Some(field) = schema.fields.get(state.selected_field_index) {
                            let max = field.list_items.len().saturating_sub(1);
                            if state.selected_list_item_index < max { state.selected_list_item_index += 1; }
                        }
                    }
                }
            }

            // Navigating ItemEditor sub-fields
            KeyCode::Up if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && !state.item_subfield_editing && !state.item_dropdown_open => {
                state.item_edit_field_index = state.item_edit_field_index.saturating_sub(1);
            }
            KeyCode::Down if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && !state.item_subfield_editing && !state.item_dropdown_open => {
                let registry = config_registry.lock().unwrap();
                let mut names: Vec<String> = registry.keys().cloned().collect();
                names.sort();
                if let Some(name) = names.get(state.selected_plugin_index) {
                    if let Some(schema) = registry.get(name) {
                        if let Some(field) = schema.fields.get(state.selected_field_index) {
                            let max = field.item_schema.len().saturating_sub(1);
                            if state.item_edit_field_index < max { state.item_edit_field_index += 1; }
                        }
                    }
                }
            }

            // --- LIST MANAGER KEYS ---
            KeyCode::Char('n') if state.mode == AppMode::Config && state.config_pane == ConfigPane::ListManager => {
                state.item_edit_buffer.clear();
                state.item_edit_field_index = 0;
                state.item_editing_index = None;
                state.item_subfield_editing = false;
                state.item_dropdown_open = false;
                state.config_pane = ConfigPane::ItemEditor;
            }
            KeyCode::Enter if state.mode == AppMode::Config && state.config_pane == ConfigPane::ListManager => {
                let registry = config_registry.lock().unwrap();
                let mut names: Vec<String> = registry.keys().cloned().collect();
                names.sort();
                if let Some(name) = names.get(state.selected_plugin_index) {
                    if let Some(schema) = registry.get(name) {
                        if let Some(field) = schema.fields.get(state.selected_field_index) {
                            if let Some(item) = field.list_items.get(state.selected_list_item_index) {
                                state.item_edit_buffer = item.clone();
                                state.item_edit_field_index = 0;
                                state.item_editing_index = Some(state.selected_list_item_index);
                                state.item_subfield_editing = false;
                                state.item_dropdown_open = false;
                                state.config_pane = ConfigPane::ItemEditor;
                            }
                        }
                    }
                }
            }
            KeyCode::Char('d') if state.mode == AppMode::Config && state.config_pane == ConfigPane::ListManager => {
                let plugin_name = {
                    let registry = config_registry.lock().unwrap();
                    let mut names: Vec<String> = registry.keys().cloned().collect();
                    names.sort();
                    names.get(state.selected_plugin_index).cloned()
                };
                if let Some(plugin) = plugin_name {
                    let field_key = {
                        let registry = config_registry.lock().unwrap();
                        registry.get(&plugin).and_then(|s| s.fields.get(state.selected_field_index)).map(|f| f.key.clone())
                    };
                    if let Some(key) = field_key {
                        let _ = tx.send(AdminCommand::DeleteListItem {
                            plugin,
                            key,
                            index: state.selected_list_item_index,
                        });
                        state.selected_list_item_index = state.selected_list_item_index.saturating_sub(1);
                    }
                }
            }

            // --- ITEM EDITOR KEYS ---
            // item dropdown navigation
            KeyCode::Up if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && state.item_dropdown_open => {
                state.item_dropdown_index = state.item_dropdown_index.saturating_sub(1);
            }
            KeyCode::Down if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && state.item_dropdown_open => {
                let registry = config_registry.lock().unwrap();
                let mut names: Vec<String> = registry.keys().cloned().collect();
                names.sort();
                let sub_type = names.get(state.selected_plugin_index)
                    .and_then(|n| registry.get(n))
                    .and_then(|s| s.fields.get(state.selected_field_index))
                    .and_then(|f| f.item_schema.get(state.item_edit_field_index))
                    .map(|sf| sf.field_type.clone())
                    .unwrap_or(ConfigType::String);
                let ds = discord_state.lock().unwrap();
                let enum_len = names.get(state.selected_plugin_index)
                    .and_then(|n| registry.get(n))
                    .and_then(|s| s.fields.get(state.selected_field_index))
                    .and_then(|f| f.item_schema.get(state.item_edit_field_index))
                    .map(|sf| sf.enum_options.len())
                    .unwrap_or(0);
                let max = match sub_type {
                    ConfigType::Channel => ds.channels.len(),
                    ConfigType::Category => ds.categories.len(),
                    ConfigType::Role => ds.roles.len(),
                    ConfigType::Enum => enum_len,
                    _ => 0,
                }.saturating_sub(1);
                if state.item_dropdown_index < max { state.item_dropdown_index += 1; }
            }
            KeyCode::Esc if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && state.item_dropdown_open => {
                state.item_dropdown_open = false;
            }
            KeyCode::Enter if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && state.item_dropdown_open => {
                let registry = config_registry.lock().unwrap();
                let mut names: Vec<String> = registry.keys().cloned().collect();
                names.sort();
                if let Some(name) = names.get(state.selected_plugin_index) {
                    if let Some(schema) = registry.get(name) {
                        if let Some(field) = schema.fields.get(state.selected_field_index) {
                            if let Some(sub_field) = field.item_schema.get(state.item_edit_field_index) {
                                let ds = discord_state.lock().unwrap();
                                let selected_id = match sub_field.field_type {
                                    ConfigType::Channel => ds.channels.get(state.item_dropdown_index).map(|(id, _)| id.clone()),
                                    ConfigType::Category => ds.categories.get(state.item_dropdown_index).map(|(id, _)| id.clone()),
                                    ConfigType::Role => ds.roles.get(state.item_dropdown_index).map(|r| r.id.clone()),
                                    ConfigType::Enum => sub_field.enum_options.get(state.item_dropdown_index).cloned(),
                                    _ => None,
                                };
                                if let Some(id) = selected_id {
                                    state.item_edit_buffer.insert(sub_field.key.clone(), id);
                                }
                            }
                        }
                    }
                }
                state.item_dropdown_open = false;
            }
            // typing inside item sub-field text box
            KeyCode::Esc if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && state.item_subfield_editing => {
                state.item_subfield_editing = false;
                state.item_subfield_buffer.clear();
            }
            KeyCode::Backspace if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && state.item_subfield_editing => {
                state.item_subfield_buffer.pop();
            }
            KeyCode::Char(c) if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && state.item_subfield_editing => {
                state.item_subfield_buffer.push(c);
            }
            KeyCode::Enter if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && state.item_subfield_editing => {
                let registry = config_registry.lock().unwrap();
                let mut names: Vec<String> = registry.keys().cloned().collect();
                names.sort();
                if let Some(name) = names.get(state.selected_plugin_index) {
                    if let Some(schema) = registry.get(name) {
                        if let Some(field) = schema.fields.get(state.selected_field_index) {
                            if let Some(sub_field) = field.item_schema.get(state.item_edit_field_index) {
                                state.item_edit_buffer.insert(sub_field.key.clone(), state.item_subfield_buffer.clone());
                            }
                        }
                    }
                }
                state.item_subfield_editing = false;
                state.item_subfield_buffer.clear();
            }
            // open editor/dropdown for selected sub-field
            KeyCode::Enter if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && !state.item_subfield_editing && !state.item_dropdown_open => {
                let registry = config_registry.lock().unwrap();
                let mut names: Vec<String> = registry.keys().cloned().collect();
                names.sort();
                if let Some(name) = names.get(state.selected_plugin_index) {
                    if let Some(schema) = registry.get(name) {
                        if let Some(field) = schema.fields.get(state.selected_field_index) {
                            if let Some(sub_field) = field.item_schema.get(state.item_edit_field_index) {
                                match sub_field.field_type {
                                    ConfigType::Channel | ConfigType::Category | ConfigType::Role | ConfigType::Enum => {
                                        state.item_dropdown_open = true;
                                        state.item_dropdown_index = 0;
                                    }
                                    ConfigType::Boolean => {
                                        let current = state.item_edit_buffer.get(&sub_field.key).map(|v| v.as_str()).unwrap_or("false");
                                        let new_val = if current == "true" { "false" } else { "true" };
                                        state.item_edit_buffer.insert(sub_field.key.clone(), new_val.to_string());
                                    }
                                    _ => {
                                        state.item_subfield_buffer = state.item_edit_buffer.get(&sub_field.key).cloned().unwrap_or_default();
                                        state.item_subfield_editing = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // save the item
            KeyCode::Char('s') if state.mode == AppMode::Config && state.config_pane == ConfigPane::ItemEditor && !state.item_subfield_editing => {
                let plugin_name = {
                    let registry = config_registry.lock().unwrap();
                    let mut names: Vec<String> = registry.keys().cloned().collect();
                    names.sort();
                    names.get(state.selected_plugin_index).cloned()
                };
                if let Some(plugin) = plugin_name {
                    let field_key = {
                        let registry = config_registry.lock().unwrap();
                        registry.get(&plugin).and_then(|s| s.fields.get(state.selected_field_index)).map(|f| f.key.clone())
                    };
                    if let Some(key) = field_key {
                        let item = state.item_edit_buffer.clone();
                        match state.item_editing_index {
                            None => {
                                let _ = tx.send(AdminCommand::AppendListItem { plugin, key, item });
                            }
                            Some(i) => {
                                let _ = tx.send(AdminCommand::UpdateListItem { plugin, key, index: i, item });
                            }
                        }
                    }
                }
                state.item_edit_buffer.clear();
                state.item_editing_index = None;
                state.config_pane = ConfigPane::ListManager;
            }

            // --- EDITING A FIELD OR SELECTING DROPDOWN ---
            KeyCode::Enter if state.mode == AppMode::Config && state.config_pane == ConfigPane::FieldList => {
                if state.is_dropdown_open {
                    // 1. Save Dropdown Selection (Optimistic UI Update)
                    let mut registry = config_registry.lock().unwrap();
                    let mut names: Vec<String> = registry.keys().cloned().collect();
                    names.sort();

                    if let Some(plugin_name) = names.get(state.selected_plugin_index).cloned() {
                        if let Some(schema) = registry.get_mut(&plugin_name) {
                            if let Some(field) = schema.fields.get_mut(state.selected_field_index) {

                                let mut selected_id = None;
                                let ds = discord_state.lock().unwrap();
                                match field.field_type {
                                    ConfigType::Channel => { if let Some((id, _)) = ds.channels.get(state.dropdown_selected_index) { selected_id = Some(id.clone()); } }
                                    ConfigType::Category => { if let Some((id, _)) = ds.categories.get(state.dropdown_selected_index) { selected_id = Some(id.clone()); } }
                                    ConfigType::Role => { if let Some(role) = ds.roles.get(state.dropdown_selected_index) { selected_id = Some(role.id.clone()); } }
                                    ConfigType::Enum => { if let Some(option) = field.enum_options.get(state.dropdown_selected_index) { selected_id = Some(option.clone()); } }
                                    _ => {}
                                }

                                if let Some(id) = selected_id {
                                    field.default_value = id.clone();

                                    let _ = tx.send(AdminCommand::SaveConfig {
                                        plugin: plugin_name.clone(),
                                        key: field.key.clone(),
                                        value: id,
                                    });
                                }
                            }
                        }
                    }
                    state.is_dropdown_open = false;
                }
                else if state.is_editing {
                    // 2. Save Text Box (Optimistic UI Update)
                    let mut registry = config_registry.lock().unwrap();
                    let mut names: Vec<String> = registry.keys().cloned().collect();
                    names.sort();

                    if let Some(plugin_name) = names.get(state.selected_plugin_index).cloned() {
                        if let Some(schema) = registry.get_mut(&plugin_name) {
                            if let Some(field) = schema.fields.get_mut(state.selected_field_index) {

                                field.default_value = state.edit_buffer.clone();

                                let _ = tx.send(AdminCommand::SaveConfig {
                                    plugin: plugin_name.clone(),
                                    key: field.key.clone(),
                                    value: state.edit_buffer.clone(),
                                });
                            }
                        }
                    }
                    state.is_editing = false;
                    state.edit_buffer.clear();
                }
                else {
                    // 3. Open Editor, Dropdown, List Manager, or Toggle Boolean
                    let mut registry = config_registry.lock().unwrap();
                    let mut names: Vec<String> = registry.keys().cloned().collect();
                    names.sort();

                    if let Some(plugin_name) = names.get(state.selected_plugin_index).cloned() {
                        if let Some(schema) = registry.get_mut(&plugin_name) {
                            if let Some(field) = schema.fields.get_mut(state.selected_field_index) {

                                if field.field_type == ConfigType::List {
                                    state.config_pane = ConfigPane::ListManager;
                                    state.selected_list_item_index = 0;
                                } else if field.field_type == ConfigType::Channel || field.field_type == ConfigType::Role || field.field_type == ConfigType::Category || field.field_type == ConfigType::Enum {
                                    state.is_dropdown_open = true;
                                    state.dropdown_selected_index = 0;
                                } else if field.field_type == ConfigType::Boolean {
                                    let new_val = if field.default_value == "true" { "false".to_string() } else { "true".to_string() };

                                    field.default_value = new_val.clone();

                                    let _ = tx.send(AdminCommand::SaveConfig {
                                        plugin: plugin_name.clone(),
                                        key: field.key.clone(),
                                        value: new_val,
                                    });
                                } else {
                                    state.is_editing = true;
                                    state.edit_buffer = field.default_value.clone();
                                }
                            }
                        }
                    }
                }
            }

            // --- TYPING INSIDE THE TEXT BOX ---
            KeyCode::Esc if state.is_editing => {
                state.is_editing = false;
                state.edit_buffer.clear();
            }
            KeyCode::Backspace if state.is_editing => { state.edit_buffer.pop(); }
            KeyCode::Char(c) if state.is_editing => { state.edit_buffer.push(c); }

            _ => {}
        }
    }
    false
}
