# Commands

`hyprbole` is the main terminal command. `hb` is available as a short alias when shell integration is loaded.

## Common Commands

```bash
hyprbole doctor
hyprbole doctor --fix
hyprbole verify
hyprbole docs
hyprbole update
hyprbole apps
hyprbole keybinds
hyprbole menu
hyprbole lock
hyprbole suspend
```

## Themes

```bash
hyprbole theme list
hyprbole theme current
hyprbole theme set default-theme
hyprbole theme font current
hyprbole theme font set all "Inter"
hyprbole theme source edit
hyprbole theme source sync
```

`hyprbole themes` opens the theme picker in the launcher UI.

## Wallpaper

```bash
hyprbole theme wallpaper list
hyprbole theme wallpaper current
hyprbole theme wallpaper set <file>
hyprbole theme wallpaper cycle
hyprbole wallpaper set <file>
hyprbole wallpaper cycle
hyprbole wallpaper timer start theme 30min
hyprbole wallpaper timer start custom 30min
hyprbole wallpaper timer status
hyprbole wallpaper timer interval 15min
hyprbole wallpaper timer stop
```

Theme wallpapers come from the active theme. Custom wallpapers come from `~/Pictures/Wallpaper`.

## Refresh

```bash
hyprbole refresh-all
hyprbole refresh-all --include-user-configs
hyprbole refresh-config <path>
hyprbole refresh-shell
```

`refresh-all` normally refreshes Hyprbole-owned runtime state. Use `--include-user-configs` only when you intentionally want Hyprbole to refresh editable config files from the repo defaults.

`refresh-config <path>` takes a path relative to the repo `config/` directory, such as `waybar/config.jsonc` or `hypr/bindings.lua`.

## Launch And Capture

```bash
hyprbole launch apps
hyprbole launch clipboard
hyprbole launch themes
hyprbole launch wallpaper-menu
hyprbole capture region
hyprbole capture screen
hyprbole capture record-region
hyprbole capture record-screen
```

The short top-level commands `hyprbole apps`, `hyprbole clipboard`, and `hyprbole keybinds` are aliases for common launcher actions.

## Defaults

```bash
hyprbole default-app terminal
hyprbole default-app terminal command
hyprbole default-app browser label
```

Runtime default app choices are read from `~/.config/hyprbole/settings.toml`. The default values are Ghostty, Brave Origin Nightly, and Nautilus.

## Repairs

Prefer `hyprbole doctor --fix` for supported repair flows. One-off setup commands are intentionally kept out of the public command surface unless they are normal user workflows.

Use `hyprbole verify` when you want a strict pass/fail check. Use `hyprbole doctor --verbose` when you want a readable report with each section shown.
