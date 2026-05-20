# App Launcher

Walker is Hyprbole's launcher UI. It opens apps, web searches, calculations, files, clipboard history, 1Password entries, symbols, commands, and Hyprbole menus.

## Open App Launcher

Open the main launcher:

```text
SUPER + SPACE
SUPER + SHIFT + SPACE
```

Open the Hyprbole menu:

```text
SUPER + ALT + SPACE
```

Open clipboard history directly:

```text
SUPER + V
```

You can also launch these from a terminal:

```bash
hb launch launcher
hb menu
hb clipboard
```

## Main Launcher

The main launcher uses Walker's default providers for applications and web search.

Start typing an app name, then press Enter to launch the selected result. Press `Escape` to close Walker.

Hyprbole config enables keyboard focus and selection wrapping, so arrowing past the end wraps through results.

## Prefixes

Hyprbole configures provider prefixes in `~/.config/walker/config.toml`.

| Prefix | Provider | Example |
| --- | --- | --- |
| `=` | Calculator | `= 12 * 8` |
| `>` | Run command | `> ghostty` |
| `/` | Files | `/ Downloads` |
| `.` | Symbols | `. lambda` |
| `@` | Web search | `@ arch wiki snapper` |
| `:` | Clipboard history | `: password` |
| `p:` | 1Password | `p: github` |

The launcher placeholder shows the same summary:

```text
Search   = calc  > run  / files  . symbols  @ web  : clipboard  p: passwords
```

## Hyprbole Menu

Open the Hyprbole menu with `SUPER + ALT + SPACE`.

The top-level menu includes:

| Entry | Purpose |
| --- | --- |
| Applications | Browser, files, passwords, app launcher |
| Settings | Audio, network, Bluetooth, desktop controls |
| Appearance | Themes, wallpapers, fonts |
| Tools | Disk usage, monitor, system info |
| Capture | Screenshots and recordings |
| Install | Pacman and AUR package install workflows |
| Remove | Package inspection and removal workflows |
| Maintenance | Update and refresh Hyprbole |
| Help | User guide and keybinds |
| Power | Suspend, reboot, power off, profiles |
| Lock | Lock the current session |
| Logout | Log out to the display manager |

Menu entries with a right-side marker open submenus. Press `Right` or `Enter` to open a submenu. Press `Left` to return to the parent menu without closing Walker.

## Menus And Commands

Hyprbole menus are powered by Elephant menu providers inside Walker.

Useful terminal equivalents:

```bash
hb apps
hb tools
hb keybinds
hb disk-usage
hb launch themes
hb launch wallpaper-menu
hb launch my-wallpaper-menu
hb launch pkg-install
hb launch pkg-remove
```

## Files, Clipboard, And Passwords

Use `/` to browse files through Walker's file provider.

Use `:` or `SUPER + V` for clipboard history.

Use `p:` or the 1Password entry from the Apps menu for password lookup. Password support depends on the 1Password and Elephant 1Password packages installed by Hyprbole.

## Tools Menu

Open `Tools` from the Hyprbole menu to launch utility workflows. `Disk Usage` opens `dua i` in a floating terminal.

## Configuration

Walker config lives here:

```text
~/.config/walker/config.toml
```

Hyprbole's Walker theme lives here:

```text
~/.config/walker/themes/hyprbole
```

Repo defaults are copied from:

```text
~/.local/share/hyprbole/config/walker
```

If you intentionally want to refresh Walker config from repo defaults, use:

```bash
hb refresh-config walker/config.toml
hb refresh-config walker/themes/hyprbole/style.css
hb refresh-config walker/themes/hyprbole/layout.xml
```

Refreshing these files can overwrite local customizations.

## Troubleshooting

If Walker or Elephant is not responding, run:

```bash
hb doctor
```

Restart Walker and Elephant runtime services with:

```bash
systemctl --user restart walker.service elephant.service
```

`hb doctor --fix` can restart supported runtime components when checks fail.
