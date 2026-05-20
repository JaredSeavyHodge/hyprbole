# Packages

Hyprbole installs official Arch packages and a small curated AUR package set.

## Commands

Open package menus:

```bash
hyprbole pkg installed
hyprbole pkg install
hyprbole pkg aur-install
hyprbole pkg remove
```

The Hyprbole menu also includes package install and removal entries.

## Updates

Run:

```bash
hyprbole update
```

Update will pull the Hyprbole checkout, update system packages, update AUR packages, run migrations, refresh Hyprbole runtime state, and print a summary.

## Package Cache

If disk space is low, inspect:

```bash
df -h / /home
```

The package cache at `/var/cache/pacman/pkg` can grow over time. Hyprbole update trims package cache with `paccache` when available.
