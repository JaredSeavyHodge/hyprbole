# Keybinds

Hyprbole ships default Hyprland keybinds under:

```text
~/.local/share/hyprbole/default/hypr/core/bindings.lua
```

User overrides and additions live under:

```text
~/.config/hypr/bindings.lua
```

## Browse Keybinds

Open the keybind browser:

```bash
hyprbole keybinds
```

or use the Keybinds entry in the Hyprbole menu.

## Common Defaults

| Keybind | Action |
| --- | --- |
| `SUPER + SPACE` | Walker launcher |
| `SUPER + ALT + SPACE` | Hyprbole menu |
| `SUPER + V` | Clipboard history |
| `SUPER + RETURN` | Terminal |
| `SUPER + SHIFT + RETURN` | Browser |
| `SUPER + SHIFT + F` | File manager |
| `SUPER + SHIFT + /` | 1Password |
| `Print` | Region screenshot |
| `Shift + Print` | Fullscreen screenshot |

## Reload

After editing Hyprland config, run:

```bash
hyprctl reload
```

or use the refresh entries in the Hyprbole system menu.
