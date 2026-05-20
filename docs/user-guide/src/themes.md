# Themes

Hyprbole's theme system is inspired by Omarchy and uses the same general method: a theme is a folder of colors and app-specific assets that are projected into the desktop. This keeps themes understandable as files on disk instead of hiding them behind a large generator.

Hyprbole themes define colors and assets that are projected into Hyprland, Waybar, Ghostty, SwayNC, GTK, VS Code, Neovim, and browser policy where supported.

Because the model is compatible with Omarchy-style theme folders, Hyprbole can sync Omarchy's theme repository and make those themes available alongside bundled and user-authored themes.

Theme application is file-based. Hyprbole merges a selected theme into `~/.config/hyprbole/current/theme`, writes the selected theme name to `~/.config/hyprbole/current/theme-name`, and points `~/.config/hyprbole/current/background` at the active wallpaper. Treat `current/theme` as generated runtime state; edit bundled, synced, or user theme source directories instead.

## Theme Locations

| Location | Purpose |
| --- | --- |
| `~/.local/share/hyprbole/themes/<theme>` | Bundled themes shipped by Hyprbole |
| `~/.config/hyprbole/themes/<theme>` | User-authored themes |
| `~/.config/hyprbole/themes/.sources/<source>/themes/<theme>` | Synced external source themes |

The shipped fallback theme is `default-theme`.

## Commands

Open the Hyprbole menu with `SUPER + ALT + SPACE`, then choose Theme to browse theme, wallpaper, and font actions.

```bash
hb theme list
hb theme current
hb theme set <theme>
hb themes
```

`hb theme set <theme>` also refreshes app theme files and restarts the affected desktop components where possible.

## External Sources

External theme repositories are configured in:

```text
~/.config/hyprbole/theme-sources.conf
```

Install seeds this file from Hyprbole defaults when it is missing. The default registry includes Omarchy's theme repository:

```text
omarchy https://github.com/basecamp/omarchy.git dev themes $HYPRBOLE_CONFIG_PATH/themes/.sources/omarchy/themes
```

That line registers the source. It does not vendor Omarchy themes into Hyprbole and it does not download them during install.

Edit sources:

```bash
hb theme source edit
```

Sync sources:

```bash
hb theme source sync
```

Sync clones each configured source into a temporary directory, uses Git sparse checkout to fetch only the configured theme path, and replaces that source's local cache under `~/.config/hyprbole/themes/.sources/<source>/themes`.

After sync, external themes appear in `hb theme list` and the theme picker. Running sync again refreshes the local cache from the configured branch, tag, or commit.

To add another source from the terminal, use:

```bash
hb theme source add <name> <repo-url> <ref> <repo-theme-path> '$HYPRBOLE_CONFIG_PATH/themes/.sources/<name>/themes'
hb theme source sync
```

## User Themes

Put your own themes under:

```text
~/.config/hyprbole/themes/<theme>
```

User themes can override bundled or synced themes with the same name. Hyprbole merges theme sources in this order: bundled theme, synced external source theme, then user-authored theme.

Common theme files are:

| File | Purpose |
| --- | --- |
| `colors.toml` | Base colors used to generate missing app theme files |
| `waybar.css` | Waybar color overrides |
| `ghostty.conf` | Ghostty palette |
| `hyprland.lua` | Theme-specific Hyprland settings |
| `swayosd.css` | SwayOSD style |
| `btop.theme` | btop colors |
| `vscode.json` | VS Code theme data |
| `neovim.lua` | Neovim theme data |
| `icons.theme` | Preferred GTK icon theme name |
| `light.mode` | Marker file for light GTK mode |

Themes can include multiple wallpapers. Place them in `backgrounds/`, or use a single `wallpaper.jpg` in the theme root.

## Wallpapers

Theme wallpapers come from the active merged theme under:

```text
~/.config/hyprbole/current/theme/backgrounds
```

From the menu, open `SUPER + ALT + SPACE`, then choose Theme, Wallpaper Settings, Theme Wallpaper. Pick a wallpaper to apply it. Use My Wallpaper in the same menu to choose from `~/Pictures/Wallpaper`.

Wallpaper cycling is also available from Theme, Wallpaper Settings, Cycle Wallpaper. Choose Theme Wallpaper or My Wallpaper for the source, then choose an interval.

Use:

```bash
hb theme wallpaper list
hb theme wallpaper set <file>
hb theme wallpaper cycle
```

For personal wallpapers independent of the current theme, put images in `~/Pictures/Wallpaper` and use:

```bash
hb wallpaper set <file>
hb wallpaper cycle
hb wallpaper timer start custom 30min
```

Use `hb wallpaper timer start theme 30min` to cycle only the current theme's wallpapers.

## Fonts

Hyprbole tracks three font roles: `ui`, `mono`, and `terminal`.

```bash
hb theme font current
hb theme font set ui "Inter"
hb theme font set mono "JetBrains Mono"
hb theme font set terminal "JetBrainsMono Nerd Font Mono"
hb theme font set all "Inter"
```

Font choices are stored in `~/.config/hyprbole/settings.toml` and projected into supported apps by refresh scripts.
