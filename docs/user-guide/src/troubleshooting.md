# Troubleshooting

Start with the built-in checks:

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

## Waybar Or Wallpaper Missing

Restart runtime components:

```bash
hyprbole-refresh-all
```

If a single component needs a restart, use:

```bash
hyprbole-restart-waybar
hyprbole-restart-wallpaper
hyprbole-restart-swaync
hyprbole-restart-swayosd
```

If user config files are intentionally being reset from Hyprbole defaults, use:

```bash
hb refresh-all --include-user-configs
```

## Installer Rerun

`install.sh` is safe to rerun and can repair Hyprbole-owned state. It should not overwrite existing user-owned config copied from `config/`, but it may intentionally reapply generated state, browser launchers, service enables, SDDM files, browser policy links, Limine/Snapper defaults, and Keyring setup.

Installer logs are under:

```text
~/.local/state/hyprbole/install-logs
```

Start with `latest.log` and `latest.diagnostics.txt`.
