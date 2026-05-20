# First Boot

Start a Hyprbole session from SDDM. Hyprland is launched through UWSM and loads the Lua config entrypoint at `~/.config/hypr/hyprland.lua`.

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
- `SUPER + SHIFT + RETURN`: browser
- `SUPER + RETURN`: terminal
- `Print`: region screenshot
- `Shift + Print`: full screen screenshot

Use the Keybinds entry in the Hyprbole menu to browse the current shortcuts.
