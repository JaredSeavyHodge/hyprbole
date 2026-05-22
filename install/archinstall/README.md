# Hyprbole Archinstall Reference

`hyprbole-archinstall` is a small Arch ISO helper that runs `archinstall` with Hyprbole's base-system defaults preselected.

`hyprbole-base.json` is the repository reference for the same minimal Arch shape Hyprbole expects. The wrapper copies it to a temporary config, patches machine-specific values such as the target disk, hostname, timezone, language, locale, and keyboard layout, and passes it to `archinstall`.

This config intentionally does not select an `archinstall` desktop profile. Hyprbole installs and owns Hyprland, UWSM, Waybar, SwayNC, Ghostty, Brave Nightly, Nautilus, and related desktop services after first boot.

Wrapper defaults:

- `profile_config` is `null` so Archinstall does not install a desktop profile.
- `app_config.audio_config` selects PipeWire.
- `network_config` selects NetworkManager.
- `bootloader_config` selects Limine.
- `packages` is limited to bootstrap essentials needed after first boot.
- The default disk layout is Btrfs with Hyprbole-friendly subvolumes.
- The wrapper uses lightweight filterable pickers for target disk, timezone, Archinstall language, system locale, and keyboard layout before running `archinstall --silent`.
- Hostname and sudo username are typed directly because they are machine-specific names.
- Sudo user credentials are written to a temporary `--creds` file and removed when the wrapper exits.
- Silent install supports one target disk. Use `--interactive` for encryption, multi-disk Btrfs, or custom storage layouts.

Example from the Arch ISO after networking is up:

```bash
curl -fsSL https://raw.githubusercontent.com/jaredseavyhodge/hyprbole/master/install/archinstall/hyprbole-archinstall -o /tmp/hyprbole-archinstall
chmod +x /tmp/hyprbole-archinstall
/tmp/hyprbole-archinstall
```

This does not launch the full Archinstall TUI. It shows numbered lists where you can type `/text` to filter, asks for hostname, sudo username, and sudo user password, then runs `archinstall --silent`.

If you want to inspect or edit the normal Archinstall TUI instead, run:

```bash
/tmp/hyprbole-archinstall --interactive
```

Write the generated config somewhere specific for inspection with:

```bash
/tmp/hyprbole-archinstall --config-path /tmp/hyprbole-base.json --generate-only
```

`--generate-only` can also run from an installed system because it writes the JSON and exits without launching `archinstall`.

After rebooting into the base system, clone Hyprbole and run `./install.sh` from the checkout as your regular user.
