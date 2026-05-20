# Packages

Hyprbole installs official Arch packages and a small curated AUR package set.

Official packages are declared in `install/packages.sh`. AUR packages are declared in `install/aur.sh`.

## Commands

Open package menus:

```bash
hyprbole pkg installed
hyprbole pkg install
hyprbole pkg aur-install
hyprbole pkg remove
```

The Hyprbole menu also includes package install and removal entries.

These commands open interactive picker workflows. Use normal `pacman` or `yay` directly when you already know the exact package operation you want.

## Updates

Run:

```bash
hyprbole update
```

Update will pull the Hyprbole checkout, update system packages, update AUR packages, run migrations, refresh Hyprbole runtime state, and print a summary.

Before updating packages, Hyprbole attempts to create a Snapper snapshot when Snapper is available. After updating, it refreshes runtime state with `hyprbole-refresh-all` and updates Limine entries when possible.

The update command writes a terminal session log to:

```text
/tmp/hyprbole-update.log
```

## Snapshots

Create a manual snapshot:

```bash
hyprbole snapshot create
```

Start the restore workflow:

```bash
hyprbole snapshot restore
```

Snapshot restore uses `limine-snapper-restore` when available.

## Package Cache

If disk space is low, inspect:

```bash
df -h / /home
```

The package cache at `/var/cache/pacman/pkg` can grow over time. Hyprbole update trims package cache with `paccache` when available.
