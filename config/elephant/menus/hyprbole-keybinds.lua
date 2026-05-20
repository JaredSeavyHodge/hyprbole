Name = "hyprbole-keybinds"
NamePretty = "Hyprbole Keybinds"
HideFromProviderlist = true
Parent = "hyprbole-help"
FixedOrder = true

local function title_case(text)
  return (text:gsub("(%a)([%w_']*)", function(first, rest)
    return first:upper() .. rest:lower()
  end))
end

local function command_label(command)
  local known = {
    ["obsidian"] = "Open Obsidian",
    ["playerctl next"] = "Next track",
    ["playerctl play-pause"] = "Play or pause",
    ["playerctl previous"] = "Previous track",
    ["wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"] = "Toggle microphone mute",
  }

  if known[command] then
    return known[command]
  end

  local parts = {}
  for part in command:gmatch("%S+") do
    table.insert(parts, part)
  end

  local executable = parts[1] or command
  executable = executable:match(".*/([^/]+)$") or executable

  if executable:match("^hyprbole%-%") then
    local label = executable:gsub("^hyprbole%-", ""):gsub("%-", " ")
    label = title_case(label)

    if #parts > 1 then
      local args = {}
      for i = 2, #parts do
        table.insert(args, parts[i])
      end
      return string.format("%s (%s)", label, table.concat(args, " "))
    end

    return label
  end

  if #parts > 1 then
    local args = {}
    for i = 2, #parts do
      table.insert(args, parts[i])
    end
    return string.format("%s (%s)", title_case(executable:gsub("%-", " ")), table.concat(args, " "))
  end

  return title_case(executable:gsub("%-", " "))
end

local function action_label(action)
  if type(action) ~= "table" or not action.__hyprbole_action then
    return "Custom action"
  end

  local options = action.args[1]

  if action.path == "dsp.exec_cmd" then
    return command_label(action.args[1] or "")
  elseif action.path == "dsp.window.close" then
    return "Close window"
  elseif action.path == "dsp.window.fullscreen" then
    return "Toggle fullscreen"
  elseif action.path == "dsp.window.drag" then
    return "Drag window"
  elseif action.path == "dsp.window.resize" then
    return "Resize window"
  elseif action.path == "dsp.window.float" and type(options) == "table" and options.action == "toggle" then
    return "Toggle floating"
  elseif action.path == "dsp.workspace.toggle_special" then
    return string.format("Toggle %s", tostring(action.args[1] or "special workspace"))
  elseif action.path == "dsp.focus" and type(options) == "table" and options.direction then
    return string.format("Focus %s", tostring(options.direction))
  elseif action.path == "dsp.focus" and type(options) == "table" and options.workspace then
    return string.format("Focus workspace %s", tostring(options.workspace))
  elseif action.path == "dsp.window.move" and type(options) == "table" and options.workspace then
    return string.format("Move window to %s", tostring(options.workspace))
  end

  return title_case((action.path or "action"):gsub("^dsp%.", ""):gsub("[._]", " "))
end

local function proxy(path)
  return setmetatable({}, {
    __index = function(_, key)
      return proxy(path .. "." .. key)
    end,
    __call = function(_, ...)
      return {
        __hyprbole_action = true,
        path = path,
        args = { ... },
      }
    end,
  })
end

local function collect_bindings()
  local bindings = {}
  local home = os.getenv("HOME")
  local hyprbole_path = os.getenv("HYPRBOLE_PATH") or (home .. "/.local/share/hyprbole")

  package.path = home .. "/.config/?.lua;" .. hyprbole_path .. "/?.lua;" .. package.path

  hl = proxy("hl")
  hl.dsp = proxy("dsp")
  hl.bind = function(keys, action)
    local binding = {
      unbind = function() end,
      remove = function() end,
      set_enabled = function() end,
      is_enabled = function() return true end,
    }

    table.insert(bindings, {
      keys = keys,
      action = action_label(action),
    })

    return binding
  end

  package.loaded["default.hypr.core.bindings"] = nil
  package.loaded["hypr.bindings"] = nil
  package.loaded["hypr.overrides"] = nil

  local ok, err = pcall(function()
    require("default.hypr.core.bindings")
    require("hypr.bindings")
    pcall(require, "hypr.overrides")
  end)

  if not ok then
    return nil, err
  end

  return bindings
end

function GetEntries()
  local entries = {}
  local bindings, err = collect_bindings()

  if not bindings then
    return {
      {
        Text = "Unable to load keybinds",
        Subtext = tostring(err),
        Actions = {
          activate = "true",
        },
        State = { "sublevel" },
      },
    }
  end

  for _, binding in ipairs(bindings) do
    table.insert(entries, {
      Text = string.format("%s  -  %s", binding.keys, binding.action),
      Subtext = binding.action,
      Actions = {
        activate = "true",
      },
      State = { "sublevel" },
    })
  end

  return entries
end
