local theme_path = os.getenv("HOME") .. "/.config/hyprbole/current/theme/hyprland.lua"
local theme_file = io.open(theme_path, "r")

if theme_file then
  theme_file:close()
  local ok, err = pcall(dofile, theme_path)
  if not ok then
    io.stderr:write("failed to load Hyprbole theme Hyprland override: " .. tostring(err) .. "\n")
  end
end
