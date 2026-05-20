# Walker

Walker is Hyprbole's launcher UI. It opens apps, web searches, calculations, files, clipboard history, 1Password entries, symbols, commands, and Hyprbole menus.

## Open Walker

Open the main launcher:

```text
SUPER + SPACE
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
| Apps | Browser, files, passwords |
| Keybinds | Browse current keyboard shortcuts |
| User Guide | Open this documentation |
| Theme | Themes, wallpapers, and fonts |
| Install | Package install and removal workflows |
| Controls | Audio, network, Bluetooth, toggles |
| Capture | Screenshots and recordings |
| System | Refresh, update, lock, logout, power |

## Menus And Commands

Hyprbole menus are powered by Elephant menu providers inside Walker.

Useful terminal equivalents:

```bash
hb apps
hb keybinds
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
