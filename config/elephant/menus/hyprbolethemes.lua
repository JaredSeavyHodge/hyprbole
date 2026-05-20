Name = "hyprbolethemes"
NamePretty = "Hyprbole Themes"
HideFromProviderlist = true
Parent = "hyprbole-theme"
FixedOrder = true

local function file_exists(path)
  local handle = io.open(path, "r")
  if handle then
    handle:close()
    return true
  end
  return false
end

local function first_image_in_dir(dir)
  local handle = io.popen("ls -1 '" .. dir .. "' 2>/dev/null | head -n 1")
  if not handle then
    return nil
  end

  local file = handle:read("*l")
  handle:close()

  if file and file ~= "" then
    return dir .. "/" .. file
  end

  return nil
end

local function find_preview_path(dir)
  local png = dir .. "/preview.png"
  local jpg = dir .. "/preview.jpg"

  if file_exists(png) then
    return png
  end

  if file_exists(jpg) then
    return jpg
  end

  return first_image_in_dir(dir .. "/backgrounds")
end

local function collect_theme_dirs(theme_dir, themes, seen)
  local handle = io.popen("find -L '" .. theme_dir .. "' -mindepth 1 -maxdepth 1 -type d 2>/dev/null")
  if not handle then
    return
  end

  for path in handle:lines() do
    local theme_name = path:match(".*/(.+)$")
    if theme_name and theme_name:sub(1, 1) ~= "." then
      if not seen[theme_name] then
        seen[theme_name] = { name = theme_name, paths = {} }
        table.insert(themes, seen[theme_name])
      end

      table.insert(seen[theme_name].paths, path)
    end
  end

  handle:close()
end

local function expand_path(path)
  local home = os.getenv("HOME")
  local config_path = os.getenv("HYPRBOLE_CONFIG_PATH") or (home .. "/.config/hyprbole")

  if path == "~" then
    return home
  end

  if path:sub(1, 2) == "~/" then
    return home .. "/" .. path:sub(3)
  end

  if path == "$HOME" then
    return home
  end

  if path:sub(1, 6) == "$HOME/" then
    return home .. "/" .. path:sub(7)
  end

  if path == "$HYPRBOLE_CONFIG_PATH" then
    return config_path
  end

  if path:sub(1, 22) == "$HYPRBOLE_CONFIG_PATH/" then
    return config_path .. "/" .. path:sub(23)
  end

  return path
end

local function collect_theme_source_dirs(config_path)
  local source_dirs = {}
  local sources_path = config_path .. "/theme-sources.conf"
  local handle = io.open(sources_path, "r")

  if not handle then
    return source_dirs
  end

  for line in handle:lines() do
    local trimmed = line:match("^%s*(.-)%s*$")
    if trimmed ~= "" and trimmed:sub(1, 1) ~= "#" then
      local fields = {}
      for field in trimmed:gmatch("%S+") do
        table.insert(fields, field)
      end

      if fields[5] then
        table.insert(source_dirs, expand_path(fields[5]))
      end
    end
  end

  handle:close()
  return source_dirs
end

function GetEntries()
  local entries = {}
  local themes = {}
  local home = os.getenv("HOME")
  local hyprbole_path = os.getenv("HYPRBOLE_PATH") or (os.getenv("HOME") .. "/.local/share/hyprbole")
  local hyprbole_config_path = os.getenv("HYPRBOLE_CONFIG_PATH") or (home .. "/.config/hyprbole")
  local theme_source_dirs = collect_theme_source_dirs(hyprbole_config_path)
  local current_theme = ""
  local current_theme_file = hyprbole_config_path .. "/current/theme-name"
  local seen = {}

  if file_exists(current_theme_file) then
    local handle = io.open(current_theme_file, "r")
    if handle then
      current_theme = handle:read("*l") or ""
      handle:close()
    end
  end

  collect_theme_dirs(hyprbole_config_path .. "/themes", themes, seen)
  for _, theme_source_dir in ipairs(theme_source_dirs) do
    collect_theme_dirs(theme_source_dir, themes, seen)
  end
  collect_theme_dirs(hyprbole_path .. "/themes", themes, seen)

  table.insert(entries, {
    Text = "Add Theme Repository",
    Subtext = "Edit theme-sources.conf, then run theme source sync",
    Actions = {
      activate = "hyprbole theme source edit",
    },
    State = { "sublevel" },
  })

  for _, theme in ipairs(themes) do
      local theme_name = theme.name
      local preview_path = nil

      for _, path in ipairs(theme.paths) do
        preview_path = find_preview_path(path)
        if preview_path and preview_path ~= "" then
          break
        end
      end

      local display_name = theme_name:gsub("_", " "):gsub("%-", " ")
      display_name = display_name:gsub("(%a)([%w_']*)", function(first, rest)
        return first:upper() .. rest:lower()
      end)

      if theme_name == current_theme then
        display_name = display_name .. "  (Current)"
      end

      local entry = {
        Text = display_name,
        Actions = {
          activate = "hyprbole theme set " .. theme_name,
        },
        State = { "sublevel" },
      }

      if preview_path and preview_path ~= "" then
        entry.Preview = preview_path
        entry.PreviewType = "file"
      end

      table.insert(entries, entry)
  end

  table.sort(entries, function(a, b)
    return a.Text < b.Text
  end)

  return entries
end
