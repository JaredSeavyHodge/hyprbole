# Implementation Plan

## Phase 1

- Create repository structure
- Add install scaffolding
- Define package manifests
- Define session ownership matrix
- Define Hyprland Lua entrypoint layout
- Define Waybar baseline config

## Phase 2

- Implement post-install bootstrap script
- Install official packages
- Install `yay`
- Install curated AUR packages
- Configure git from user prompts
- Deploy vendor defaults and user config

## Phase 3

- Implement theme data model
- Ship the first default theme
- Apply theme to Hyprland, Waybar, Ghostty, and `swaync`

## Phase 4

- Add diagnostics and update helpers
- Add reset/refresh flows
- Add higher-level `hyprbole` command surface

## Current Open Questions

- Exact package target or replacement for `hyprcapture`
- Whether a display manager belongs in v1
- Exact GTK/Qt theming packages and application flow
