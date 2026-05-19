# Hyprbole Archinstall Baseline

Use `hyprbole-base.json` when installing Arch for a machine where Hyprbole will own the desktop layer.

This config intentionally does not select an `archinstall` desktop profile. Hyprbole installs and owns Hyprland, UWSM, Waybar, SwayNC, PipeWire audio support, Ghostty, Brave Nightly, Nautilus, and related desktop services after first boot.

Recommended flow from the Arch ISO:

```bash
archinstall --config hyprbole-base.json
```

Important choices:

- `profile_config` is `null` so Archinstall does not install a desktop profile.
- `audio_config` is `null` so Archinstall does not choose the audio stack.
- `packages` is limited to bootstrap essentials needed after first boot.
- Disk layout, encryption, hostname, timezone, mirrors, and users remain interactive unless you extend the JSON for a specific machine.

After rebooting into the base system, clone Hyprbole and run `./install.sh` from the checkout as your regular user.
