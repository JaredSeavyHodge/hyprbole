# Architecture

## Layer Split

Hyprbole is intentionally split into two layers:

1. Base install layer
   - owned by `archinstall`
   - partitions, bootloader, user creation, locale, networking, filesystem

2. Desktop layer
   - owned by `hyprbole`
   - packages, AUR tooling, defaults, theming, session behavior, shell commands, user-facing CLI

## Checkout And Config Ownership

Hyprbole follows a vendor-defaults plus user-config model.

- Vendor-owned files live in a full git checkout at `~/.local/share/hyprbole`
- User-owned editable files live under `~/.config`
- Generated runtime state lives under `~/.config/hyprbole/current`

Repository mapping:

- `~/.local/share/hyprbole` is the whole repository checkout, not a partial copy
- `config/` is copied to `~/.config/...` only when the destination is missing
- `default/` remains vendor-owned inside the checkout
- `themes/` contains shipped theme packs inside the checkout
- user-authored themes live under `~/.config/hyprbole/themes/<theme>`
- external source themes sync under `~/.config/hyprbole/themes/.sources/<source>/themes/<theme>`
- generated current theme files live under `~/.config/hyprbole/current/theme`
- the external theme source registry is `~/.config/hyprbole/theme-sources.conf`
- the seed source registry is `default/hyprbole/theme-sources.conf`

The installer should not overwrite existing user-owned files in `~/.config`. Refresh commands may regenerate files under `~/.config/hyprbole/current` because that directory is Hyprbole-owned runtime state.

External theme repositories are not vendored. `hyprbole theme source sync` is the explicit user action for pulling source themes into user config. Omarchy themes are an external source example, not repository content.

## Install Idempotency Model

`install.sh` must be safe to run repeatedly on the same system. Rerunning the installer is an expected repair path for Hyprbole-owned defaults, but it should not silently consume or duplicate user-owned state.

Repeat install runs may intentionally reapply:

- package installation with `--needed`
- user and system service enables
- Hyprbole-owned generated runtime state under `~/.config/hyprbole/current`
- browser launchers and MIME/default-browser associations
- browser policy symlinks
- SDDM theme/config files
- Limine/Snapper defaults
- Secret Service setup

Repeat install runs must not duplicate:

- shell source lines in profile files
- GNOME Keyring `login.keyring` backups when `login.keyring` is already absent
- SDDM PAM edits
- desktop entries
- theme source registry entries
- service definitions

User-owned config copied from `config/` must remain copy-if-missing by default. Commands that intentionally refresh user config must make that explicit, such as `refresh-all --include-user-configs`.

PAM changes must be deletion-safe and no-op when already applied. For Secret Service, removing SDDM `auth`/`password` `pam_gnome_keyring.so` hooks repeatedly is acceptable; session autostart hooks should remain.

## Update Model

Hyprbole updates are git-checkout based.

- `HYPRBOLE_PATH` defaults to `~/.local/share/hyprbole`
- `HYPRBOLE_PATH` should contain `.git`
- `hyprbole update` pulls that checkout before running migrations and refreshes
- install should convert older partial deployments by moving them aside and cloning a full checkout
- install should not create a separate checkout from a dirty source tree because uncommitted changes would be lost

Do not reintroduce partial deployment of only `bin/`, `config/`, `default/`, `themes/`, or `migrations/` into `HYPRBOLE_PATH`.

## Verification Rule

When encoding package names, command names, service names, policy paths, config paths, or external behavior, verify them from the repo manifests, installed files, command output, or upstream documentation first. Do not guess.

Package, user-service, refresh-path, diagnostic-path, doctor-path, and verify-path lists live in `default/hyprbole/manifest.sh`. Installer stages, diagnostics, doctor checks, verify checks, and refresh commands should read those arrays instead of duplicating package, unit, or path names in separate scripts.

## Command Model

Hyprbole uses a route-first command model.

- `hyprbole` is the primary human-facing CLI
- `bin/hyprbole-*` leaf scripts remain only for integration points that need stable executable paths, status probes, service jobs, app wrappers, or low-latency helpers

In practice:

