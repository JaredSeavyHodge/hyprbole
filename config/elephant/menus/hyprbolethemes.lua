Name = "hyprbolethemes"
NamePretty = "Hyprbole Themes"
HideFromProviderlist = true

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

function GetEntries()
  local entries = {}
  local hyprbole_path = os.getenv("HYPRBOLE_PATH") or (os.getenv("HOME") .. "/.local/share/hyprbole")
  local theme_dir = hyprbole_path .. "/themes"
  local current_theme = ""
  local current_theme_file = os.getenv("HOME") .. "/.config/hyprbole/current/theme-name"
  local seen = {}

  if file_exists(current_theme_file) then
    local handle = io.open(current_theme_file, "r")
    if handle then
      current_theme = handle:read("*l") or ""
      handle:close()
    end
  end

  local handle = io.popen("find -L '" .. theme_dir .. "' -mindepth 1 -maxdepth 1 -type d 2>/dev/null")
  if not handle then
    return entries
  end

  for path in handle:lines() do
    local theme_name = path:match(".*/(.+)$")
    if theme_name and not seen[theme_name] then
      seen[theme_name] = true

      local preview_path = find_preview_path(path)
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
      }

      if preview_path and preview_path ~= "" then
        entry.Preview = preview_path
        entry.PreviewType = "file"
      end

      table.insert(entries, entry)
    end
  end

  handle:close()
  table.sort(entries, function(a, b)
    return a.Text < b.Text
  end)

  return entries
end
