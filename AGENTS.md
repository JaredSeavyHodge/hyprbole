# Hyprbole Agent Guide

## Purpose

This repository defines the `hyprbole` desktop layer for Arch Linux. The project should feel polished and opinionated, but remain understandable and maintainable.

## Architecture Model

- `archinstall` handles base system installation
- `hyprbole` handles the desktop environment layer after first boot
- `config/` mirrors the editable `~/.config` surface
- `default/` contains vendor-owned framework pieces and templates
- `themes/` contains theme packs and theme source data

## Current Product Direction

- Hyprland with Lua-first config
- UWSM-managed startup
- Waybar instead of Noctalia for v1
- `hyprbole` as the main command surface for users in a terminal
- Ghostty terminal
- Brave Nightly (`brave-origin-nightly-bin`) browser
- Nautilus file manager
- `yay` is installed and used for a small curated AUR package set

## Editing Guidance

- Prefer the smallest correct change
- Keep naming direct and boring
- Add new scripts as shell scripts unless there is a clear reason otherwise
- Avoid creating abstraction layers until repeated patterns are obvious
- When introducing user-facing commands in `bin/`, add metadata comments near the top of the file
- Prefer extending `bin/hyprbole` for human-facing terminal workflows
- Prefer short top-level `hyprbole` routes for very common interactive actions when they improve ergonomics without creating ambiguity
- Keep `bin/hyprbole-*` leaf scripts for integration points such as Hyprland keybinds, Walker/Elephant menus, systemd services, and file manager actions
- When a leaf script and `bin/hyprbole` overlap, prefer making the leaf script a thin wrapper around `hyprbole ...`

## Config Guidance

- User-editable config should live under `config/`
- Vendor defaults/framework should live under `default/`
- Theme-specific data should live under `themes/`
- Hyprland config should remain modular and should not collapse into a single giant Lua file
- Waybar config should be simple and robust before becoming feature-dense

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
