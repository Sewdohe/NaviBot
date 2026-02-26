use mlua::prelude::*;
use poise::serenity_prelude as serenity;

/// Builds a `CreateInteractionResponseMessage` with embed + optional components
/// from the same NaviEmbed table shape used by `navi.send_message`.
pub fn build_reply_embed(
    data: &LuaTable,
    ephemeral: bool,
) -> LuaResult<serenity::CreateInteractionResponseMessage> {
    let title: Option<String> = data.get("title").ok();
    let description: Option<String> = data.get("description").ok();
    let color: Option<u32> = data.get("color").ok();
    let image_url: Option<String> = data.get::<_, String>("image").ok().filter(|s| !s.is_empty());

    let mut fields = Vec::new();
    if let Ok(lua_fields) = data.get::<_, Vec<LuaTable>>("fields") {
        for f in lua_fields {
            let name: String = f.get("name").unwrap_or_default();
            let value: String = f.get("value").unwrap_or_default();
            let inline: bool = f.get("inline").unwrap_or(false);
            fields.push((name, value, inline));
        }
    }

    let mut action_rows: Vec<serenity::CreateActionRow> = Vec::new();
    let mut current_buttons: Vec<serenity::CreateButton> = Vec::new();
    if let Ok(comps) = data.get::<_, Vec<LuaTable>>("components") {
        for c in comps {
            let c_type: String = c.get("type").unwrap_or("button".into());
            if c_type == "button" {
                let label: String = c.get("label").unwrap_or("Button".into());
                let style_str: String = c.get("style").unwrap_or("primary".into());
                if style_str == "link" || style_str == "url" {
                    let url: String = c.get("url").unwrap_or_default();
                    current_buttons.push(serenity::CreateButton::new_link(url).label(label));
                } else {
                    let custom_id: String = c.get("id").unwrap_or("unknown".into());
                    let style = match style_str.as_str() {
                        "secondary" | "gray" => serenity::ButtonStyle::Secondary,
                        "success" | "green" => serenity::ButtonStyle::Success,
                        "danger" | "red" => serenity::ButtonStyle::Danger,
                        _ => serenity::ButtonStyle::Primary,
                    };
                    current_buttons
                        .push(serenity::CreateButton::new(custom_id).style(style).label(label));
                }
            } else if c_type == "select" {
                if !current_buttons.is_empty() {
                    action_rows.push(serenity::CreateActionRow::Buttons(current_buttons.clone()));
                    current_buttons.clear();
                }
                let custom_id: String = c.get("id").unwrap_or("select_menu".into());
                let placeholder: String =
                    c.get("placeholder").unwrap_or("Select an option...".into());
                let mut options = Vec::new();
                if let Ok(lua_opts) = c.get::<_, Vec<LuaTable>>("options") {
                    for opt in lua_opts {
                        let lbl: String = opt.get("label").unwrap_or("Option".into());
                        let val: String = opt.get("value").unwrap_or(lbl.clone());
                        let mut builder = serenity::CreateSelectMenuOption::new(lbl, val);
                        if let Ok(desc) = opt.get::<_, String>("description") {
                            builder = builder.description(desc);
                        }
                        options.push(builder);
                    }
                }
                let menu = serenity::CreateSelectMenu::new(
                    custom_id,
                    serenity::CreateSelectMenuKind::String { options },
                )
                .placeholder(placeholder);
                action_rows.push(serenity::CreateActionRow::SelectMenu(menu));
            }
        }
    }
    if !current_buttons.is_empty() {
        action_rows.push(serenity::CreateActionRow::Buttons(current_buttons));
    }

    let mut embed = serenity::CreateEmbed::new();
    if let Some(t) = title { embed = embed.title(t); }
    if let Some(d) = description { embed = embed.description(d); }
    if let Some(c) = color { embed = embed.color(serenity::Color::new(c)); }
    for (n, v, i) in fields { embed = embed.field(n, v, i); }
    if let Some(img) = image_url { embed = embed.image(img); }

    let mut msg = serenity::CreateInteractionResponseMessage::new()
        .embed(embed)
        .ephemeral(ephemeral);
    if !action_rows.is_empty() {
        msg = msg.components(action_rows);
    }
    Ok(msg)
}
