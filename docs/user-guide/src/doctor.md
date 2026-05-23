# Doctor

`hb doctor` is Hyprbole's readable desktop health report. It checks packages, services, runtime processes, config files, generated theme state, browser policy, SDDM theme files, hourly health timer state, health helper scripts, and Keyring/Secret Service behavior.

Use it after install, after a system update, or whenever something feels broken.

## Commands

```bash
hb doctor
hb doctor --verbose
hb doctor --fix
```

`hb doctor` prints one summary line per section.

`hb doctor --verbose` prints every individual check.

`hb doctor --fix` runs supported repairs, then reports what changed or what still needs attention.

## Status Labels

Doctor output uses four labels:

| Label | Meaning |
| --- | --- |
| `ok` | The check passed |
| `fix` | `--fix` repaired the issue |
| `warn` | Something is wrong or needs attention, but the system may still run |
| `miss` | A required package, command, file, service, or config is missing |

In normal mode, successful checks are folded into section summaries. In verbose mode, every `ok`, `warn`, `miss`, and `fix` line is shown.

## What It Checks

Doctor currently checks these sections:

| Section | What it checks |
| --- | --- |
| Checkout | `~/.local/share/hyprbole` is a Git checkout |
| Packages: core desktop | Hyprland, UWSM, Waybar, SDDM, PipeWire, portals, lock/idle tools, and related desktop packages |
| Competing packages | Known conflicting packages such as `dunst` and PulseAudio packages are absent |
| Packages: apps | Required app-support packages such as recording and system monitor tools |
| Default apps | The selected runtime apps for roles such as terminal, browser, file manager, editor, and PDF viewer resolve to available commands |
| Curated extras | Hyprbole-integrated apps such as VS Code, Neovim, 1Password, Obsidian, Apostrophe, Mousepad, and Papers when they are installed |
| Packages: launcher | Walker and Elephant provider packages |
| Packages: system tools | Shell tools, fonts, icons, Snapper/Limine tools, `eza`, `dua-cli`, `man-db`, and other support packages |
| Commands | Required executables and Hyprbole health helpers are available |
| Services: user | PipeWire, WirePlumber, GNOME Keyring socket, SwayOSD, Polkit agent, Elephant, Walker, and SwayNC user units |
| Runtime | Running Waybar, SwayNC, SwayOSD, Walker, Elephant, and wallpaper daemon processes during graphical sessions |
| Services: system | SDDM, Polkit, Hyprbole health timer, Limine Snapper sync, and Snapper cleanup timer state |
| SDDM Theme | Installed Hyprbole SDDM theme files and generated background assets |
| Configs | Expected config files, browser launchers, and gnome-libsecret flags |
| Secret Service | GNOME Keyring/libsecret Secret Service ownership, default keyring files, PAM hooks, session opening, alias, and lock state |
| Theme State | Generated current theme files under `~/.config/hyprbole/current` and GTK settings |
| Browser Policy | Generated Brave policy and system policy symlink |

## What `--fix` Can Repair

`hb doctor --fix` is intentionally limited to Hyprbole-owned state and normal runtime services.

Doctor does not require optional curated extras to be installed. When an extra is installed, Doctor checks the related command and integration files. Runtime default app roles are checked by the selected command, not by forcing the original Hyprbole preference.

It can repair or attempt to repair:

| Area | Repair behavior |
| --- | --- |
| Failed user units | Runs `systemctl --user reset-failed` |
| Inactive user services | Restarts or enables Hyprbole-managed user units when appropriate |
| Runtime processes | Restarts Waybar, wallpaper daemon, SwayNC, SwayOSD, Walker, or Elephant |
| SDDM theme | Redeploys the Hyprbole SDDM theme, reinstalls `/etc/sddm.conf.d/hyprbole.conf`, and regenerates SDDM wallpaper assets |
| Secret Service | Runs Hyprbole's Keyring setup, falls back to user-only repair when sudo is unavailable, and unlocks the default collection when possible |

Some repairs need sudo. If sudo is not available to the process, doctor reports the failed repair instead of hiding it.

## What It Does Not Repair

Doctor does not install missing packages directly. Use the installer, update flow, or package commands for package changes.

Doctor does not overwrite user-owned config copied from `config/` unless a specific repair path owns that generated/runtime state. For refreshing editable user config from repo defaults, use:

```bash
hb refresh-config <path>
hb refresh-all --include-user-configs
```

`refresh-all --include-user-configs` can overwrite local customizations after creating backups.

## Doctor Versus Verify

Use `hb doctor` for a human-readable health report and supported repair flow.

Use `hb verify` for a stricter install-oriented pass/fail check. `verify` exits with failure when required install state is missing or incorrect.

Use `hb health` for cached disk, filesystem, systemd, and update-freshness alerts that feed the Waybar health heart.

Good post-install sequence:

```bash
hb doctor --fix
hb doctor --verbose
hb verify
```

## Reading Secret Service Results

The Secret Service checks are about GNOME Keyring/libsecret integration. Hyprbole expects a passwordless `Default_keyring`, no encrypted `login.keyring`, SDDM auth/password GNOME Keyring PAM hooks removed, SDDM session hooks present, and the default collection unlocked.

If `hb doctor --fix` says the user defaults were repaired but the collection remains locked, log out and back in. Browser and editor secret storage usually need a fresh session after Keyring repair.

## Reading SDDM Theme Results

The SDDM Theme section checks Hyprbole-owned installed files under:

```text
/usr/share/sddm/themes/hyprbole
/etc/sddm.conf.d/hyprbole.conf
```

`Main.qml` and `metadata.desktop` should match repo defaults. `theme.conf`, `background.jpg`, and `background-blur.jpg` are generated or refreshed from current Hyprbole theme state.

If SDDM falls back to a plain screen, run:

```bash
hb doctor --fix
```
