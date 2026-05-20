# Themes

Hyprbole themes define colors and assets that are projected into Hyprland, Waybar, Ghostty, SwayNC, GTK, VS Code, Neovim, and browser policy where supported.

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
```

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
