Name = "hyprbole-apps"
NamePretty = "Hyprbole Apps"
HideFromProviderlist = true

local function app_label(role)
  local handle = io.popen("hyprbole default-app " .. role .. " label 2>/dev/null")
  if handle == nil then
    return ""
  end

  local output = handle:read("*a") or ""
  handle:close()
  return output:gsub("%s+$", "")
end

function GetEntries()
  return {
    {
      Text = "Browser",
      Sub = app_label("browser"),
      Actions = {
        activate = "hyprbole-launch-browser",
      },
    },
    {
      Text = "Files",
      Sub = app_label("files"),
      Actions = {
        activate = "hyprbole-launch-files",
      },
    },
    {
      Text = "Passwords",
      Sub = "Open 1Password",
      Actions = {
        activate = "hyprbole-launch-passwords",
      },
    },
  }
end
