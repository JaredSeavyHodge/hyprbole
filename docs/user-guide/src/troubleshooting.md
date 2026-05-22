# Troubleshooting

Start with the built-in checks. `doctor` is the human-readable report; `verify` is stricter and useful after installs, updates, and repairs.

```bash
hb doctor
hb verify
```

Use repair mode for supported fixes:

```bash
hb doctor --fix
```

Use verbose output when you need the full checklist:

```bash
hb doctor --verbose
```

Use the health report when the Waybar heart shows an alert:

```bash
hb health
hb health refresh
```

## Fresh Install Looks Partial

Run:

```bash
hb refresh-all
hb doctor --verbose
```

If generated GTK settings are missing, refresh GNOME/GTK appearance:

```bash
hyprbole-refresh-gnome
```

Expected files:

```text
~/.config/gtk-3.0/settings.ini
~/.config/gtk-4.0/settings.ini
```

If many config files are missing, rerun `./install.sh` from the Hyprbole checkout.

## Login Does Not Start Hyprbole

Hyprbole expects SDDM to start Hyprland through UWSM so user services and the graphical session target behave correctly.

Check that `uwsm` is installed:

```bash
pacman -Q uwsm
```

## Browser Secret Warning

Run:

```bash
hb doctor --fix
hb verify
```

Then fully close and relaunch Brave or VS Code.

## Browser Opens With Wrong Flags

Regenerate the Hyprbole browser launcher:

```bash
hyprbole-refresh-browser-launchers
```

Check that the default browser is `hyprbole-brave-origin-nightly.desktop`.

Then close all Brave windows and relaunch through Hyprbole.

## Theme Looks Wrong

Reapply the current theme:

```bash
hb theme set "$(hb theme current)"
```

or switch back to the fallback:

```bash
hb theme set default-theme
```

## I Messed Up A Config

If one editable file under `~/.config` is broken, refresh that single file from Hyprbole's shipped default instead of resetting everything.

Use the path relative to the repo `config/` directory. For example, `~/.config/waybar/config.jsonc` becomes `waybar/config.jsonc`:

```bash
hb refresh-config waybar/config.jsonc
```

Common examples:

```bash
hb refresh-config hypr/bindings.lua
hb refresh-config hypr/monitors.lua
hb refresh-config hypr/input.lua
hb refresh-config waybar/style.css
hb refresh-config ghostty/config
hb refresh-config walker/config.toml
```

When the destination already exists, Hyprbole saves a timestamped backup next to it before replacing it, such as:

```text
~/.config/waybar/config.jsonc.bak.20260522143000
```

To find refreshable paths, inspect the shipped config tree:

```bash
fd --type f . ~/.local/share/hyprbole/config
```

Use `hb refresh-all --include-user-configs` only when you intentionally want to refresh many editable config files from Hyprbole defaults.

## Waybar Or Wallpaper Missing

Refresh Hyprbole-owned runtime state:

```bash
hb refresh-all
```

If a single component needs a restart, use:

```bash
hb restart waybar
hb restart wallpaper
hb restart swaync
hb restart swayosd
```

If user config files are intentionally being reset from Hyprbole defaults, use:

```bash
hb refresh-all --include-user-configs
```

## Waybar Health Bell Shows An Alert

Open the cached report:

```bash
hb health
```

Run a fresh check:

```bash
hb health refresh
```

If you only care about update freshness, run:

```bash
hb health updates
```

The heart is orange for warnings and red for critical alerts. A clean report shows `No Alerts` in the tooltip.

## Package Operation Refused For Disk Space

Hyprbole refuses package installs and updates when a local filesystem is at or above `98%` usage.

Open Disk Usage from the Utilities menu or run:

```bash
hb disk-usage
```

Free space, then rerun the package operation.

## Installer Rerun

`install.sh` is safe to rerun and can repair Hyprbole-owned state. It should not overwrite existing user-owned config copied from `config/`, but it may intentionally reapply generated state, browser launchers, service enables, SDDM files, browser policy links, Limine/Snapper defaults, and Keyring setup.

Installer logs are under:

```text
~/.local/state/hyprbole/install-logs
```

Start with `latest.log` and `latest.diagnostics.txt`.
