print("--- Loading Verification Plugin ---")

-- 1. Register Configuration for the TUI Dashboard
navi.register_config("verification", {
  { key = "verified_role",   name = "Verified Role",   description = "The role given after clicking verify.",         type = "role",    default = "" },
  { key = "unverified_role", name = "Unverified Role", description = "(Optional) Role to remove after verification.", type = "role",    default = "" },
  { key = "log_channel",     name = "Log Channel",     description = "(Optional) Channel to log verifications.",      type = "channel", default = "" }
})

-- 2. Slash Command to Spawn the Panel
navi.create_slash("setup_verification", "Spawns the human verification panel in the current channel", {},
  ---@param ctx NaviSlashCtx
  function(ctx)
    -- Spawn a rich embed with the verify button
    navi.send_message(ctx.channel_id, {
      title = "🛡️ Server Verification",
      description =
      "Welcome to the server! To gain access to the rest of the channels, please prove you are human by clicking the button below.",
      color = 0x2ECC71, -- Emerald Green
      components = {
        {
          type = "button",
          id = "btn_verify_human",
          label = "✅ Verify",
          style = "success"
        }
      }
    })

    -- Silent confirmation to the admin who ran the command
    ctx.reply("Verification panel spawned successfully!")
  end)

-- 3. Component Handler for the Button
navi.register_component("btn_verify_human",
  ---@param ctx NaviComponentCtx
  function(ctx)
    -- Explicitly fetch the exact keys the TUI generated
    local verified_role = navi.db.get("config:verification:verified_role")
    local unverified_role = navi.db.get("config:verification:unverified_role")
    local log_channel = navi.db.get("config:verification:log_channel")

    -- Check if the server owner has configured the role yet
    if not verified_role or verified_role == "" then
      ctx.reply("⚠️ Verification is not configured yet. Please set the Verified Role in the TUI dashboard.", true)
      return
    end

    -- Give the verified role so they can access the server channels
    navi.add_role(ctx.guild_id, ctx.user_id, verified_role)

    -- Remove the quarantine/unverified role if one is configured
    if unverified_role and unverified_role ~= "" then
      navi.remove_role(ctx.guild_id, ctx.user_id, unverified_role)
    end

    -- Log it if an admin log channel is configured
    if log_channel and log_channel ~= "" then
      navi.say(log_channel, "🛡️ <@" .. ctx.user_id .. "> has passed human verification.")
    end

    -- Reply ephemerally so only the user sees the success message
    ctx.reply("✅ You have been successfully verified. Welcome to the server!", true)
  end)

-- 4. Automatically quarantine new users via the Event Bus
navi.on("member_join", function(data)
  local unverified_role = navi.db.get("unverified_role")

  if unverified_role and unverified_role ~= "" then
    navi.add_role(data.guild_id, data.user_id, unverified_role)
    navi.log("🛡️ Automatically quarantined new user: " .. data.username)
  end
end)
