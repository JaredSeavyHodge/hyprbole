# First Boot

Start a Hyprbole session from SDDM. Hyprland is launched through UWSM and loads the Lua config entrypoint at `~/.config/hypr/hyprland.lua`.

In SDDM, choose `Hyprland (uwsm)` if more than one Hyprland session is listed.

## First Checks

Open a terminal and run:

```bash
hyprbole doctor
hyprbole verify
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

Use the Keybinds entry in the Hyprbole menu to browse the current shortcuts.

## Open The Guide

Build and open the local guide with:

```bash
hyprbole docs
```

The generated guide is written to `~/.config/hyprbole/docs/index.html`.

## Check Default Files

Hyprbole copies editable defaults into `~/.config` only when missing. The most useful first files to inspect are:

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
hyprbole doctor --verbose
```

If it reports missing generated theme state or browser launchers, run:

```bash
hyprbole refresh-all
hyprbole doctor
```
