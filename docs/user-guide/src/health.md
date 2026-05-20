# System Health

Hyprbole runs a lightweight system health check every hour.

The check reads cached device and filesystem metadata. It does not run surface scans, long SMART tests, or write-heavy diagnostics.

## Waybar Alert

Waybar shows no health icon when the cached report is clean.

If warnings or critical issues are found, Waybar shows a warning icon. Click it to open the health report.

## Commands

```bash
hb health
hb launch health
```

`hb health` prints the cached report. `hb launch health` opens the report in a floating terminal.

## Checks

Hyprbole currently checks:

- NVMe SMART health through `nvme-cli`.
- SATA/SCSI SMART health through `smartmontools`.
- Btrfs-aware local filesystem usage.
- Whether `/` is unexpectedly mounted read-only.
- Failed system services.

Virtual block devices such as VM `vd*` disks are skipped for SMART checks because they usually do not expose physical drive health data.

Disk usage warns when a local filesystem is at or above `95%` full. Btrfs subvolumes on the same backing filesystem are grouped so the same full Btrfs filesystem is not reported repeatedly.

NVMe wear warns at `80%` used and becomes critical at `100%` used. This is the drive's vendor endurance estimate, not filesystem usage.

Package updates, package installs, and the installer refuse to start package operations when any local filesystem is at or above `98%` full. Free space first, then retry.

## Fresh Check

To run a fresh check manually:

```bash
sudo systemctl start hyprbole-health-check.service
```

The cached report lives at:

```text
/var/lib/hyprbole/health.json
```
