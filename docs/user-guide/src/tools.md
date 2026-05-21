# Utilities

Hyprbole's Utilities menu groups useful local system utilities and maintenance actions that are not primary apps. The first utility is Disk Usage, powered by `dua-cli`.

## Open Utilities

Open the Hyprbole menu:

```text
SUPER + ALT + SPACE
```

Then choose `System`, `Utilities`.

From a terminal, use:

```bash
hb tools
hb utilities
```

## Disk Usage

`Disk Usage` opens `dua i` in a floating terminal. `dua` is a fast terminal disk-usage browser.

Open it from the Utilities menu, search for `Disk Usage` in the app launcher, or run:

```bash
hb disk-usage
```

From a terminal, `hb disk-usage` runs `dua i` in the current terminal. Launcher and menu entries use `hb launch disk-usage` so a floating terminal opens for the tool.

## System Health

`System Health` opens the cached hourly health report. Waybar shows a health bell beside the date; it inherits the theme when there are no alerts, turns orange for warnings, and turns red for high alerts.

Left-click the Waybar bell to open the report. Right-click it to refresh the health check and update the bell state.

Run it directly with:

```bash
hb health
hb health refresh
hb health updates
```

`hb health updates` is the quickest way to check whether your last full system update is stale or the Hyprbole repo has upstream updates available.

The Utilities menu also includes `System Monitor` for `btop`, `System Info` for Hyprland system information, and refresh actions for Hyprbole runtime state and shipped configs.

`Refresh Desktop` reapplies theme and runtime state. `Refresh Shipped Configs` runs the same refresh with user config files included, which can overwrite local customizations after backing them up.

To inspect a specific path, pass it to Hyprbole:

```bash
hb disk-usage ~/Downloads
hb disk-usage ~/.local/share/hyprbole
```

You can also run `dua` directly:

```bash
dua i
dua i ~/Downloads
```

## Launcher Entry

Hyprbole generates this desktop entry so Walker's app launcher can find Disk Usage:

```text
~/.local/share/applications/hyprbole-disk-usage.desktop
```

Refresh generated tool launchers with:

```bash
hyprbole-refresh-tool-launchers
```
