# Commands

`hyprbole` is the main terminal command. `hb` is the short alias and is what this guide uses for day-to-day examples. If `hb` is not loaded in the current shell yet, use `hyprbole` instead.

## Common Commands

```bash
hb doctor
hb doctor --fix
hb verify
hb docs
hb update
hb launch update
hb apps
hb tools
hb utilities
hb disk-usage
hb health
hb health updates
hb keybinds
hb menu
hb restart waybar
hb lock
hb logout
hb suspend
hb reboot
hb poweroff
```

## Themes

```bash
hb theme list
hb theme current
hb theme set default-theme
hb theme font current
hb theme font set all "Inter"
hb theme source edit
hb theme source sync
```

`hb themes` opens the theme picker in the launcher UI.

## Packages

```bash
hb extra list
hb extra install vscode
hb extra install steam
hb pkg installed
hb pkg install
hb pkg aur-install
hb pkg remove
```

`hb extra install <name>` installs curated Hyprbole integrations such as VS Code, 1Password, and Obsidian. The Hyprbole menu exposes these under `Install`, `Extras`. These are optional preference apps, not required desktop components.

## Wallpaper

```bash
hb theme wallpaper list
hb theme wallpaper current
hb theme wallpaper set <file>
hb theme wallpaper cycle
hb wallpaper set <file>
hb wallpaper cycle
hb wallpaper timer start theme 30min
hb wallpaper timer start custom 30min
hb wallpaper timer status
hb wallpaper timer interval 15min
hb wallpaper timer stop
```

Theme wallpapers come from the active theme. Custom wallpapers come from `~/Pictures/Wallpaper`.

## Refresh

```bash
hb refresh-all
hb refresh-all --include-user-configs
hb refresh-config <path>
hb refresh-shell
```

`refresh-all` normally refreshes Hyprbole-owned runtime state without replacing editable user config. Use `--include-user-configs` only when you intentionally want Hyprbole to refresh editable files under `~/.config` from repo defaults; this can overwrite local customizations after backups are created.

`refresh-config <path>` refreshes one editable file and takes a path relative to the repo `config/` directory, such as `waybar/config.jsonc` or `hypr/bindings.lua`.

## Launch And Capture

```bash
hb launch apps
hb launch launcher
hb launch tools
hb launch utilities
hb launch keybinds
hb launch clipboard
hb launch themes
hb launch wallpaper-menu
hb launch obsidian
hb launch passwords
hb launch steam
hb launch steam-gaming-mode
hb launch pkg-installed
hb launch pkg-install
hb launch pkg-aur-install
hb launch pkg-remove
hb launch extra-install steam
hb capture region
hb capture screen
hb capture record-region
hb capture record-screen
```

The short top-level commands `hb apps`, `hb clipboard`, and `hb keybinds` are aliases for common launcher actions.

## System And Power

```bash
hb lock
hb logout
hb suspend
hb reboot
hb poweroff
hb system lock
hb system logout
hb power suspend
hb power reboot
hb power off
```

Short top-level routes cover common session and power actions. Grouped `system` and `power` routes remain available for scripts and clarity.

## Restart UI Components

```bash
hb restart all
hb restart waybar
hb restart wallpaper
hb restart swaync
hb restart swayosd
hb restart menus
```

`hb restart menus` restarts both Walker and Elephant. Use `hb restart walker` or `hb restart elephant` when you only need one side of the menu stack.

## Tools

```bash
hb tools
hb utilities
hb disk-usage
hb disk-usage ~/Downloads
hb launch disk-usage
hb launch mount-share
hb mount
hb mirrors
hb health
hb health refresh
hb health updates
hb launch health
```

`hb tools` and `hb utilities` open the Utilities menu. `hb disk-usage` runs `dua i` in the current terminal. `hb mount` runs the Mount Share workflow in the current terminal. `hb launch disk-usage`, `hb launch health`, `hb launch mount-share`, `hb launch pkg-*`, and `hb launch extra-install <name>` open `com.hyprbole.*` floating terminals for menu and launcher workflows. `hb mirrors` opens the mirror refresh helper.

Health commands:

| Command | Purpose |
| --- | --- |
| `hb health` | Print the cached hourly health report |
| `hb health refresh` | Run a fresh health check through systemd, then print the updated report |
| `hb health updates` | Check package update age and Hyprbole repo freshness directly |
| `hb launch health` | Open the cached health report in a floating terminal |

## Defaults

```bash
hb default-app terminal
hb default-app terminal command
hb default-app browser label
hb default-app editor
hb default-app pdf
```

Runtime default app choices are read from `~/.config/hyprbole/settings.toml`. The default values are Ghostty, Brave Origin Nightly, Nautilus, Mousepad, and Papers.

Use `hb default-app <role>` to see the app name Hyprbole uses for one runtime role. Add `command` when you need the executable that wrappers call, or `label` when you want the human-readable name:

```bash
hb default-app browser command
hb default-app pdf label
```

These roles are Hyprbole runtime choices, not the whole XDG MIME database. For example, `hb default-app pdf` reports `papers`, while `xdg-mime query default application/pdf` reports the desktop entry that file managers and portals use, such as `org.gnome.Papers.desktop`.

## Snapshots

```bash
hb snapshot create
hb snapshot restore
hb refresh-limine
```

Snapshots use Snapper when available. `refresh-limine` refreshes Limine boot entries and synced snapshot entries when the related commands are installed.

## Repairs

Prefer `hb doctor --fix` for supported repair flows. One-off setup commands are intentionally kept out of the public command surface unless they are normal user workflows.

Use `hb verify` when you want a strict pass/fail check. Use `hb doctor --verbose` when you want a readable report with each section shown. See the [Doctor](doctor.md) guide for the exact checks and repair behavior.
