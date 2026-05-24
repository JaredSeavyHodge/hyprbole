# Packages

Hyprbole installs the core desktop package set, default role apps, and a small curated AUR package set. Optional app integrations live in Extras so the base desktop does not require every preferred app.

Package sets are declared in `default/hyprbole/manifest.sh`.

## Commands

Open package menus:

```bash
hb pkg installed
hb pkg install
hb pkg aur-install
hb pkg remove
hb extra list
hb extra install vscode
```

The Hyprbole menu also includes package workflows at the top level. Open `Install` for curated extras, `Pacman`, and `AUR`; open `Remove` for installed package inspection and removal.

These commands open interactive picker workflows. Use normal `pacman` or `yay` directly when you already know the exact package operation you want.

## Curated Extras

Curated extras are optional apps with Hyprbole integration such as launchers, theme refresh hooks, Nautilus context actions, or Walker providers.

Install them from the Hyprbole menu:

```text
Install -> Extras
```

Or from a terminal:

```bash
hb extra list
hb extra install vscode
hb extra install onepassword
hb extra install obsidian
hb extra install steam
```

The Gaming menu links to the same Extras submenu for Steam, Discord, and related optional app installs so there is one canonical curated install location.

Current extras:

| Extra | What it adds |
| --- | --- |
| VS Code | Editor install, theme data, Secret Service flags, Nautilus context action when `code` exists |
| 1Password | Password manager, launcher action, Walker/Elephant password provider |
| Obsidian | Markdown knowledge-base app and Hyprbole keybind launcher |
| Steam | Steam, Gamescope, GameMode, MangoHud, and controller support best-effort setup |
| Discord | Discord chat client |
| LibreOffice | LibreOffice office suite |

Mousepad, Papers, Apostrophe, and Neovim are installed with the base desktop because they are basic local file/editing tools. Doctor checks the selected runtime default roles by command availability and reports curated extras only when they are installed.

## Updates

Run:

```bash
hb update
hb launch update
```

Update will pull the Hyprbole checkout, update system packages, update AUR packages, run migrations, refresh Hyprbole runtime state, and print a summary.

`hb launch update` opens the same update flow in a floating terminal and asks for confirmation before starting. The Waybar update icon uses this launcher when the last full system update is older than the configured warning threshold.

Before updating packages, Hyprbole attempts to create a Snapper snapshot when Snapper is available. After updating, it refreshes runtime state with `hyprbole-refresh-all` and updates Limine entries when possible.

Hyprbole checks local filesystem usage before package operations. If a local filesystem is at or above `98%` usage, package installs and updates are refused until you free space.

Check update freshness without running an update:

```bash
hb health updates
```

The health update probe reads `/var/log/pacman.log` for the last full system upgrade and checks whether the Hyprbole repo has upstream changes available.

The update command writes a terminal session log to:

```text
/tmp/hyprbole-update.log
```
