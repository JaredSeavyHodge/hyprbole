# Architecture

## Layer Split

Hyprbole is intentionally split into two layers:

1. Base install layer
   - owned by `archinstall`
   - partitions, bootloader, user creation, locale, networking, filesystem

2. Desktop layer
   - owned by `hyprbole`
   - packages, AUR tooling, defaults, theming, session behavior, shell commands, user-facing CLI

## Config Ownership

Hyprbole follows a vendor-defaults plus user-config model.

- Vendor-owned files live under `~/.local/share/hyprbole`
- User-owned editable files live under `~/.config`

Repository mapping:

- `config/` -> `~/.config/...`
- `default/` -> `~/.local/share/hyprbole/...`
- `themes/` -> `~/.local/share/hyprbole/themes/...`

## Command Model

Hyprbole uses a hybrid command model.

- `hyprbole` is the primary human-facing CLI
- `bin/hyprbole-*` leaf scripts remain for integration points that need stable executable paths

In practice:

- terminal users should prefer `hyprbole ...`
- short common routes such as `hyprbole menu`, `hyprbole lock`, and `hyprbole suspend` are preferred for frequent interactive use
- Hyprland binds, Walker/Elephant menus, systemd units, and Nautilus actions can continue to call leaf scripts
- leaf scripts should prefer delegating to `hyprbole ...` when the behavior is part of the public command surface

This keeps the user experience coherent without forcing desktop integration through a single giant script entrypoint.

## Session Ownership

- Hyprland is launched through UWSM
- Waybar is the top bar
- `swaync` owns notifications
- `hyprlock` owns locking
- `hypridle` owns idle behavior
- `awww` owns wallpapers
- `walker` owns launching and clipboard history
- `satty` owns screenshot annotation after capture
- `1password-beta` is the default password manager

## Session Services

Hyprbole uses user systemd services for session-critical long-running helpers where reliability matters more than simple compositor autostart.

Current examples include:

- Walker
- Elephant
- SwayOSD
- polkit agent

Short-lived tray-style applications can still be launched from Hyprland autostart when they do not need service supervision.

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
