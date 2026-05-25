local hyprbole_path = os.getenv("HYPRBOLE_PATH") or (os.getenv("HOME") .. "/.local/share/hyprbole")

local hyprbole = hyprbole_path .. "/bin/hyprbole"

local terminal = hyprbole .. " launch terminal"
local browser = hyprbole .. " launch browser"
local file_manager = hyprbole .. " launch files"
local obsidian = hyprbole .. " launch obsidian"
local launcher = hyprbole .. " launch launcher"

local audio_up = hyprbole_path .. "/bin/hyprbole-audio-up"
local audio_down = hyprbole_path .. "/bin/hyprbole-audio-down"
local audio_mute = hyprbole_path .. "/bin/hyprbole-audio-mute"
local brightness_up = hyprbole_path .. "/bin/hyprbole-brightness-up"
local brightness_down = hyprbole_path .. "/bin/hyprbole-brightness-down"

local screenshot_region = hyprbole_path .. "/bin/hyprbole-screenshot region"
local screenshot_screen = hyprbole_path .. "/bin/hyprbole-screenshot screen"
local record_region = hyprbole_path .. "/bin/hyprbole-record-toggle region"
local record_screen = hyprbole_path .. "/bin/hyprbole-record-toggle screen"
local system_lock = hyprbole .. " system lock"
local system_logout = hyprbole .. " system logout"

local launch_audio = hyprbole .. " launch audio"
local launch_network = hyprbole .. " launch network"
local launch_bluetooth = hyprbole .. " launch bluetooth"
local launch_passwords = hyprbole .. " launch passwords"
local launch_clipboard = hyprbole .. " launch clipboard"
local launch_menu = hyprbole .. " launch menu"
local bindings = {}

bindings.terminal = hl.bind("SUPER + RETURN", hl.dsp.exec_cmd(terminal))
bindings.browser = hl.bind("SUPER + SHIFT + B", hl.dsp.exec_cmd(browser))
bindings.files = hl.bind("SUPER + SHIFT + F", hl.dsp.exec_cmd(file_manager))
bindings.obsidian = hl.bind("SUPER + SHIFT + N", hl.dsp.exec_cmd(obsidian))
bindings.passwords = hl.bind("SUPER + SHIFT + SLASH", hl.dsp.exec_cmd(launch_passwords))
bindings.clipboard = hl.bind("SUPER + V", hl.dsp.exec_cmd(launch_clipboard))
bindings.menu = hl.bind("SUPER + ALT + SPACE", hl.dsp.exec_cmd(launch_menu))
bindings.launcher = hl.bind("SUPER + SPACE", hl.dsp.exec_cmd(launcher))
bindings.launcher_alt = hl.bind("SUPER + SHIFT + SPACE", hl.dsp.exec_cmd(launcher))
bindings.close_window = hl.bind("SUPER + Q", hl.dsp.window.close())
bindings.toggle_scratchpad = hl.bind("SUPER + S", hl.dsp.workspace.toggle_special("scratchpad"))
bindings.move_to_scratchpad = hl.bind("SUPER + SHIFT + S", hl.dsp.window.move({ workspace = "special:scratchpad", follow = false }))
bindings.toggle_floating = hl.bind("SUPER + T", hl.dsp.window.float({ action = "toggle" }))
bindings.fullscreen = hl.bind("SUPER + F", hl.dsp.window.fullscreen())
bindings.focus_left = hl.bind("SUPER + H", hl.dsp.focus({ direction = "left" }))
bindings.focus_right = hl.bind("SUPER + L", hl.dsp.focus({ direction = "right" }))
bindings.focus_up = hl.bind("SUPER + K", hl.dsp.focus({ direction = "up" }))
bindings.focus_down = hl.bind("SUPER + J", hl.dsp.focus({ direction = "down" }))
bindings.drag_window = hl.bind("SUPER + mouse:272", hl.dsp.window.drag(), { mouse = true })
bindings.resize_window = hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })

bindings.focus_workspace = {}
bindings.move_to_workspace = {}
for i = 1, 10 do
  local key = i % 10
  bindings.focus_workspace[i] = hl.bind("SUPER + " .. key, hl.dsp.focus({ workspace = i }))
  bindings.move_to_workspace[i] = hl.bind("SUPER + SHIFT + " .. key, hl.dsp.window.move({ workspace = i }))
end

bindings.audio_up = hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd(audio_up), { locked = true, repeating = true })
bindings.audio_down = hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd(audio_down), { locked = true, repeating = true })
bindings.audio_mute = hl.bind("XF86AudioMute", hl.dsp.exec_cmd(audio_mute), { locked = true, repeating = true })
bindings.mic_mute = hl.bind("XF86AudioMicMute", hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"), { locked = true, repeating = true })
bindings.brightness_up = hl.bind("XF86MonBrightnessUp", hl.dsp.exec_cmd(brightness_up), { locked = true, repeating = true })
bindings.brightness_down = hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd(brightness_down), { locked = true, repeating = true })
bindings.media_next = hl.bind("XF86AudioNext", hl.dsp.exec_cmd("playerctl next"), { locked = true })
bindings.media_pause = hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
bindings.media_play = hl.bind("XF86AudioPlay", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
bindings.media_prev = hl.bind("XF86AudioPrev", hl.dsp.exec_cmd("playerctl previous"), { locked = true })

bindings.screenshot_region = hl.bind("Print", hl.dsp.exec_cmd(screenshot_region))
bindings.lock = hl.bind("SUPER + SHIFT + L", hl.dsp.exec_cmd(system_lock))
bindings.logout = hl.bind("SUPER + CTRL + L", hl.dsp.exec_cmd(system_logout))
bindings.screenshot_screen = hl.bind("SHIFT + Print", hl.dsp.exec_cmd(screenshot_screen))
bindings.record_region = hl.bind("SUPER + SHIFT + R", hl.dsp.exec_cmd(record_region))
bindings.record_screen = hl.bind("SUPER + SHIFT + ALT + R", hl.dsp.exec_cmd(record_screen))
bindings.audio_settings = hl.bind("SUPER + SHIFT + A", hl.dsp.exec_cmd(launch_audio))
bindings.network_settings = hl.bind("SUPER + N", hl.dsp.exec_cmd(launch_network))
bindings.bluetooth_settings = hl.bind("SUPER + SHIFT + ALT + B", hl.dsp.exec_cmd(launch_bluetooth))

_G.Hyprbole = _G.Hyprbole or {}
_G.Hyprbole.bindings = bindings

return bindings
