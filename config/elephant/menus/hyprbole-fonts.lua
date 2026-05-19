Name = "hyprbole-fonts"
NamePretty = "Hyprbole Fonts"
HideFromProviderlist = true

local function shell_quote(value)
  return string.format("'%s'", tostring(value):gsub("'", "'\\''"))
end

local function current_fonts()
  local fonts = {}
  local handle = io.popen("hyprbole theme font current 2>/dev/null")
  if not handle then
    return fonts
  end

  for line in handle:lines() do
    local role, family = line:match("^([^=]+)=(.*)$")
    if role and family then
      fonts[role] = family
    end
  end

  handle:close()
  return fonts
end

local function all_current(fonts, family)
  return fonts.ui == family and fonts.mono == family and fonts.terminal == family
end

function GetEntries()
  local entries = {}
  local seen = {}
  local fonts = current_fonts()
  local handle = io.popen("fc-list : family 2>/dev/null")

  if not handle then
    return entries
  end

  for line in handle:lines() do
    for family in line:gmatch("[^,]+") do
      local font_family = family:gsub("^%s+", ""):gsub("%s+$", "")
      if font_family ~= "" and not seen[font_family] then
        seen[font_family] = true

        local text = font_family
        if all_current(fonts, font_family) then
          text = text .. "  (Current)"
        end

        table.insert(entries, {
          Text = text,
          Actions = {
            activate = "hyprbole theme font set all " .. shell_quote(font_family),
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
