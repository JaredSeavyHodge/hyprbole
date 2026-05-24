# Hyprbole Agent Guide

## Purpose

This repository defines the `hyprbole` desktop layer for Arch Linux. The project should feel polished and opinionated, but remain understandable and maintainable.

## Architecture Model

- `archinstall` handles base system installation
- `hyprbole` handles the desktop environment layer after first boot
- `config/` mirrors the editable `~/.config` surface
- `default/` contains vendor-owned framework pieces and templates
- `themes/` contains shipped theme packs only
- external theme repositories are synced into user config, not vendored into this repo

## Current Product Direction

- Hyprland with Lua-first config
- UWSM-managed startup
- Waybar instead of Noctalia for v1
- `hyprbole` as the main command surface for users in a terminal
- Ghostty terminal
- Brave Nightly (`brave-origin-nightly-bin`) browser
- Nautilus file manager
- `yay` is installed and used for a small curated AUR package set
- GNOME Keyring/libsecret owns app Secret Service integration
- Browser startup is owned by Hyprbole wrappers rather than package launchers where needed

## Editing Guidance

- Prefer the smallest correct change
- Keep naming direct and boring
- This is initial development and users are not running Hyprbole yet; do not add backward-compatibility branches, migrations, or legacy handling unless explicitly requested
- Never guess repo facts when editing. Verify package names, commands, service names, file paths, config locations, policy paths, and external behavior from repo manifests, installed files, command output, or upstream documentation before encoding them.
- Add new scripts as shell scripts unless there is a clear reason otherwise
- Avoid creating abstraction layers until repeated patterns are obvious
- When introducing user-facing commands in `bin/`, add metadata comments near the top of the file
- Prefer extending `bin/hyprbole` for human-facing terminal workflows
- Prefer short top-level `hyprbole` routes for very common interactive actions when they improve ergonomics without creating ambiguity
- Keep `bin/hyprbole-*` leaf scripts for integration points such as Hyprland keybinds, Walker/Elephant menus, systemd services, and file manager actions
- When a leaf script and `bin/hyprbole` overlap, prefer making the leaf script a thin wrapper around `hyprbole ...`
- Shared workflow logic belongs in `bin/hyprbole` or `bin/hyprbole-lib`; leaf scripts should forward args and add integration context, not duplicate route parsing, install fallback policy, terminal launch policy, or persistence logic
- Do not expose one-off install repair commands on the public `hyprbole` command surface unless they are normal user workflows; prefer `doctor --fix` or internal leaf scripts

## Design Decisions To Preserve

- `hyprbole` is the human CLI; `bin/hyprbole-*` scripts are integration leaf scripts for menus, keybinds, systemd, Nautilus, and wrappers
- Leaf scripts should be thin wrappers around `hyprbole ...` when the workflow is user-facing
- Terminal workflows launched from menus must open in floating terminals through `hyprbole_launch_floating_terminal` or `hyprbole-launch-floating`
- Hyprland floating terminal windows use `com.hyprbole.*` classes and are covered by the default floating window rule
- Floating terminal helpers must preserve the configured `com.hyprbole.*` class/title metadata for supported terminal backends, or fail loudly instead of silently opening a non-floating terminal
- Avoid duplicated menu entries across sibling submenus unless there is a deliberate product reason; prefer one canonical menu location and link to it
- `doctor` should not require sudo unless running `doctor --fix`; normal checks must use non-privileged probes where possible
- Package arrays must not drift: packages checked as required by doctor should be installed by the default install path, or explicitly marked optional/lazy
- `default/systemd/user/` contains Hyprbole-owned user units and should be deployed as vendor state; `config/` remains user-owned copy-if-missing config
- User timers should be modeled separately from graphical session services; do not put timers in graphical-service groups unless they are actually session-bound
- Network share persistence is owned by the Mount Share workflow. It uses `/etc/fstab` entries marked with `# hyprbole mount-share`, safe automount options, validated fstab fields, and SMB credentials under `/etc/smb-credentials` with root-only permissions
- Browser flags have two paths: system Brave desktop launches read `~/.config/brave-origin-nightly-flags.conf`; Hyprbole wrapper launches read `~/.config/hyprbole/brave-origin-nightly-flags.conf`
- Synced external themes under `~/.config/hyprbole/themes/.sources/` are source-owned and should not be edited directly; use user-authored theme overrides or install missing dependencies
- Docs must be updated whenever user-facing commands, menu entries, extras, install behavior, or persistence/security behavior changes

