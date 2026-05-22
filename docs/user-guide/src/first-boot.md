# First Boot

Log in from SDDM. Hyprbole configures SDDM to start Hyprland through UWSM, which loads the Lua config entrypoint at `~/.config/hypr/hyprland.lua`.

On a fresh install, Hyprbole opens the local guide automatically on the first Hyprland login. If it does not open, press `SUPER + RETURN` and run `hb docs`.

## First Five Minutes

1. Make sure the guide opened, or run `hb docs`.
2. Press `SUPER + SPACE` to open the app launcher.
3. Press `SUPER + ALT + SPACE` to open the Hyprbole menu.
4. Press `SUPER + RETURN` to open a terminal.
5. Run the first checks below.

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

To rebuild and reopen this guide later, run:

```bash
hb docs
```

The generated guide is written to `~/.config/hyprbole/docs/index.html`.

## First Files To Know

Hyprbole copies editable defaults into `~/.config` only when missing. Start with these files when you want to tune monitors, input, keybinds, or startup behavior:

| File | Purpose |
| --- | --- |
| `~/.config/hypr/monitors.lua` | Monitor layout and scale overrides |
| `~/.config/hypr/input.lua` | Keyboard, pointer, and touchpad overrides |
| `~/.config/hypr/bindings.lua` | Personal keybind additions |
| `~/.config/hypr/looknfeel.lua` | Local gaps, borders, blur, and decoration tweaks |
| `~/.config/hypr/autostart.lua` | Personal startup commands |
| `~/.config/hypr/overrides.lua` | One-off local experiments |

Run `hyprctl reload` after editing Hyprland config. If you break one file, see [Troubleshooting](troubleshooting.md) for `hb refresh-config` examples.

## If Something Looks Incomplete

Run:

```bash
hb doctor --verbose
```

If it reports missing generated theme state, browser launchers, or runtime state, run:

```bash
hb refresh-all
hb doctor
```

If the Waybar health bell is orange or red, run:

```bash
hb health
```

Right-clicking the bell runs a fresh check.
