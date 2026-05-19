Name = "hyprbole-font-mono"
NamePretty = "Hyprbole Monospace Fonts"
HideFromProviderlist = true

local function shell_quote(value)
  return string.format("'%s'", tostring(value):gsub("'", "'\\''"))
end

local function current_font(role)
  local handle = io.popen("hyprbole theme font current " .. role .. " 2>/dev/null")
  if not handle then
    return ""
  end

  local output = handle:read("*l") or ""
  handle:close()
  return output
end

local function font_entries(role, command)
  local entries = {}
  local seen = {}
  local current = current_font(role)
  local handle = io.popen(command)

  if not handle then
    return entries
  end

  for line in handle:lines() do
    for family in line:gmatch("[^,]+") do
      local font_family = family:gsub("^%s+", ""):gsub("%s+$", "")
      if font_family ~= "" and not seen[font_family] then
        seen[font_family] = true

        local text = font_family
        if font_family == current then
          text = text .. "  (Current)"
        end

        table.insert(entries, {
          Text = text,
          Actions = {
            activate = "hyprbole theme font set " .. role .. " " .. shell_quote(font_family),
          },
        })
      end
    end
  end

  handle:close()
  table.sort(entries, function(a, b)
    return a.Text < b.Text
  end)

  return entries
end

function GetEntries()
  return font_entries("mono", "fc-list :spacing=100 family 2>/dev/null")
end