- terminal users should prefer `hyprbole ...`
- short common routes such as `hyprbole menu`, `hyprbole lock`, and `hyprbole suspend` are preferred for frequent interactive use
- Hyprland binds, Walker/Elephant menus, Waybar actions, and Nautilus actions should call `hyprbole ...` routes directly when quoting is simple
- leaf scripts should not exist only to delegate to `hyprbole ...`; remove route-alias scripts unless a concrete executable-path need exists
- one-off repair/setup behavior should stay out of the public command surface unless it is a normal user workflow; prefer `doctor --fix` or internal leaf scripts

This keeps the user experience coherent while reserving leaf scripts for places where a script file adds real integration value.

`bin/hyprbole` sources `bin/hyprbole-lib.d/core` at startup and lazy-loads domain helpers with `hyprbole_load <module>` when a route needs them. The compatibility file `bin/hyprbole-lib` loads the current helper modules for older retained helper scripts that need the historical full helper surface.

Current helper modules:

- `core`: environment, manifest, settings, default apps, browser helpers
- `terminal`: terminal and floating-terminal launch behavior
- `theme`: font, theme source, wallpaper, and theme asset behavior
- `refresh`: user config refresh and backup pruning
- `mirrors`: mirror ranking workflow
- `theme-routes`, `package-routes`, `doctor-routes`, `update-routes`, `launch-routes`, `capture-routes`, `system-routes`, `maintenance-routes`: route-domain functions loaded only by matching dispatcher branches

## Browser Model

Brave Origin Nightly is the default browser package, but Hyprbole owns startup behavior through `bin/hyprbole-launch-brave-origin-nightly`.

The wrapper always passes:

- `--password-store=gnome-libsecret`
- `--ozone-platform-hint=auto`

`bin/hyprbole-refresh-browser-launchers` writes browser flags to `~/.config/brave-origin-nightly-flags.conf`, sets the system desktop entry as the default browser for XDG settings and common web MIME handlers, and suppresses unwanted system desktop entries (from transitive dependencies).

Keep `~/.config/brave-origin-nightly-flags.conf` safe for the upstream package wrapper, which does not handle multiple flags robustly. Hyprbole-specific extra Brave flags belong in `~/.config/hyprbole/brave-origin-nightly-flags.conf` and are read by the Hyprbole wrapper.

Do not globally enable Chromium fractional-scaling flags. They are monitor and Chromium-build dependent; users can opt in through the Hyprbole extra flags file.

## Secret Service Model

GNOME Keyring/libsecret owns Secret Service integration for Brave, VS Code, and other Chromium/Electron-style apps.

Install and repair behavior lives in `bin/hyprbole-setup-secret-service`. It is intentionally not exposed as a public `hyprbole secret-service` route. Users should run `hyprbole doctor --fix` if Secret Service checks fail.

Secret Service setup:

- creates a passwordless `~/.local/share/keyrings/Default_keyring.keyring`
- writes `~/.local/share/keyrings/default` with `Default_keyring`
- backs up an encrypted `~/.local/share/keyrings/login.keyring` if present
- removes SDDM `auth` and `password` `pam_gnome_keyring.so` hooks that recreate encrypted login keyrings
- keeps SDDM session autostart hooks so the daemon starts on login
- restarts `gnome-keyring-daemon.service` in the user session when possible

## Session Ownership

- Hyprland is launched through UWSM
- Waybar is the top bar
- `swaync` owns notifications
- `hyprlock` owns locking
- `hypridle` owns idle behavior
- `awww` owns wallpapers
- `walker` owns launching and clipboard history
- `satty` owns screenshot annotation after capture
- `1password-beta` is the curated password manager extra

## Session Services

Hyprbole uses user systemd services for session-critical long-running helpers where reliability matters more than simple compositor autostart.

Current examples include:

- Walker
- Elephant
- SwayOSD
- SwayNC
- polkit agent

Short-lived tray-style applications can still be launched from Hyprland autostart when they do not need service supervision.

User timers are tracked separately from graphical session services. Timers such as `hyprbole-trash-clean.timer` are enabled and started independently of `graphical-session.target`, and verification checks that they remain enabled and active so install reruns can repair them before the next GUI session.

## Hyprland Config Model

Hyprland is configured with Lua.

The user entrypoint should stay small and load:

- vendor defaults/framework from `~/.local/share/hyprbole`
- user modules from `~/.config/hypr`

This keeps the running config understandable while still supporting a reusable framework.

## Theme Model

Themes should define a shared set of visual primitives that can be projected into:

- Hyprland
- Waybar
- Ghostty
- `swaync`
- GTK settings
- Qt settings

The theme data model should be centralized and versionable.
