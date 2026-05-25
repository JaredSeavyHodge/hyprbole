# App Launcher

Walker is Hyprbole's launcher UI. It opens apps, web searches, calculations, files, clipboard history, symbols, commands, and Hyprbole menus. 1Password entries are available when the 1Password extra is installed.

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
| `p:` | 1Password extra | `p: github` |

The launcher placeholder shows the same summary:

```text
Search   = calc  > run  / files  . symbols  @ web  : clipboard  p: passwords when installed
```

## Hyprbole Menu

Open the Hyprbole menu with `SUPER + ALT + SPACE`.

The top-level menu includes:

| Entry | Purpose |
| --- | --- |
| Applications | Browser, files, passwords, app launcher |
| Appearance | Themes, wallpapers, fonts |
| Capture | Screenshots and recordings |
| Gaming | Steam launch, Steam gaming mode, and curated install link |
| Install | Curated extras, Pacman, and AUR package install workflows |
| Remove | Package inspection and removal workflows |
| System | Settings, utilities, power, update, session actions |
| Help | User guide and keybinds |

Menu entries with a right-side marker open submenus. Press `Enter` to open or activate the selected item. Press `Escape` to return to the parent menu or close Walker at the top level.

## Menus And Commands

Hyprbole menus are powered by Elephant menu providers inside Walker.

Useful terminal equivalents:

```bash
hb apps
hb menu hyprbole "Hyprbole menu"
hb launch menu hyprbole "Hyprbole menu"
hb launch browser
hb launch files
hb launch terminal
hb launch launcher
hb tools
hb launch tools
hb utilities
hb launch utilities
hb keybinds
hb launch keybinds
hb launch audio
hb launch network
hb launch bluetooth
hb disk-usage
hb launch btop
hb launch update
hb launch themes
hb launch wallpaper-menu
hb launch my-wallpaper-menu
hb launch steam
hb launch steam-gaming-mode
hb launch obsidian
hb launch passwords
hb launch pkg-installed
hb launch pkg-install
hb launch pkg-aur-install
hb launch pkg-remove
hb launch extra-install steam
```

## Files, Clipboard, And Passwords

Use `/` to browse files through Walker's file provider.

Use `:` or `SUPER + V` for clipboard history.

Use `p:` or the Passwords entry from the Applications menu for password lookup after installing the 1Password extra from `Install`, `Extras`.

## Utilities Menu

Open `System`, `Utilities` from the Hyprbole menu to launch utility and maintenance workflows. `Disk Usage` opens `dua i` in a floating terminal. `Mount Share` opens the NAS share workflow in a floating terminal. `Refresh Desktop` reapplies Hyprbole runtime state through a retained refresh helper. `Refresh Shipped Configs` also refreshes shipped user config files and can overwrite local customizations after creating backups.

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
