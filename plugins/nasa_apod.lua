navi.log.info("Loading NASA APOD Plugin")

navi.register_config("nasa_apod", {
  { key = "api_key", name = "NASA API Key",
    description = "Get a free key at api.nasa.gov",
    type = "string", default = "DEMO_KEY" },
})

navi.create_slash("apod", "Post NASA's Astronomy Picture of the Day", {
  { name = "date", description = "Date in YYYY-MM-DD format (default: today)",
    type = "string", required = false },
},
---@param ctx NaviSlashCtx
function(ctx)
  local api_key = navi.db.get("config:nasa_apod:api_key") or "DEMO_KEY"
  local url     = "https://api.nasa.gov/planetary/apod?api_key=" .. api_key
  if ctx.args.date and ctx.args.date ~= "" then
    url = url .. "&date=" .. ctx.args.date
  end

  local raw = navi.http.get(url)
  if not raw then
    ctx.reply("❌ Failed to reach the NASA API. Check your connection.")
    return
  end

  local ok, data = pcall(navi.json.decode, raw)
  if not ok or not data then
    ctx.reply("❌ Could not parse the NASA API response.")
    return
  end

  -- NASA returns an error object if the date is invalid
  if data.code then
    ctx.reply("❌ NASA API error: " .. tostring(data.msg or data.code))
    return
  end

  local title       = tostring(data.title or "Astronomy Picture of the Day")
  local explanation = tostring(data.explanation or "")
  local date        = tostring(data.date or "")
  local media_type  = tostring(data.media_type or "image")
  local img_url     = tostring(data.url or "")
  local copyright   = data.copyright and ("© " .. tostring(data.copyright)) or "NASA / Public Domain"

  -- Truncate explanation to fit Discord embed limits (4096 chars max)
  if #explanation > 900 then
    explanation = string.sub(explanation, 1, 897) .. "..."
  end

  if media_type == "video" then
    -- Videos can't be embedded as images; post a link instead
    navi.send_message(ctx.channel_id, {
      title       = "🎬 " .. title,
      description = explanation .. "\n\n📅 " .. date .. "  •  " .. copyright
                 .. "\n\n🔗 [Watch Video](" .. img_url .. ")",
      color       = 0x0B3D91,
    })
  else
    navi.send_message(ctx.channel_id, {
      title       = "🔭 " .. title,
      description = explanation .. "\n\n📅 " .. date .. "  •  " .. copyright,
      image       = img_url,
      color       = 0x0B3D91,
    })
  end

  ctx.reply("APOD posted!")
end)
