# Packages

Hyprbole installs official Arch packages and a small curated AUR package set.

Official packages are declared in `install/packages.sh`. AUR packages are declared in `install/aur.sh`.

## Commands

Open package menus:

```bash
hb pkg installed
hb pkg install
hb pkg aur-install
hb pkg remove
```

The Hyprbole menu also includes package workflows at the top level. Open `Install` for `Pacman` and `AUR`; open `Remove` for installed package inspection and removal.

These commands open interactive picker workflows. Use normal `pacman` or `yay` directly when you already know the exact package operation you want.

## Updates

Run:

```bash
hb update
```

Update will pull the Hyprbole checkout, update system packages, update AUR packages, run migrations, refresh Hyprbole runtime state, and print a summary.

Before updating packages, Hyprbole attempts to create a Snapper snapshot when Snapper is available. After updating, it refreshes runtime state with `hyprbole-refresh-all` and updates Limine entries when possible.

The update command writes a terminal session log to:

```text
/tmp/hyprbole-update.log
```
