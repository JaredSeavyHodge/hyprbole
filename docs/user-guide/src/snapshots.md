# Snapshots

Hyprbole uses Snapper on Btrfs for root filesystem recovery. It is intended for pre-update recovery, not as a full personal backup system.

Snapshot support assumes the Arch install uses Btrfs for `/`.

## What Hyprbole Configures

Install configures Snapper when the Snapper and Limine tooling are installed and available.

Hyprbole creates a Snapper config named `root` for `/` if it is missing, then applies Hyprbole's root Snapper defaults:

```text
/etc/snapper/configs/root
```

The shipped defaults are intentionally small:

```text
SUBVOLUME="/"
FSTYPE="btrfs"
NUMBER_LIMIT="5"
NUMBER_LIMIT_IMPORTANT="5"
TIMELINE_CREATE="no"
```

That means Hyprbole keeps numbered root snapshots for recovery and does not create automatic timeline snapshots.

## Update Snapshots

`hb update` attempts to create a snapshot before package updates:

```bash
hb update
```

The snapshot command creates a numbered snapshot in every configured Snapper config and describes it as `hyprbole YYYY-MM-DD`. After creating snapshots, it runs Snapper's `number` cleanup for those configs.

## Manual Snapshots

Create a manual Hyprbole snapshot:

```bash
hb snapshot create
```

This is a wrapper around Snapper. It requires `snapper`; if Snapper is unavailable, the command exits without creating a snapshot.

## Inspect Snapshots

List Snapper configs:

```bash
sudo snapper list-configs
```

List root snapshots:

```bash
sudo snapper -c root list
```

Inspect root Snapper config values:

```bash
sudo snapper -c root get-config
```

Compare changed files between two snapshots:

```bash
sudo snapper -c root status <old>..<new>
```

Show file diffs between two snapshots:

```bash
sudo snapper -c root diff <old>..<new>
```

Use snapshot numbers from `sudo snapper -c root list`.

## Limine Integration

Hyprbole installs `limine`, `limine-snapper-sync`, and `limine-mkinitcpio-hook` as part of its package set.

Install enables `limine-snapper-sync.service` when the command exists, runs `limine-snapper-sync`, and enables `snapper-cleanup.timer`. Hyprbole also writes Limine defaults with `MAX_SNAPSHOT_ENTRIES=5`.

Refresh Limine and snapshot boot entries with:

```bash
hb refresh-limine
```

That command runs `limine-update` when available and `limine-snapper-sync` when available.

## Restore

Start the restore workflow:

```bash
hb snapshot restore
```

This runs:

```bash
sudo limine-snapper-restore
```

`limine-snapper-restore` also supports restoring kernel files from a specific Snapper snapshot ID:

```bash
sudo limine-snapper-restore --kernels <ID>
```

Use restore deliberately. A snapshot can help recover system files after a broken update, but it is not a substitute for backups of personal data.
