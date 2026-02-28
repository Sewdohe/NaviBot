use mlua::Lua;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::UnboundedSender;

// Permission levels (hardcoded; owner is auto from guild.owner_id)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermLevel {
    User = 0,
    Helper = 1,
    Moderator = 2,
    Admin = 3,
    Owner = 4,
}

impl PermLevel {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "user" => Some(Self::User),
            "helper" => Some(Self::Helper),
            "moderator" => Some(Self::Moderator),
            "admin" => Some(Self::Admin),
            "owner" => Some(Self::Owner),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Helper => "helper",
            Self::Moderator => "moderator",
            Self::Admin => "admin",
            Self::Owner => "owner",
        }
    }
}

// Log levels for structured TUI output
#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

// Events: Bot -> TUI
// "Hey TUI, this just happened"
#[derive(Debug)]
pub enum BotEvent {
    Log(LogLevel, String),
    // Status { uptime: String, shard_id: u32 },
}

// Commands: TUI -> Bot
// "Hey Bot, do this!"
#[derive(Debug)]
pub enum AdminCommand {
    Shutdown,
    Reload,
    RefreshCache,
    CheckUpdate,
    // SendMessage { channel_id: u64, content: String },
    SaveConfig {
        plugin: String,
        key: String,
        value: String,
    },
    AppendListItem {
        plugin: String,
        key: String,
        item: HashMap<String, String>,
    },
    UpdateListItem {
        plugin: String,
        key: String,
        index: usize,
        item: HashMap<String, String>,
    },
    DeleteListItem {
        plugin: String,
        key: String,
        index: usize,
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
    pub tui_tx: UnboundedSender<BotEvent>,
    pub discord_state: SharedDiscordState,
    pub interval_registry: IntervalRegistry,
}

// Configuration Types
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConfigType {
    String,
    Number,
    Boolean,
    Channel,
    Role,
    Category,
    List,
    Enum,
}

// Schema for a sub-field within a List config item
#[derive(Clone, Debug)]
pub struct ConfigItemSchema {
    pub key: String,
    pub name: String,
    pub field_type: ConfigType, // Any type except List (no nesting)
    pub enum_options: Vec<String>,
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
    pub guild_owners: HashMap<String, String>, // guild_id -> owner_user_id
}

// A single setting for a plugin
#[derive(Clone, Debug)]
pub struct ConfigField {
    pub key: String,            // e.g., "channel_id"
    pub name: String,           // e.g., "Welcome Channel"
    pub description: String,    // e.g., "The ID of the channel to send welcome messages in"
    pub field_type: ConfigType, // e.g., ConfigType::String
    pub default_value: String,  // e.g., "" (unused for List fields)
    pub item_schema: Vec<ConfigItemSchema>,          // for List fields: sub-field definitions
    pub list_items: Vec<HashMap<String, String>>,    // for List fields: current items (in-memory)
    pub enum_options: Vec<String>,                   // non-empty only when field_type == Enum
}

// The full schema for a single plugin
#[derive(Clone, Debug)]
pub struct PluginSchema {
    pub fields: Vec<ConfigField>,
}

// The Shared Registry (Thread-safe map of all plugin schemas)
pub type ConfigRegistry = Arc<Mutex<HashMap<String, PluginSchema>>>;

// Registry of active interval tasks; each entry maps an interval ID to its cancel sender
pub type IntervalRegistry = Arc<Mutex<HashMap<u64, tokio::sync::watch::Sender<bool>>>>;

