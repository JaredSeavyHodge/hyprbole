# Hyprbole

Hyprbole is a curated post-install desktop layer for Arch Linux built around Hyprland, UWSM, Waybar, and a managed defaults model inspired by Omarchy.

`archinstall` owns base installation. `hyprbole` owns the desktop stack, defaults, scripts, themes, and ongoing system personality.

## Current Scope

- Guided `archinstall` wrapper plus post-install desktop bootstrap
- Hyprland Lua config as the primary compositor config format
- Waybar as the default status bar
- `hyprbole` as the primary human-facing command surface
- Walker as the default launcher and clipboard picker
- Ghostty as the default terminal
- `brave-origin-nightly-bin` as the default browser
- Nautilus as the default file manager
- Managed vendor defaults under `~/.local/share/hyprbole`
- User-facing editable config under `~/.config`
- GNOME Keyring/libsecret for Chromium/Electron secret storage
- External theme sources synced into user config on demand

## Repository Layout

- `bin/`: user-facing commands and helper tools
- `install/`: bootstrap stages and install logic
- `config/`: source for files that map to `~/.config`
- `default/`: vendor-owned framework files, helper assets, and templates
- `themes/`: shipped theme packs and theme inputs
- `docs/`: product, architecture, and implementation notes

## Design Principles

- Keep the user-editable surface in `~/.config`
- Keep reusable framework pieces in `~/.local/share/hyprbole`
- Prefer small, composable shell scripts over a large monolithic installer
- Prefer `hyprbole ...` as the primary human-facing CLI
- Keep leaf scripts for integration points such as keybinds, menus, services, and file manager actions
- Prefer official Arch packages first, then a small curated AUR set through `yay`
- Keep Hyprland config modular and readable
- Treat theming as a first-class system concern, not an afterthought

## Command Surface

`hyprbole` is the main user-facing interface in terminals.

Examples:

- `hyprbole update`
- `hyprbole theme set default-theme`
- `hyprbole pkg install`
- `hyprbole pkg aur-install`
- `hyprbole launch steam`
- `hyprbole launch obsidian`
- `hyprbole apps`
- `hyprbole keybinds`
- `hyprbole menu`
- `hyprbole lock`
- `hyprbole suspend`

Short common routes exist for frequent actions, while grouped routes remain available for clarity and scripting.

Examples of equivalent grouped routes:

- `hyprbole launch menu`
- `hyprbole system lock`
- `hyprbole power suspend`

Leaf scripts under `bin/hyprbole-*` still exist for desktop plumbing such as Hyprland binds, Walker/Elephant menus, systemd units, and Nautilus actions.

## Install And Verify

From the Arch ISO, Hyprbole can preseed safe guided `archinstall` defaults:

```bash
curl -fsSL https://raw.githubusercontent.com/jaredseavyhodge/hyprbole/master/install/archinstall/hyprbole-archinstall -o /tmp/hyprbole-archinstall
chmod +x /tmp/hyprbole-archinstall
/tmp/hyprbole-archinstall
```

After rebooting into the base system, clone Hyprbole and run the desktop installer.

Run the installer from a normal user account, not with `sudo`:

```bash
./install.sh
```

After install or after changing core defaults, use:

```bash
hyprbole doctor
hyprbole verify
```

Use `hyprbole doctor --fix` for supported repairs. Secret Service repair is intentionally behind `doctor --fix`; there is no public `hyprbole secret-service` workflow.

`install.sh` is designed to be safe to run more than once on the same machine. Re-running it is a valid repair path for Hyprbole-owned defaults.

Repeat install runs should not overwrite existing user-owned config copied from `config/`. They may intentionally reapply Hyprbole-owned state such as generated theme/runtime files, browser launchers, service enables, browser policy links, SDDM theme files, Limine/Snapper defaults, and Secret Service setup.

PAM cleanup is idempotent: removing already-removed GNOME Keyring `auth`/`password` hooks is a no-op. Secret Service setup also avoids repeated `login.keyring` backups because the encrypted keyring is moved only when it exists.

## Browser Defaults

Hyprbole owns Brave Origin Nightly startup through `bin/hyprbole-launch-brave-origin-nightly` and writes browser flags to `~/.config/brave-origin-nightly-flags.conf` which the system launcher reads.

The wrapper always passes:

- `--password-store=gnome-libsecret`
- `--ozone-platform-hint=auto`

Keep `~/.config/brave-origin-nightly-flags.conf` single-flag safe for the package wrapper. Put Hyprbole-specific extra Brave flags in:

```text
~/.config/hyprbole/brave-origin-nightly-flags.conf
```

Chromium fractional-scaling flags are not enabled globally because they are monitor and Chromium-build dependent. Opt in through the Hyprbole extra flags file when testing an odd-scale monitor setup.

## Themes

The shipped fallback theme is `default-theme`.

Theme locations:

- Bundled themes: `~/.local/share/hyprbole/themes/<theme>`
- User-authored themes: `~/.config/hyprbole/themes/<theme>`
- Synced source themes: `~/.config/hyprbole/themes/.sources/<source>/themes/<theme>`

Theme source registry:

```text
~/.config/hyprbole/theme-sources.conf
```

External repositories are synced explicitly:

```bash
hyprbole theme source sync
```

Omarchy themes are treated as an external source example, not as vendored repository content.

## Secret Service

Install configures GNOME Keyring/libsecret for apps such as Brave and VS Code.

Hyprbole's setup creates a passwordless `Default_keyring`, backs up an encrypted `login.keyring` if present, removes SDDM `auth`/`password` GNOME Keyring PAM hooks, keeps session autostart hooks, and restarts the user keyring daemon when possible.

If browser or editor secret storage regresses, run:

```bash
hyprbole doctor --fix
hyprbole verify
```

## Planned Ownership Matrix

- Session/compositor: `hyprland` + `uwsm`
- Status bar: `waybar`
- Notifications: `swaync`
- Lock screen: `hyprlock`
- Idle: `hypridle`
- Wallpaper: `awww`
- Launcher: `walker`
- Clipboard history: Walker + Elephant clipboard provider
- Screenshot annotation: `satty`
- Password manager extra: `1password-beta`

## Status

Current desktop defaults include:

- `SUPER + SPACE`: Walker launcher
- `SUPER + V`: Walker clipboard history
- `Print`: region screenshot into `satty`
- `Shift + Print`: fullscreen screenshot into `satty`
- `SUPER + SHIFT + /`: `1Password` extra
- `SUPER + S`: toggle scratchpad workspace

Session-critical background components are increasingly service-managed through user systemd units, including:

- Walker
- Elephant
- SwayOSD
- polkit agent
