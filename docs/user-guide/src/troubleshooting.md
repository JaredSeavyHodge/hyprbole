# Troubleshooting

Start with the built-in checks:

```bash
hyprbole doctor
hyprbole verify
```

Use repair mode for supported fixes:

```bash
hyprbole doctor --fix
```

## Browser Secret Warning

Run:

```bash
hyprbole doctor --fix
hyprbole verify
```

Then fully close and relaunch Brave or VS Code.

## Browser Opens With Wrong Flags

Regenerate the Hyprbole browser launcher:

```bash
hyprbole-refresh-browser-launchers
```

Check that the default browser is `hyprbole-brave-origin-nightly.desktop`.

## Theme Looks Wrong

Reapply the current theme:

```bash
hyprbole theme set "$(hyprbole theme current)"
```

or switch back to the fallback:

```bash
hyprbole theme set default-theme
```

## Waybar Or Wallpaper Missing

Restart runtime components:

```bash
hyprbole-refresh-all
```

If user config files are intentionally being reset from Hyprbole defaults, use:

```bash
hyprbole refresh-all --include-user-configs
```

## Installer Rerun

`install.sh` is safe to rerun and can repair Hyprbole-owned state. It should not overwrite existing user-owned config copied from `config/`, but it may intentionally reapply generated state, browser launchers, service enables, SDDM files, browser policy links, Limine/Snapper defaults, and Secret Service setup.

## Disk Space

Package installs can fail when `/` is full. Check space with:

```bash
df -h / /home
```

Common cleanup targets are package cache and user cache. Avoid deleting project files or config unless you know what owns them.
