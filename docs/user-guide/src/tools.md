# Utilities

Hyprbole's Utilities menu groups useful local system utilities and maintenance actions that are not primary apps. The first utility is Disk Usage, powered by `dua-cli`.

## Open Utilities

Open the Hyprbole menu:

```text
SUPER + ALT + SPACE
```

Then choose `System`, `Utilities`.

From a terminal, use:

```bash
hb tools
hb utilities
```

## Disk Usage

`Disk Usage` opens `dua i` in a floating terminal. `dua` is a fast terminal disk-usage browser.

Open it from the Utilities menu, search for `Disk Usage` in the app launcher, or run:

```bash
hb disk-usage
```

From a terminal, `hb disk-usage` runs `dua i` in the current terminal. Launcher and menu entries use `hb launch disk-usage` so a floating terminal opens for the tool.

## System Health

`System Health` opens the cached hourly health report. Waybar shows a health heart to the left of the date; it inherits the theme when there are no alerts, turns orange for warnings, and turns red for critical alerts.

Left-click the Waybar heart to open the report. Right-click it to refresh the health check and update the heart state.

Run it directly with:

```bash
hb health
hb health refresh
hb health updates
```

`hb health updates` is the quickest way to check whether your last full system update is stale or the Hyprbole repo has upstream updates available.

The Utilities menu also includes `System Monitor` for `btop`, `System Info` for Hyprland system information, `Mount Share` for NAS shares, and refresh actions for Hyprbole runtime state and shipped configs. App-like utility actions call `hyprbole launch ...` routes directly; refresh and toggle actions keep dedicated helper scripts where the script is the integration point.

`Refresh Desktop` reapplies theme and runtime state. `Refresh Shipped Configs` runs the same refresh with user config files included, which can overwrite local customizations after backing them up.

## Mount Share

`Mount Share` mounts SMB/CIFS or NFS network shares and can save Hyprbole-managed `/etc/fstab` entries marked with `# hyprbole mount-share`.

Mount Share installs missing helper packages with `sudo pacman` when needed, including `gum`, `cifs-utils`, `smbclient`, and `nfs-utils`.

For SMB shares with a username and password, credentials are stored under `/etc/smb-credentials` with root-only permissions. Hyprbole rejects share, export, and mount fields that would corrupt fstab entries, including whitespace and `#`, and it derives a safe credential filename from the SMB server and share name. NFS server input currently supports non-colon hostnames or IPv4 addresses, not IPv6 literals. Saved entries use `nofail`, `x-systemd.automount`, `x-systemd.mount-timeout=30`, and `_netdev` so boot is not blocked by an offline network share. Removing a saved share from the Mount Share workflow removes the managed fstab entry, offers to unmount the active mount, reloads systemd, and offers to remove the related Hyprbole-owned SMB credential file.

Open it from `System`, `Utilities`, `Mount Share`, or run:

```bash
hb launch mount-share
```

## Notifications

SwayNC owns desktop notifications. Waybar shows a notification bell on the right when SwayNC has notifications waiting.

| Action | Behavior |
| --- | --- |
| Left-click bell | Open or close the SwayNC panel |
| Middle-click bell | Toggle notification silence |
| Right-click bell | Dismiss all SwayNC notifications |

When notifications are silenced, Waybar shows the muted notification icon. If there are pending notifications while silenced, the count still appears.

## Settings Controls

Open the Settings submenu from `System`, `Settings`.

| Entry | Behavior |
| --- | --- |
| Audio | Opens `pavucontrol` through `hb launch audio` |
| Network | Opens NetworkManager connection editing through `hb launch network` |
| Bluetooth | Opens Blueman through `hb launch bluetooth` |
| Notifications | Toggles SwayNC notification silence |
| Idle Lock | Toggles `hypridle` idle locking |

Waybar also exposes the notification and idle controls. Click the idle indicator to toggle idle locking. Use the notification bell clicks described above for notification controls.

To inspect a specific path, pass it to Hyprbole:

```bash
hb disk-usage ~/Downloads
hb disk-usage ~/.local/share/hyprbole
```

You can also run `dua` directly:

```bash
dua i
dua i ~/Downloads
```

## Launcher Entry

Hyprbole generates this desktop entry so Walker's app launcher can find Disk Usage:

```text
~/.local/share/applications/hyprbole-disk-usage.desktop
```

Refresh generated tool launchers through the normal runtime refresh route:

```bash
hb refresh-all
```
