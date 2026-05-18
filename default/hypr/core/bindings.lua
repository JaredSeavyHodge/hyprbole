local hyprbole_path = os.getenv("HYPRBOLE_PATH") or (os.getenv("HOME") .. "/.local/share/hyprbole")

local terminal = hyprbole_path .. "/bin/hyprbole-launch-terminal"
local browser = hyprbole_path .. "/bin/hyprbole-launch-browser"
local file_manager = hyprbole_path .. "/bin/hyprbole-launch-files"
local obsidian = "obsidian"
local launcher = hyprbole_path .. "/bin/hyprbole-launch-walker"

local audio_up = hyprbole_path .. "/bin/hyprbole-audio-up"
local audio_down = hyprbole_path .. "/bin/hyprbole-audio-down"
local audio_mute = hyprbole_path .. "/bin/hyprbole-audio-mute"
local brightness_up = hyprbole_path .. "/bin/hyprbole-brightness-up"
local brightness_down = hyprbole_path .. "/bin/hyprbole-brightness-down"

local screenshot_region = hyprbole_path .. "/bin/hyprbole-screenshot region"
local screenshot_screen = hyprbole_path .. "/bin/hyprbole-screenshot screen"
local record_region = hyprbole_path .. "/bin/hyprbole-record-toggle region"
local record_screen = hyprbole_path .. "/bin/hyprbole-record-toggle screen"
local system_lock = hyprbole_path .. "/bin/hyprbole-system-lock"
local system_logout = hyprbole_path .. "/bin/hyprbole-system-logout"

local launch_audio = hyprbole_path .. "/bin/hyprbole-launch-audio"
local launch_network = hyprbole_path .. "/bin/hyprbole-launch-network"
local launch_bluetooth = hyprbole_path .. "/bin/hyprbole-launch-bluetooth"
local launch_passwords = hyprbole_path .. "/bin/hyprbole-launch-passwords"
local launch_clipboard = hyprbole_path .. "/bin/hyprbole-launch-clipboard"
local launch_menu = hyprbole_path .. "/bin/hyprbole-launch-menu"

hl.bind("SUPER + RETURN", hl.dsp.exec_cmd(terminal))
hl.bind("SUPER + SHIFT + RETURN", hl.dsp.exec_cmd(browser))
hl.bind("SUPER + SHIFT + B", hl.dsp.exec_cmd(browser))
hl.bind("SUPER + SHIFT + F", hl.dsp.exec_cmd(file_manager))
hl.bind("SUPER + SHIFT + N", hl.dsp.exec_cmd(obsidian))
hl.bind("SUPER + SHIFT + SLASH", hl.dsp.exec_cmd(launch_passwords))
hl.bind("SUPER + V", hl.dsp.exec_cmd(launch_clipboard))
hl.bind("SUPER + ALT + SPACE", hl.dsp.exec_cmd(launch_menu))
hl.bind("SUPER + SPACE", hl.dsp.exec_cmd(launcher))
hl.bind("SUPER + Q", hl.dsp.window.close())
hl.bind("SUPER + S", hl.dsp.workspace.toggle_special("scratchpad"))
hl.bind("SUPER + SHIFT + S", hl.dsp.window.move({ workspace = "special:scratchpad", follow = false }))
hl.bind("SUPER + T", hl.dsp.window.float({ action = "toggle" }))
hl.bind("SUPER + F", hl.dsp.window.fullscreen())
hl.bind("SUPER + H", hl.dsp.focus({ direction = "left" }))
hl.bind("SUPER + L", hl.dsp.focus({ direction = "right" }))
hl.bind("SUPER + K", hl.dsp.focus({ direction = "up" }))
hl.bind("SUPER + J", hl.dsp.focus({ direction = "down" }))
hl.bind("SUPER + mouse:272", hl.dsp.window.drag(), { mouse = true })
hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })

for i = 1, 10 do
  local key = i % 10
  hl.bind("SUPER + " .. key, hl.dsp.focus({ workspace = i }))
  hl.bind("SUPER + SHIFT + " .. key, hl.dsp.window.move({ workspace = i }))
end

hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd(audio_up), { locked = true, repeating = true })
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd(audio_down), { locked = true, repeating = true })
hl.bind("XF86AudioMute", hl.dsp.exec_cmd(audio_mute), { locked = true, repeating = true })
hl.bind("XF86AudioMicMute", hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"), { locked = true, repeating = true })
hl.bind("XF86MonBrightnessUp", hl.dsp.exec_cmd(brightness_up), { locked = true, repeating = true })
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd(brightness_down), { locked = true, repeating = true })
hl.bind("XF86AudioNext", hl.dsp.exec_cmd("playerctl next"), { locked = true })
hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPlay", hl.dsp.exec_cmd("playerctl play-pause"), { locked = true })
hl.bind("XF86AudioPrev", hl.dsp.exec_cmd("playerctl previous"), { locked = true })

hl.bind("Print", hl.dsp.exec_cmd(screenshot_region))
hl.bind("SUPER + SHIFT + L", hl.dsp.exec_cmd(system_lock))
hl.bind("SUPER + CTRL + L", hl.dsp.exec_cmd(system_logout))
hl.bind("SHIFT + Print", hl.dsp.exec_cmd(screenshot_screen))
hl.bind("SUPER + SHIFT + R", hl.dsp.exec_cmd(record_region))
hl.bind("SUPER + SHIFT + ALT + R", hl.dsp.exec_cmd(record_screen))
hl.bind("SUPER + SHIFT + A", hl.dsp.exec_cmd(launch_audio))
hl.bind("SUPER + N", hl.dsp.exec_cmd(launch_network))
hl.bind("SUPER + SHIFT + ALT + B", hl.dsp.exec_cmd(launch_bluetooth))
