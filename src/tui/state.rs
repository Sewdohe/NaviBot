use crate::types::LogLevel;
use std::collections::HashMap;

#[derive(PartialEq)]
pub enum AppMode {
    Logs,
    Config,
}

#[derive(PartialEq)]
pub enum ConfigPane {
    PluginList,
    FieldList,
    ListManager,
    ItemEditor,
}

pub struct AppState {
    // Log view
    pub logs: Vec<(LogLevel, String)>,
    pub log_scroll: u16,
    pub log_auto_scroll: bool,
    pub log_scroll_max: u16,
    pub input_mode: bool,
    pub input_buffer: String,
    // Mode / navigation
    pub mode: AppMode,
    pub config_pane: ConfigPane,
    pub selected_plugin_index: usize,
    pub selected_field_index: usize,
    // Field editing
    pub is_editing: bool,
    pub edit_buffer: String,
    // Field-level dropdown
    pub is_dropdown_open: bool,
    pub dropdown_selected_index: usize,
    // List manager
    pub selected_list_item_index: usize,
    // Item editor
    pub item_edit_buffer: HashMap<String, String>,
    pub item_edit_field_index: usize,
    pub item_editing_index: Option<usize>,
    pub item_subfield_editing: bool,
    pub item_subfield_buffer: String,
    pub item_dropdown_open: bool,
    pub item_dropdown_index: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            logs: Vec::new(),
            log_scroll: 0,
            log_auto_scroll: true,
            log_scroll_max: 0,
            input_mode: false,
            input_buffer: String::new(),
            mode: AppMode::Logs,
            config_pane: ConfigPane::PluginList,
            selected_plugin_index: 0,
            selected_field_index: 0,
            is_editing: false,
            edit_buffer: String::new(),
            is_dropdown_open: false,
            dropdown_selected_index: 0,
            selected_list_item_index: 0,
            item_edit_buffer: HashMap::new(),
            item_edit_field_index: 0,
            item_editing_index: None,
            item_subfield_editing: false,
            item_subfield_buffer: String::new(),
            item_dropdown_open: false,
            item_dropdown_index: 0,
        }
    }
}
