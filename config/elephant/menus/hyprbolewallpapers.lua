Name = "hyprbolewallpapers"
NamePretty = "Hyprbole Wallpapers"
HideFromProviderlist = true
Parent = "hyprbole-wallpaper-settings"
FixedOrder = true

local function file_exists(path)
  local handle = io.open(path, "r")
  if handle then
    handle:close()
    return true
  end
  return false
end

local function shell_quote(value)
  return string.format("'%s'", tostring(value):gsub("'", "'\\''"))
end

function GetEntries()
  local entries = {}
  local home = os.getenv("HOME")
  local hyprbole_config_path = os.getenv("HYPRBOLE_CONFIG_PATH") or (home .. "/.config/hyprbole")
  local current_theme_file = hyprbole_config_path .. "/current/theme-name"
  local current_wallpaper_file = hyprbole_config_path .. "/current/background"
  local current_theme = ""
  local current_wallpaper = ""

  if file_exists(current_theme_file) then
    local handle = io.open(current_theme_file, "r")
    if handle then
      current_theme = handle:read("*l") or ""
      handle:close()
    end
  end

  local handle = io.popen("readlink -f '" .. current_wallpaper_file .. "' 2>/dev/null")
  if handle then
    local resolved = handle:read("*l") or ""
    handle:close()
    current_wallpaper = resolved:match(".*/(.+)$") or ""
  end

  if current_theme == "" then
    return entries
  end

  local backgrounds_dir = hyprbole_config_path .. "/current/theme/backgrounds"
  local handle = io.popen("find -L '" .. backgrounds_dir .. "' -maxdepth 1 -type f 2>/dev/null | sort")
  if not handle then
    return entries
  end

  for path in handle:lines() do
    local wallpaper_name = path:match(".*/(.+)$")
    if wallpaper_name then
      local text = wallpaper_name
      if wallpaper_name == current_wallpaper then
        text = text .. "  (Current)"
      end

      table.insert(entries, {
        Text = text,
        Subtext = current_theme,
        Preview = path,
        PreviewType = "file",
        Actions = {
          activate = "hyprbole theme wallpaper set " .. shell_quote(current_theme) .. " " .. shell_quote(wallpaper_name),
        },
        State = { "sublevel" },
      })
    end
  end

  handle:close()
  return entries
end
