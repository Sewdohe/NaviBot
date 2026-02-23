use mlua::Lua;
use rusqlite::Connection;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::UnboundedSender;

// Events: Bot -> TUI
// "Hey TUI, this just happened"
#[derive(Debug)]
pub enum BotEvent {
    Log(String),
    // Status { uptime: String, shard_id: u32 },
    UserJoined(String),
}

// Commands: TUI -> Bot
// "Hey Bot, do this!"
#[derive(Debug)]
pub enum AdminCommand {
    Shutdown,
    Reload,
    RefreshCache,
    // SendMessage { channel_id: u64, content: String },
    SaveConfig {
        plugin: String,
        key: String,
        value: String,
    },
}

// ------------------------------------------------------------------
// 1. Core Types
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

// The "Context" Object
// In Poise, every command gets a "Context" passed to it.
// This Context holds "Data" (your custom state) and "Error" (what happens if it fails).
pub struct Data {
    pub lua: Arc<Mutex<Lua>>,
    pub db: Arc<Mutex<Connection>>,
    pub tui_tx: UnboundedSender<BotEvent>,
    pub discord_state: SharedDiscordState,
}

// Configuration Types
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigType {
    String,
    Number,
    Boolean,
    Channel,
    Role,
    Category,
}

#[derive(Clone, Debug)]
pub struct DiscordRole {
    pub id: String,
    pub name: String,
    pub color: (u8, u8, u8), // (R, G, B) tuple for Ratatui
}

pub type SharedDiscordState = Arc<Mutex<DiscordState>>;

#[derive(Clone, Debug, Default)]
pub struct DiscordState {
    pub channels: Vec<(String, String)>, // Stores (Channel_ID, Channel_Name)
    pub categories: Vec<(String, String)>,
    pub roles: Vec<DiscordRole>,
}

// A single setting for a plugin
#[derive(Clone, Debug)]
pub struct ConfigField {
    pub key: String,            // e.g., "channel_id"
    pub name: String,           // e.g., "Welcome Channel"
    pub description: String,    // e.g., "The ID of the channel to send welcome messages in"
    pub field_type: ConfigType, // e.g., ConfigType::String
    pub default_value: String,  // e.g., ""
}

// The full schema for a single plugin
#[derive(Clone, Debug)]
pub struct PluginSchema {
    pub plugin_name: String,
    pub fields: Vec<ConfigField>,
}

// The Shared Registry (Thread-safe map of all plugin schemas)
pub type ConfigRegistry = Arc<Mutex<HashMap<String, PluginSchema>>>;

