# First Boot

Log in from SDDM. Hyprbole configures SDDM to start Hyprland through UWSM, which loads the Lua config entrypoint at `~/.config/hypr/hyprland.lua`.

## First Checks

Open a terminal and run:

```bash
hb doctor
hb health
hb verify
```

`doctor` gives a readable desktop report. `health` shows the cached hourly disk, OS, and update-freshness report. `verify` is stricter and is useful after install, updates, and repairs.

## Learn The Desktop

Read [Quick Start](quick-start.md) for basic desktop usage: launching apps, switching workspaces, moving windows, using the top bar, and taking screenshots.

Use `Help`, `Keybinds` in the Hyprbole menu when you want the complete shortcut list.

## Open The Guide

On a fresh install, Hyprbole opens this local guide automatically on the first Hyprland login.

To rebuild and reopen it later, run:

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

If the Waybar health bell is orange or red, run:

```bash
hb health
```

Right-clicking the bell runs a fresh check.
