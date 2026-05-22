# System Overview

Hyprbole is the desktop layer on top of Arch Linux. This page is for understanding what Hyprbole owns, where files live, and where local changes should go.

## What Hyprbole Owns

- Hyprland with Lua configuration.
- Waybar status bar.
- The Waybar health bell beside the date.
- Walker launcher and clipboard picker.
- SwayNC notifications.
- Ghostty terminal.
- Brave Origin Nightly browser through a Hyprbole wrapper.
- Nautilus file manager.
- Theme and wallpaper state under `~/.config/hyprbole/current`.
- Keyring setup for GNOME Keyring/libsecret Secret Service integration.

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

If you are unsure whether a change belongs in user config or Hyprbole defaults, prefer user config first. You can reset or refresh Hyprbole-owned runtime state with `hb refresh-all`.
