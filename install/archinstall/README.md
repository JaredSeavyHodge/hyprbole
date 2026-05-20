# Hyprbole Archinstall Reference

`hyprbole-base.json` is a repository reference for the minimal Arch shape Hyprbole expects. User-facing install docs should describe the manual `archinstall` choices because this file is not available on a fresh Arch ISO unless someone has already copied the repo there.

This config intentionally does not select an `archinstall` desktop profile. Hyprbole installs and owns Hyprland, UWSM, Waybar, SwayNC, Ghostty, Brave Nightly, Nautilus, and related desktop services after first boot.

Important choices:

- `profile_config` is `null` so Archinstall does not install a desktop profile.
- User-facing docs recommend choosing PipeWire in `archinstall`; this reference leaves `audio_config` null because Hyprbole also installs PipeWire packages.
- `packages` is limited to bootstrap essentials needed after first boot.
- Sudo access belongs to the `archinstall` user/authentication choices, not the package list.
- Disk layout, encryption, hostname, timezone, mirrors, and users remain interactive unless you extend the JSON for a specific machine.

After rebooting into the base system, clone Hyprbole and run `./install.sh` from the checkout as your regular user.
