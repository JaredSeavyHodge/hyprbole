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
- Open the local user guide automatically on first login after fresh install. Done.
- Remove the temporary `hyprbole-mount-share` script. Done.
- Add troubleshooting guidance for refreshing a single user-owned `.config` file. Done.
- Do a focused user-guide cleanup before the next fresh install test. Done.
- Add high-value `hb restart` routes for common UI workflows. Done.
- Change the Waybar health indicator from a bell to a heart. Done.
- Add a lightweight Waybar notification bell for SwayNC with right-click dismissal. Done.
- Evaluate lightweight, nicely themable text/PDF default app candidates. Done.
- Use `mousepad` and `papers` as default text/PDF apps. Done.
- Evaluate clean, free, lightweight Markdown viewer candidates. Done.
- Add a simple TUI for mounting NAS NFS or SMB shares with proper permissions. Done.
- Add a main Walker `Gaming` submenu with Steam launch, a shared Extras install link, and Steam gaming mode. Done.

## Future Features

- Add a Walker/Elephant menu for switching runtime default apps such as browser, terminal, and file manager.
- Add optional Walker/Elephant provider installation from the Hyprbole menu for extras such as package search, provider list, bookmarks, snippets, bluetooth, and window actions.
- Store selected runtime defaults in `~/.config/hyprbole/settings.toml` and keep install-time package defaults hard-coded for now.
- Have runtime wrappers and menus honor selected defaults for roles such as browser, terminal, file manager, image viewer, video player, and PDF viewer.
- Add a MIME/default-app refresh command that applies selected roles through `xdg-mime` and `xdg-settings` for web links, directories, images, video/audio files, PDFs, text/code files, and archives.
- Let the default-app menu install missing supported alternatives, update `settings.toml`, refresh MIME associations, and restart or reload affected runtime components.
- Add a CLI cleanup pass before broadening the public `hyprbole` command surface.
- Consider an opt-in browser scaling helper that writes Chromium fractional-scaling flags to `~/.config/hyprbole/brave-origin-nightly-flags.conf`.
- Discuss whether a deeply integrated optional agentic OS layer, similar to Hermes, is viable with efficient access to Hyprland, Hyprbole, and Linux while preserving security and avoiding excessive tool calls or token use.
- Add optional `xournalpp` or `okular` install workflows for PDF signing/annotation.
- Consider a `glow`-backed Markdown viewer role or launcher for read-only Markdown viewing; keep `text/markdown` editable in Mousepad unless users explicitly choose a viewer workflow.
- Add a Walker `Install` submenu for office suites, including LibreOffice and any better alternatives, with an option to consider a Brave-powered Microsoft Office web app.

## Current Open Questions

- Whether to keep SDDM as the v1 display-manager default long term.
- Exact GTK/Qt theming package set and application flow beyond current baseline.
- How much of the current `hyprbole` command tree should remain public after CLI cleanup.
