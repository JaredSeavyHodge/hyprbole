# Hyprland

Hyprbole uses Hyprland's Lua config path. The entrypoint is:

```text
~/.config/hypr/hyprland.lua
```

That file loads Hyprbole's shipped defaults first, then user-editable files under `~/.config/hypr`.

## Load Order

Hyprbole loads config in this order:

| Step | File |
| --- | --- |
| 1 | `~/.local/share/hyprbole/default/hypr/hyprbole.lua` |
| 2 | `~/.config/hypr/monitors.lua` |
| 3 | `~/.config/hypr/input.lua` |
| 4 | `~/.config/hypr/bindings.lua` |
| 5 | `~/.config/hypr/looknfeel.lua` |
| 6 | `~/.config/hypr/autostart.lua` |
| 7 | `~/.config/hypr/overrides.lua` |

Shipped defaults live under:

```text
~/.local/share/hyprbole/default/hypr/core
```

Edit files under `~/.config/hypr` for your machine. Treat files under `default/hypr/core` as Hyprbole-owned unless you are changing Hyprbole itself.

## User Config Files

| File | Purpose |
| --- | --- |
| `monitors.lua` | Monitor layout, mode, scale, reserved areas, HDR-related options |
| `input.lua` | Keyboard, pointer, touchpad, repeat rate, device input settings |
| `bindings.lua` | Personal keybinds and default keybind overrides |
| `looknfeel.lua` | Gaps, borders, layout, blur, opacity, animations, decoration |
| `autostart.lua` | Personal startup commands |
| `overrides.lua` | One-off local rules or experiments loaded last |

## Monitors

Default monitor config uses the preferred mode, automatic position, and automatic scale:

```lua
hl.monitor({
  output = "",
  mode = "preferred",
  position = "auto",
  scale = "auto",
})
```

Add monitor overrides in `~/.config/hypr/monitors.lua`:

```lua
hl.monitor({
  output = "DP-1",
  mode = "2560x1440@144",
  position = "0x0",
  scale = 1,
})

hl.monitor({
  output = "HDMI-A-1",
  mode = "1920x1080@60",
  position = "2560x0",
  scale = 1,
})
```

Use monitor names from Hyprland:

```bash
hyprctl monitors
```

## Input

Hyprbole's shipped input defaults include Caps Lock as Compose, fast key repeat, numlock by default, focus follows mouse, and touchpad click-finger behavior.

Add local input changes in `~/.config/hypr/input.lua`:

```lua
hl.config({
  input = {
    kb_layout = "us",
    repeat_rate = 40,
    repeat_delay = 300,
    touchpad = {
      natural_scroll = true,
      tap_to_click = true,
      clickfinger_behavior = true,
    },
  },
})
```

## Keybindings

Hyprbole loads shipped keybindings before `~/.config/hypr/bindings.lua`, so that file is where personal bindings and default binding overrides belong.

Common defaults include `SUPER + SPACE` for the app launcher, `SUPER + RETURN` for the terminal, `SUPER + SHIFT + B` for the browser, `SUPER + SHIFT + F` for files, and `SUPER + Q` to close the focused window.

For the full list and override examples, see the [Keybinds](keybinds.md) guide or run:

```bash
hb keybinds
```

## Layout And Look

Hyprbole's shipped layout is `dwindle` with preserved splits. Look-and-feel defaults also set gaps, borders, rounding, inactive opacity, shadow, blur, and animations.

Add local changes in `~/.config/hypr/looknfeel.lua`:

```lua
hl.config({
  general = {
    gaps_in = 6,
    gaps_out = 12,
    border_size = 2,
    layout = "dwindle",
  },
  decoration = {
    rounding = 12,
    inactive_opacity = 0.9,
    blur = {
      enabled = true,
      size = 6,
      passes = 2,
    },
  },
  dwindle = {
    preserve_split = true,
  },
})
```

## Window Rules

Shipped window rules live in:

```text
~/.local/share/hyprbole/default/hypr/core/rules.lua
```

Current shipped rules suppress maximize events, work around some XWayland drag windows, float Hyprbole terminal helpers, float Satty, and send 1Password to the scratchpad while preventing it from screen sharing.

Add local rules in `~/.config/hypr/overrides.lua`:

```lua
hl.window_rule({
  name = "float-calculator",
  match = { class = "^org\.gnome\.Calculator$" },
  float = true,
})
```

Another common pattern is assigning an app to a workspace:

```lua
hl.window_rule({
  name = "browser-workspace",
  match = { class = "^Brave-browser$" },
  workspace = "2",
})
```

Use `hyprctl clients` to inspect current window classes and titles.

## Autostart

Hyprbole starts core session pieces from shipped autostart defaults: environment import, Waybar restart, wallpaper daemon restart, and `blueman-applet`.

Add personal startup commands in `~/.config/hypr/autostart.lua`:

```lua
hl.on("hyprland.start", function()
  hl.exec_cmd("copyq")
end)
```

For delayed startup:

```lua
hl.on("hyprland.start", function()
  hl.exec_cmd("bash -lc 'sleep 2; localsend_app'")
end)
```

Keep long-running or critical services in systemd user units when possible.

## Theme Overrides

Themes can include a source `hyprland.lua` file for theme-specific Hyprland settings. For user-authored themes, that file lives in the theme source directory:

```text
~/.config/hyprbole/themes/<theme>/hyprland.lua
```

When a theme is applied, Hyprbole copies merged theme files into `~/.config/hyprbole/current/theme`. Do not edit files under `current/theme`; they are runtime state and can be replaced when switching or reapplying themes.

Hyprbole loads the active theme's generated Hyprland override before user files. Use user config under `~/.config/hypr` when you want your local machine to win over theme defaults.

## Reload

After editing Hyprland config, reload:

```bash
hyprctl reload
```

If a change breaks the session, fix the edited file from a terminal TTY or restore it from the Hyprbole repo defaults.
