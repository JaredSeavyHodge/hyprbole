# System Overview

Hyprbole is the desktop layer on top of Arch Linux. This page is for understanding what Hyprbole owns, where files live, and where local changes should go.

## What Hyprbole Owns

- Hyprland with Lua configuration.
- Waybar status bar.
- The Waybar health heart beside the date.
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

## Default Apps And MIME

Hyprbole has two related default-app layers:

| Layer | Purpose |
| --- | --- |
| `hb default-app <role>` | Shows the runtime app Hyprbole wrappers use for roles such as `browser`, `terminal`, `files`, `editor`, and `pdf` |
| `xdg-mime` | Controls which desktop file opens a MIME type from file managers, portals, browsers, and other desktop apps |

Examples:

```bash
hb default-app editor
hb default-app pdf command
xdg-mime query default text/plain
xdg-mime query default application/pdf
```

Install applies Hyprbole's current MIME defaults with `xdg-mime`: Mousepad for text, Papers for PDFs, Nautilus for folders, imv for images, mpv for video, and the Hyprbole Brave launcher for web links.

Set a MIME default directly when you know the desktop file and MIME type:

```bash
xdg-mime default org.xfce.mousepad.desktop text/plain
xdg-mime default org.gnome.Papers.desktop application/pdf
```

## Editing Rule Of Thumb

Edit files under `~/.config` for your own machine. Treat `~/.local/share/hyprbole/default` and `~/.local/share/hyprbole/bin` as Hyprbole-owned framework files unless you are developing Hyprbole itself.

If you are unsure whether a change belongs in user config or Hyprbole defaults, prefer user config first. You can reset or refresh Hyprbole-owned runtime state with `hb refresh-all`.
