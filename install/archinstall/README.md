# Hyprbole Archinstall Reference

`hyprbole-archinstall` is a small Arch ISO helper that launches guided `archinstall` with Hyprbole's base-system defaults preselected.

`hyprbole-base.json` is the repository reference for the same minimal Arch shape Hyprbole expects. It is useful for review and testing, but the wrapper generates its own temporary config so it can be copied or fetched as a standalone script on the Arch ISO.

This config intentionally does not select an `archinstall` desktop profile. Hyprbole installs and owns Hyprland, UWSM, Waybar, SwayNC, Ghostty, Brave Nightly, Nautilus, and related desktop services after first boot.

Wrapper defaults:

- `profile_config` is `null` so Archinstall does not install a desktop profile.
- `app_config.audio_config` selects PipeWire.
- `network_config` selects NetworkManager.
- `bootloader_config` selects Limine.
- `packages` is limited to bootstrap essentials needed after first boot.
- Sudo access belongs to the `archinstall` user/authentication choices, not the package list.
- Disk layout, encryption, hostname, timezone, mirrors, and users remain editable in the guided TUI unless you extend the JSON for a specific machine.

Example from the Arch ISO after networking is up:

```bash
curl -fsSL https://raw.githubusercontent.com/jaredseavyhodge/hyprbole/master/install/archinstall/hyprbole-archinstall -o /tmp/hyprbole-archinstall
chmod +x /tmp/hyprbole-archinstall
/tmp/hyprbole-archinstall
```

Write the generated config somewhere specific for inspection with:

```bash
/tmp/hyprbole-archinstall --config-path /tmp/hyprbole-base.json --dry-run
```

After rebooting into the base system, clone Hyprbole and run `./install.sh` from the checkout as your regular user.
