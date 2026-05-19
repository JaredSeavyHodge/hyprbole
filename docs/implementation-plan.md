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

## Future Features

- Add a Walker/Elephant menu for switching runtime default apps such as browser, terminal, and file manager.
- Add optional Walker/Elephant provider installation from the Hyprbole menu for extras such as package search, provider list, bookmarks, snippets, bluetooth, and window actions.
- Store selected runtime defaults in `~/.config/hyprbole/settings.toml` and keep install-time package defaults hard-coded for now.
- Have runtime wrappers and menus honor selected defaults for roles such as browser, terminal, file manager, image viewer, video player, and PDF viewer.
- Add a MIME/default-app refresh command that applies selected roles through `xdg-mime` and `xdg-settings` for web links, directories, images, video/audio files, PDFs, text/code files, and archives.
- Let the default-app menu install missing supported alternatives, update `settings.toml`, refresh MIME associations, and restart or reload affected runtime components.

## Current Open Questions

- Exact package target or replacement for `hyprcapture`
- Whether a display manager belongs in v1
- Exact GTK/Qt theming packages and application flow
