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
| `SUPER + SHIFT + B` | Browser |
| `SUPER + SHIFT + F` | File manager |
| `SUPER + SHIFT + N` | Obsidian |
| `SUPER + SHIFT + /` | 1Password |
| `SUPER + Q` | Close focused window |
| `SUPER + T` | Toggle floating window |
| `SUPER + F` | Toggle fullscreen |
| `SUPER + H/J/K/L` | Move focus left/down/up/right |
| `SUPER + 1..0` | Focus workspace 1 through 10 |
| `SUPER + SHIFT + 1..0` | Move focused window to workspace 1 through 10 |
| `SUPER + S` | Toggle scratchpad workspace |
| `SUPER + SHIFT + S` | Move focused window to scratchpad |
| `SUPER + SHIFT + L` | Lock |
| `SUPER + CTRL + L` | Logout |
| `Print` | Region screenshot |
| `Shift + Print` | Fullscreen screenshot |
| `SUPER + SHIFT + R` | Toggle region recording |
| `SUPER + SHIFT + ALT + R` | Toggle screen recording |
| `SUPER + SHIFT + A` | Audio settings |
| `SUPER + N` | Network settings |
| `SUPER + SHIFT + ALT + B` | Bluetooth settings |

Mouse bindings:

| Keybind | Action |
| --- | --- |
| `SUPER + left mouse drag` | Move window |
| `SUPER + right mouse drag` | Resize window |

Media and brightness keys are also bound when available.

## Add A Binding

Edit:

```text
~/.config/hypr/bindings.lua
```

Example:

```lua
hl.bind("SUPER + E", hl.dsp.exec_cmd("hyprbole-launch-terminal yazi"))
```

Hyprbole loads shipped bindings first and user bindings second, so local binds can extend the defaults without editing vendor files.

## Reload

After editing Hyprland config, run:

```bash
hyprctl reload
```

or use the refresh entries in the Hyprbole system menu.
