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
hb keybinds
```

or use `Help`, `Keybinds` in the Hyprbole menu.

## Common Defaults

As a rule of thumb, OS and Hyprland-level actions use `SUPER`: window management, workspaces, audio, networking, controls, and system actions. Everyday app launchers usually use `SUPER + SHIFT`.

There are a few practical exceptions. `H/J/K/L` and `S` are already used for window focus and scratchpad behavior, so some app and settings shortcuts use nearby or more specific combinations instead.

| Keybind | Action |
| --- | --- |
| `SUPER + SPACE` | Walker launcher |
| `SUPER + SHIFT + SPACE` | Walker launcher |
| `SUPER + ALT + SPACE` | Hyprbole menu |
| `SUPER + V` | Clipboard history |
| `SUPER + RETURN` | Terminal |
| `SUPER + SHIFT + B` | Browser |
| `SUPER + SHIFT + F` | File manager |
| `SUPER + SHIFT + N` | Obsidian, when installed |
| `SUPER + SHIFT + /` | 1Password, or install the 1Password extra |
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

Examples:

```lua
-- Launch a terminal file manager.
hl.bind("SUPER + E", hl.dsp.exec_cmd("hyprbole launch terminal yazi"))

-- Launch an app directly.
hl.bind("SUPER + SHIFT + C", hl.dsp.exec_cmd("code"))

-- Toggle floating for the focused window.
hl.bind("SUPER + SHIFT + T", hl.dsp.window.float({ action = "toggle" }))

-- Move the focused window to the scratchpad without following it.
hl.bind("SUPER + ALT + S", hl.dsp.window.move({ workspace = "special:scratchpad", follow = false }))

-- Focus a specific workspace.
hl.bind("SUPER + ALT + 1", hl.dsp.focus({ workspace = 1 }))

-- Move the focused window to a specific workspace.
hl.bind("SUPER + CTRL + 1", hl.dsp.window.move({ workspace = 1 }))

-- Add a mouse resize binding.
hl.bind("SUPER + CTRL + mouse:273", hl.dsp.window.resize(), { mouse = true })

-- Add a locked and repeating hardware-key style binding.
hl.bind("XF86AudioMicMute", hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"), { locked = true, repeating = true })
```

Hyprbole loads shipped bindings first and user bindings second, so local binds can extend or replace defaults without editing vendor files.

## Replacing A Binding

The safe rule is the same as Hyprland's normal config style: do not stack a new binding on top of an existing binding unless you intentionally want both actions on the same key.

Hyprbole exposes shipped default binding handles through `Hyprbole.bindings`, so user config can unbind a default and then add a replacement:

```lua
-- Replace the default browser binding.
Hyprbole.bindings.browser:unbind()
hl.bind("SUPER + SHIFT + W", hl.dsp.exec_cmd("hyprbole launch browser"))
```

Common default handles include `browser`, `terminal`, `files`, `launcher`, `menu`, `clipboard`, `close_window`, `toggle_floating`, `fullscreen`, `lock`, `logout`, `screenshot_region`, and `screenshot_screen`. Workspace defaults are grouped as `Hyprbole.bindings.focus_workspace[1]` through `[10]` and `Hyprbole.bindings.move_to_workspace[1]` through `[10]`.

View the shipped default bindings and handle names here:

```text
~/.local/share/hyprbole/default/hypr/core/bindings.lua
```

## Reload

After editing Hyprland config, run:

```bash
hyprctl reload
```

or use the refresh entries in the Hyprbole system menu.
