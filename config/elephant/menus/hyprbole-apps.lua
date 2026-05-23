Name = "hyprbole-apps"
NamePretty = "Hyprbole Applications"
HideFromProviderlist = true
Parent = "hyprbole"
FixedOrder = true

local function app_label(role)
  local handle = io.popen("hyprbole default-app " .. role .. " label 2>/dev/null")
  if handle == nil then
    return ""
  end

  local output = handle:read("*a") or ""
  handle:close()
  return output:gsub("%s+$", "")
end

local function command_exists(command)
  local handle = io.popen("command -v " .. command .. " 2>/dev/null")
  if handle == nil then
    return false
  end

  local output = handle:read("*a") or ""
  handle:close()
  return output ~= ""
end

function GetEntries()
  return {
    {
      Text = "󰀻  App Launcher (Super + Shift + Space)",
      Subtext = "Open Walker's main app launcher",
      Actions = {
        activate = "hyprbole launch launcher",
      },
      State = { "sublevel" },
    },
    {
      Text = "󰖟  Browser",
      Subtext = app_label("browser"),
      Actions = {
        activate = "hyprbole-launch-browser",
      },
      State = { "sublevel" },
    },
    {
      Text = "󰉋  Files",
      Subtext = app_label("files"),
      Actions = {
        activate = "hyprbole-launch-files",
      },
      State = { "sublevel" },
    },
    command_exists("1password") and {
      Text = "󰌆  Passwords",
      Subtext = "Open 1Password",
      Actions = {
        activate = "hyprbole-launch-passwords",
      },
      State = { "sublevel" },
    } or {
      Text = "󰌆  Passwords",
      Subtext = "Install 1Password from Extras",
      Actions = {
        activate = "hyprbole-launch-extra-install onepassword",
      },
      State = { "sublevel" },
    },
  }
end
