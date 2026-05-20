# Themes

Hyprbole themes define colors and assets that are projected into Hyprland, Waybar, Ghostty, SwayNC, GTK, VS Code, Neovim, and browser policy where supported.

Theme application is file-based. Hyprbole merges a selected theme into `~/.config/hyprbole/current/theme`, writes the selected theme name to `~/.config/hyprbole/current/theme-name`, and points `~/.config/hyprbole/current/background` at the active wallpaper.

## Theme Locations

| Location | Purpose |
| --- | --- |
| `~/.local/share/hyprbole/themes/<theme>` | Bundled themes shipped by Hyprbole |
| `~/.config/hyprbole/themes/<theme>` | User-authored themes |
| `~/.config/hyprbole/themes/.sources/<source>/themes/<theme>` | Synced external source themes |

The shipped fallback theme is `default-theme`.

## Commands

```bash
hyprbole theme list
hyprbole theme current
hyprbole theme set <theme>
hyprbole themes
```

`hyprbole theme set <theme>` also refreshes app theme files and restarts the affected desktop components where possible.

## External Sources

External theme repositories are configured in:

```text
~/.config/hyprbole/theme-sources.conf
```

Edit sources:

```bash
hyprbole theme source edit
```

Sync sources:

```bash
hyprbole theme source sync
```

Hyprbole does not vendor external themes into the repository. Omarchy themes are an example external source.

The default source registry already includes an Omarchy example. To add another source from the terminal, use:

```bash
hyprbole theme source add <name> <repo-url> <ref> <repo-theme-path> '$HYPRBOLE_CONFIG_PATH/themes/.sources/<name>/themes'
hyprbole theme source sync
```

After sync, external themes appear in `hyprbole theme list` and the theme picker.

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

Place wallpapers either in `backgrounds/` or as `wallpaper.jpg` in the theme root.

## Wallpapers

Theme wallpapers come from the active merged theme under:

```text
~/.config/hyprbole/current/theme/backgrounds
```

Use:

```bash
hyprbole theme wallpaper list
hyprbole theme wallpaper set <file>
hyprbole theme wallpaper cycle
```

For personal wallpapers independent of the current theme, put images in `~/Pictures/Wallpaper` and use:

```bash
hyprbole wallpaper set <file>
hyprbole wallpaper cycle
hyprbole wallpaper timer start custom 30min
```

Use `hyprbole wallpaper timer start theme 30min` to cycle only the current theme's wallpapers.

## Fonts

Hyprbole tracks three font roles: `ui`, `mono`, and `terminal`.

```bash
hyprbole theme font current
hyprbole theme font set ui "Inter"
hyprbole theme font set mono "JetBrains Mono"
hyprbole theme font set terminal "JetBrainsMono Nerd Font Mono"
hyprbole theme font set all "Inter"
```

Font choices are stored in `~/.config/hyprbole/settings.toml` and projected into supported apps by refresh scripts.