## Before Adding User-Facing Features

- Add or update the `hyprbole` route if the workflow is normal terminal use
- Add a leaf script only when needed for menus, keybinds, services, or file-manager integration
- If a workflow exists in both CLI and integration form, implement it once as a `hyprbole` route and make the leaf script a thin `exec hyprbole ... "$@"` wrapper
- If the workflow opens an interactive terminal from a menu, make it floating
- Add package dependencies to the correct install set or make the workflow install/check them lazily
- Add doctor checks only if they are actionable and non-sudo by default
- Keep install reruns idempotent: no duplicate fstab entries, service definitions, source lines, desktop entries, PAM lines, or backups
- Update user-guide docs and implementation-plan status in the same change
- When adding a new command family, launcher pattern, menu taxonomy, or persistence/security workflow, update the relevant `.opencode/agent/*.md` reviewer scope so future reviews cover it

## Config Guidance

- User-editable config should live under `config/`
- Vendor defaults/framework should live under `default/`
- Theme-specific data should live under `themes/`
- Hyprland config should remain modular and should not collapse into a single giant Lua file
- Waybar config should be simple and robust before becoming feature-dense

## Install Idempotency

- `install.sh` should remain safe to run multiple times on the same system
- Re-running install is an expected repair path for some Hyprbole-owned defaults
- Install may reapply Hyprbole-owned generated/runtime state, browser launchers, service enables, system policy links, SDDM theme files, and Secret Service setup
- Install must not overwrite user-owned config copied from `config/` when the destination already exists, except where a command explicitly asks to refresh user configs
- Repeat runs must not duplicate shell source lines, keyring backups, PAM lines, desktop entries, source registry entries, or service definitions
- PAM edits must remain idempotent; deleting already-removed GNOME Keyring `auth`/`password` hooks is acceptable and should be a no-op
- If an install step cannot be made no-op, document the intentional reapplication and keep it limited to Hyprbole-owned state

## Current Implementation Facts

- Shipped fallback theme is `themes/default-theme`
- User-authored themes live under `~/.config/hyprbole/themes/<theme>`
- Synced external source themes live under `~/.config/hyprbole/themes/.sources/<source>/themes/<theme>`
- Theme source registry lives at `~/.config/hyprbole/theme-sources.conf`
- Repo seed for that registry lives at `default/hyprbole/theme-sources.conf`
- Install seeds `theme-sources.conf` only when missing and does not auto-sync external theme repos
- `hyprbole theme source sync` is the explicit sync command for external theme repos
- Omarchy themes are an example external source, not vendored repo content
- Secret Service setup is install-owned via `bin/hyprbole-setup-secret-service`
- Users should repair Secret Service through `hyprbole doctor --fix`, not a public `secret-service` command
- Secret Service setup creates a passwordless `Default_keyring`, backs up an encrypted `login.keyring`, removes SDDM `auth`/`password` GNOME Keyring PAM hooks, keeps session autostart hooks, and restarts the user keyring daemon when possible
- Brave Origin Nightly is launched through `bin/hyprbole-launch-brave-origin-nightly`
- `bin/hyprbole-refresh-browser-launchers` writes browser flags to `~/.config/brave-origin-nightly-flags.conf` and sets the system desktop entry as the default browser
- Gaming is surfaced through a `hyprbole-gaming` Elephant submenu with Steam launch, Gamescope gaming mode launch, and a link to the shared curated Extras install menu
- `bin/hyprbole-launch-steam` and `bin/hyprbole-launch-steam-gaming-mode` are the leaf scripts for gaming menu actions
- Hyprbole's Brave wrapper always passes `--password-store=gnome-libsecret` and `--ozone-platform-hint=auto`
- Keep `~/.config/brave-origin-nightly-flags.conf` single-flag safe for the package wrapper; user extra Brave flags belong in `~/.config/hyprbole/brave-origin-nightly-flags.conf`
- Do not enable Chromium fractional-scaling flags globally; keep them opt-in through the Hyprbole Brave extra flags file

## Priorities For Early Work

1. Bootstrap install flow
2. Package manifests for official and AUR packages
3. Hyprland Lua config framework
4. Waybar baseline config and styling
5. Theme data model and first shipped theme
6. Basic `hyprbole` command surface

## Things To Avoid Early

- Full distro installer replacement
- Multiple competing shell layers
- Large plugin systems
- Heavy config generation that obscures what the user is running
- Too many optional branches before the default path feels solid
