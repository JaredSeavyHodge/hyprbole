# Hyprbole

Hyprbole is a curated post-install desktop layer for Arch Linux built around Hyprland, UWSM, Waybar, and a managed defaults model inspired by Omarchy.

`archinstall` owns base installation. `hyprbole` owns the desktop stack, defaults, scripts, themes, and ongoing system personality.

## Current Scope

- Post-`archinstall` bootstrap only
- Hyprland Lua config as the primary compositor config format
- Waybar as the default status bar
- `hyprbole` as the primary human-facing command surface
- Walker as the default launcher and clipboard picker
- Ghostty as the default terminal
- `brave-origin-nightly-bin` as the default browser
- Nautilus as the default file manager
- Managed vendor defaults under `~/.local/share/hyprbole`
- User-facing editable config under `~/.config`

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
- `hyprbole secret-service setup`
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
- Password manager: `1password-beta`

## Status

Current desktop defaults include:

- `SUPER + SPACE`: Walker launcher
- `SUPER + V`: Walker clipboard history
- `Print`: region screenshot into `satty`
- `Shift + Print`: fullscreen screenshot into `satty`
- `SUPER + SHIFT + /`: `1Password`
- `SUPER + S`: toggle scratchpad workspace

Session-critical background components are increasingly service-managed through user systemd units, including:

- Walker
- Elephant
- SwayOSD
- polkit agent
