# Commands

`hyprbole` is the main terminal command. `hb` is the short alias and is what this guide uses for day-to-day examples. If `hb` is not loaded in the current shell yet, use `hyprbole` instead.

## Common Commands

```bash
hb doctor
hb doctor --fix
hb verify
hb docs
hb update
hb apps
hb tools
hb disk-usage
hb keybinds
hb menu
hb lock
hb suspend
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

`refresh-all` normally refreshes Hyprbole-owned runtime state. Use `--include-user-configs` only when you intentionally want Hyprbole to refresh editable user config files under `~/.config` from the repo defaults; this can overwrite local customizations.

`refresh-config <path>` takes a path relative to the repo `config/` directory, such as `waybar/config.jsonc` or `hypr/bindings.lua`.

## Launch And Capture

```bash
hb launch apps
hb launch clipboard
hb launch themes
hb launch wallpaper-menu
hb capture region
hb capture screen
hb capture record-region
hb capture record-screen
```

The short top-level commands `hb apps`, `hb clipboard`, and `hb keybinds` are aliases for common launcher actions.

## Tools

```bash
hb tools
hb disk-usage
hb disk-usage ~/Downloads
hb launch disk-usage
```

`hb tools` opens the Tools menu. `hb disk-usage` runs `dua i` in the current terminal. `hb launch disk-usage` opens it in the default terminal for menu and launcher workflows.

## Defaults

```bash
hb default-app terminal
hb default-app terminal command
hb default-app browser label
```

Runtime default app choices are read from `~/.config/hyprbole/settings.toml`. The default values are Ghostty, Brave Origin Nightly, and Nautilus.

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
