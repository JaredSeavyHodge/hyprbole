# Implementation Plan

## Phase 1

- Create repository structure. Done.
- Add install scaffolding. Done.
- Define package manifests. Done.
- Define session ownership matrix. Done.
- Define Hyprland Lua entrypoint layout. Done.
- Define Waybar baseline config. Done.

## Phase 2

- Implement post-install bootstrap script. Done.
- Install official packages. Done.
- Install `yay`. Done.
- Install curated AUR packages. Done.
- Configure git from user prompts. Done.
- Deploy vendor defaults and user config. Done.
- Configure Secret Service for GNOME Keyring/libsecret. Done.
- Generate Hyprbole-owned Brave launcher and default-browser association. Done.

## Phase 3

- Implement theme data model. Done.
- Ship the first default theme as `default-theme`. Done.
- Apply theme to Hyprland, Waybar, Ghostty, and `swaync`. Done.
- Add external theme source registry and explicit sync workflow. Done.

## Phase 4

- Add diagnostics and update helpers. In progress.
- Add reset/refresh flows. In progress.
- Add higher-level `hyprbole` command surface. In progress.
- Keep one-off repair commands behind `hyprbole doctor --fix` rather than public routes. In progress.

## Future Features

- Add a Walker/Elephant menu for switching runtime default apps such as browser, terminal, and file manager.
- Add optional Walker/Elephant provider installation from the Hyprbole menu for extras such as package search, provider list, bookmarks, snippets, bluetooth, and window actions.
- Store selected runtime defaults in `~/.config/hyprbole/settings.toml` and keep install-time package defaults hard-coded for now.
- Have runtime wrappers and menus honor selected defaults for roles such as browser, terminal, file manager, image viewer, video player, and PDF viewer.
- Add a MIME/default-app refresh command that applies selected roles through `xdg-mime` and `xdg-settings` for web links, directories, images, video/audio files, PDFs, text/code files, and archives.
- Let the default-app menu install missing supported alternatives, update `settings.toml`, refresh MIME associations, and restart or reload affected runtime components.
- Add a CLI cleanup pass before broadening the public `hyprbole` command surface.
- Consider an opt-in browser scaling helper that writes Chromium fractional-scaling flags to `~/.config/hyprbole/brave-origin-nightly-flags.conf`.

## Current Open Questions

- Whether to keep SDDM as the v1 display-manager default long term.
- Exact GTK/Qt theming package set and application flow beyond current baseline.
- How much of the current `hyprbole` command tree should remain public after CLI cleanup.
