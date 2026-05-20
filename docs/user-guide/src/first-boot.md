# First Boot

Log in from SDDM. Hyprbole configures SDDM to start Hyprland through UWSM, which loads the Lua config entrypoint at `~/.config/hypr/hyprland.lua`.

## First Checks

Open a terminal and run:

```bash
hb doctor
hb verify
```

`doctor` gives a readable health report. `verify` is stricter and is useful after install, updates, and repairs.

## Default Apps

- Terminal: Ghostty
- Browser: Brave Origin Nightly through the Hyprbole launcher
- File manager: Nautilus
- Launcher: Walker
- Notifications: SwayNC
- Password manager: 1Password

## Main Shortcuts

- `SUPER + SPACE`: app launcher
- `SUPER + ALT + SPACE`: Hyprbole menu
- `SUPER + V`: clipboard history
- `SUPER + SHIFT + B`: browser
- `SUPER + RETURN`: terminal
- `Print`: region screenshot
- `Shift + Print`: full screen screenshot

## Window Management Shortcuts

- `SUPER + Q`: close focused window
- `SUPER + T`: toggle floating window
- `SUPER + F`: toggle fullscreen
- `SUPER + H/J/K/L`: move focus left/down/up/right
- `SUPER + 1..0`: focus workspace 1 through 10
- `SUPER + SHIFT + 1..0`: move focused window to workspace 1 through 10
- `SUPER + S`: toggle scratchpad workspace
- `SUPER + SHIFT + S`: move focused window to scratchpad
- `SUPER + left mouse drag`: move window
- `SUPER + right mouse drag`: resize window

Use the Keybinds entry in the Hyprbole menu to browse the current shortcuts.

## Open The Guide

Build and open the local guide with:

```bash
hb docs
```

The generated guide is written to `~/.config/hyprbole/docs/index.html`.

## Check Default Files

Hyprbole copies editable defaults into `~/.config` only when missing. The most useful first Hyprland files to inspect are:

| File | Purpose |
| --- | --- |
| `~/.config/hypr/monitors.lua` | Monitor layout and scale overrides |
| `~/.config/hypr/input.lua` | Keyboard, pointer, and touchpad overrides |
| `~/.config/hypr/bindings.lua` | Personal keybind additions |
| `~/.config/hypr/looknfeel.lua` | Local gaps, borders, blur, and decoration tweaks |
| `~/.config/hypr/autostart.lua` | Personal startup commands |
| `~/.config/hypr/overrides.lua` | One-off local experiments |

Run `hyprctl reload` after editing Hyprland config.

## If Something Looks Incomplete

Run:

```bash
hb doctor --verbose
```

If it reports missing generated theme state or browser launchers, run:

```bash
hb refresh-all
hb doctor
```
