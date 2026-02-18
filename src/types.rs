use std::sync::{Arc, Mutex};
use mlua::Lua;
use rusqlite::Connection;
use serde::Deserialize;

// 1. Core Types
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

// The "Context" Object
// In Poise, every command gets a "Context" passed to it.
// This Context holds "Data" (your custom state) and "Error" (what happens if it fails).
pub struct Data {
    pub lua: Arc<Mutex<Lua>>,
    pub db: Arc<Mutex<Connection>>, // Added this to match your code
}

// 2. Embed Structs (Needed for send_embed)
// We make fields 'pub' so other modules can read them.
#[derive(Debug, Deserialize)]
pub struct LuaEmbedField {
    pub name: String,
    pub value: String,
    pub inline: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct LuaEmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LuaEmbed {
    pub title: Option<String>,
    pub description: Option<String>,
    pub color: Option<u32>,
    pub url: Option<String>,
    pub image: Option<String>,
    pub thumbnail: Option<String>,
    pub footer: Option<LuaEmbedFooter>,
    pub fields: Option<Vec<LuaEmbedField>>,
}