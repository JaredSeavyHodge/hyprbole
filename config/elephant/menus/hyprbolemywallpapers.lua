Name = "hyprbolemywallpapers"
NamePretty = "Hyprbole My Wallpapers"
HideFromProviderlist = true

local function shell_quote(value)
  return string.format("'%s'", tostring(value):gsub("'", "'\\''"))
end

function GetEntries()
  local entries = {}
  local home = os.getenv("HOME")
  local wallpaper_dir = home .. "/Pictures/Wallpaper"
  local handle = io.popen("find -L '" .. wallpaper_dir .. "' -type f \\( -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.png' -o -iname '*.webp' \\) 2>/dev/null | sort")

  if not handle then
    return entries
  end

  for path in handle:lines() do
    local wallpaper_name = path:match(".*/(.+)$")
    if wallpaper_name then
      table.insert(entries, {
        Text = wallpaper_name,
        Sub = path:gsub("^" .. home, "~"),
        Preview = path,
        PreviewType = "file",
        Actions = {
          activate = "hyprbole wallpaper set " .. shell_quote(path),
        },
      })
    end
  end

  handle:close()
  return entries
end
