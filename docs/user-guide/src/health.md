# System Health

Hyprbole runs a lightweight system health check every hour.

The check reads cached device, filesystem, and update-freshness metadata. It does not run surface scans, long SMART tests, or write-heavy diagnostics.

## Waybar Alert

Waybar shows a health bell to the left of the date. When the cached report is clean, it inherits the active theme colors and its tooltip says `No Alerts`.

If warnings or critical issues are found, the bell turns orange for warnings and red for high alerts. Left-click it to open the health report. Right-click it to run a fresh health check and reset the bell state from the updated report.

## Commands

```bash
hb health
hb health refresh
hb health updates
hb launch health
```

`hb health` prints the cached report. `hb health refresh` runs a fresh check through the installed systemd service, then prints the updated report. `hb health updates` checks package update age and Hyprbole repo freshness directly. `hb launch health` opens the report in a floating terminal.

## Checks

Hyprbole currently checks:

- NVMe SMART health through `nvme-cli`.
- SATA/SCSI SMART health through `smartmontools`.
- Btrfs-aware local filesystem usage.
- Whether `/` is unexpectedly mounted read-only.
- Failed system services.
- Whether the last full package update is older than `14` days.
- Whether the Hyprbole git checkout has upstream updates available.

Virtual block devices such as VM `vd*` disks are skipped for SMART checks because they usually do not expose physical drive health data.

Disk usage warns when a local filesystem is at or above `95%` full. Btrfs subvolumes on the same backing filesystem are grouped so the same full Btrfs filesystem is not reported repeatedly.

NVMe wear warns at `80%` used and becomes critical at `100%` used. This is the drive's vendor endurance estimate, not filesystem usage.

Package updates, package installs, and the installer refuse to start package operations when any local filesystem is at or above `98%` full. Free space first, then retry.

Update freshness warnings come from the separate `hyprbole-health-updates` probe. It reads `/var/log/pacman.log` for the latest full system upgrade and performs a short-timeout git remote check for the Hyprbole repo. Network failures are ignored rather than shown as health warnings.

## Fresh Check

To run a fresh check manually:

```bash
hb health refresh
```

If the desktop authorization prompt is unavailable, run the service directly with `sudo systemctl start hyprbole-health-check.service`.

The cached report lives at:

```text
/var/lib/hyprbole/health.json
```
