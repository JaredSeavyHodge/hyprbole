# Welcome To Hyprbole

Hyprbole is the desktop layer that runs after Arch Linux is installed. It owns the Hyprland session, default apps, themes, keybinds, browser launcher, and repair tools.

Use this guide when you want to understand what Hyprbole installed, how to change the everyday defaults, and how to repair the common desktop services.

## Start Here

- Run `hyprbole doctor` to check the desktop state.
- Run `hyprbole verify` after install or major changes.
- Run `hyprbole doctor --fix` for supported repairs.
- Run `hyprbole docs` to build and open this guide locally.
- Open the launcher with `SUPER + SPACE`.
- Open the Hyprbole menu with `SUPER + ALT + SPACE`.

## What Hyprbole Owns

- Hyprland with Lua configuration.
- Waybar status bar.
- Walker launcher and clipboard picker.
- SwayNC notifications.
- Ghostty terminal.
- Brave Origin Nightly browser through a Hyprbole wrapper.
- Nautilus file manager.
- Theme and wallpaper state under `~/.config/hyprbole/current`.
- Secret Service setup for GNOME Keyring/libsecret.

## Important Paths

| Path | Purpose |
| --- | --- |
| `~/.local/share/hyprbole` | Hyprbole git checkout and vendor defaults |
| `~/.config` | User-editable config copied from `config/` |
| `~/.config/hyprbole/current` | Generated Hyprbole runtime state |
| `~/.config/hyprbole/settings.toml` | User runtime choices |
| `~/.config/hyprbole/theme-sources.conf` | External theme source registry |
| `~/.config/hypr` | User-editable Hyprland Lua config |
| `~/.local/share/hyprbole/default` | Hyprbole framework defaults |
| `~/.local/share/hyprbole/themes` | Bundled theme packs |

Hyprbole should be understandable from files on disk. When in doubt, inspect the relevant file under these paths.

## Editing Rule Of Thumb

Edit files under `~/.config` for your own machine. Treat `~/.local/share/hyprbole/default` and `~/.local/share/hyprbole/bin` as Hyprbole-owned framework files unless you are developing Hyprbole itself.

If you are unsure whether a change belongs in user config or Hyprbole defaults, prefer user config first. You can always reset or refresh Hyprbole-owned runtime state with `hyprbole refresh-all`.
