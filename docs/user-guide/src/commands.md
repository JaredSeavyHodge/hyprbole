# Commands

`hyprbole` is the main terminal command. `hb` is available as a short alias when shell integration is loaded.

## Common Commands

```bash
hyprbole doctor
hyprbole doctor --fix
hyprbole verify
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
hyprbole theme source edit
hyprbole theme source sync
```

## Wallpaper

```bash
hyprbole theme wallpaper list
hyprbole theme wallpaper set <file>
hyprbole wallpaper set <file>
hyprbole wallpaper timer start theme 30min
hyprbole wallpaper timer stop
```

## Refresh

```bash
hyprbole refresh-all
hyprbole refresh-all --include-user-configs
hyprbole refresh-config <path>
hyprbole refresh-shell
```

`refresh-all` normally refreshes Hyprbole-owned runtime state. Use `--include-user-configs` only when you intentionally want Hyprbole to refresh editable config files from the repo defaults.

## Repairs

Prefer `hyprbole doctor --fix` for supported repair flows. One-off setup commands are intentionally kept out of the public command surface unless they are normal user workflows.
