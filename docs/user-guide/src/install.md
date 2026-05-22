# Install

### _if you're here, you've probably done this._

Hyprbole is installed after a minimal Arch Linux install. `archinstall` should create the OS, user, bootloader, filesystem, mirrors, locale, and networking. Hyprbole then installs and owns the desktop layer.

## Archinstall Wrapper

From the Arch ISO, connect networking first, then run Hyprbole's archinstall wrapper:

```bash
curl -fsSL https://raw.githubusercontent.com/jaredseavyhodge/hyprbole/master/install/archinstall/hyprbole-archinstall -o /tmp/hyprbole-archinstall
chmod +x /tmp/hyprbole-archinstall
/tmp/hyprbole-archinstall
```

The wrapper runs `archinstall` with Hyprbole base defaults preselected. It avoids the full Archinstall TUI by default and uses a lightweight picker flow for the target disk, timezone, Archinstall language, system locale, and keyboard layout. It also asks for hostname, sudo username, and sudo user password before running `archinstall --silent`.

If you want the normal Archinstall TUI with Hyprbole's config loaded, run:

```bash
/tmp/hyprbole-archinstall --interactive
```

If you want to inspect the generated config first, run:

```bash
/tmp/hyprbole-archinstall --config-path /tmp/hyprbole-base.json --generate-only
```

`--generate-only` only writes the JSON, so it can be used from an already installed system. Running `archinstall` still belongs on the Arch ISO.

Preselected defaults:

- Profile: no desktop profile.
- Bootloader: Limine.
- Kernel: `linux`.
- Audio: PipeWire.
- Network configuration: NetworkManager.
- Disk layout: Btrfs with `/`, `/home`, `/var/log`, and pacman package-cache subvolumes.
- Swap: zram with `zstd`.
- Packages: `git` only.

The picker prompts show numbered options. Type `/text` to filter the list, then choose a number. Hostname and username are typed directly because they are machine-specific names.

Choose carefully when the wrapper prompts:

- Target disk. It will be wiped.
- Hostname.
- Timezone, Archinstall language, system locale, and keyboard layout from filterable lists.
- Sudo username and password.

Disk encryption is not enabled by the default wrapper config yet. Silent install supports one target disk. Use `--interactive` if you need encryption, multi-disk Btrfs, or any custom storage layout.

After `archinstall` finishes, reboot into the installed system. You should land at a non-desktop terminal TTY, not a graphical login screen.

## Manual Archinstall Choices

From the Arch ISO, run `archinstall` without selecting a desktop profile. Hyprbole expects to install Hyprland, UWSM, Waybar, browser, file manager, and desktop services itself.

Recommended Arch choices:

- Profile: no desktop profile.
- Locale and keyboard: choose for your machine.
- Mirrors: choose sane local mirrors.
- Authentication: create your normal user with administrator or sudo access.
- Bootloader: Limine.
- Disk layout: Btrfs.
- Audio: PipeWire.
- Network configuration: NetworkManager.
- Packages: include `git`.

After `archinstall` finishes, reboot into the installed system. You should land at a non-desktop terminal TTY, not a graphical login screen.

## Clone Hyprbole

Log in as your normal user and make sure networking is up. If needed, connect with NetworkManager tooling from the TTY.

Clone the repo:

```bash
mkdir -p ~/.local/share
git clone https://github.com/jaredseavyhodge/hyprbole ~/.local/share/hyprbole
cd ~/.local/share/hyprbole
```

## Run Hyprbole Install

Run the installer as your normal user:

```bash
./install.sh
```

Do not run the installer with `sudo`. It will ask for sudo when system changes are needed.

The installer is interactive. It asks for your Git full name and email, then uses sudo for package installation and system setup.

## Before Running Install

- Boot into the fresh Arch system as your normal user.
- Confirm you are in a terminal TTY with no desktop profile already installed.
- Make sure networking is working.
- Make sure `sudo` works for your user.
- Make sure the Hyprbole checkout is a Git checkout, not a copied directory without `.git`.

## What Install Does

- Installs official Arch packages.
- Installs `yay`.
- Installs the curated AUR package set.
- Configures Git identity from prompts.
- Deploys missing user config from `config/`.
- Generates current theme/runtime state.
- Configures SDDM, services, browser launchers, and Keyring.
- Runs verification at the end.

## Rerunning Install

`install.sh` is designed to be safe to run more than once. Rerunning install is a supported repair path for Hyprbole-owned defaults.

Repeat runs should not overwrite existing user-owned config copied from `config/`. They may intentionally reapply Hyprbole-owned state such as browser launchers, service enables, generated theme files, SDDM assets, browser policy links, and Keyring setup.

## After Install

Run:

```bash
hb doctor
hb verify
```

If supported checks fail, run:

```bash
hb doctor --fix
```

Then reboot:

```bash
sudo reboot
```

After reboot, SDDM should appear. Log in with your user to start the Hyprbole desktop.

## Logs

Install writes logs under:

```text
~/.local/state/hyprbole/install-logs
```

The latest files are symlinked as:

```text
~/.local/state/hyprbole/install-logs/latest.log
~/.local/state/hyprbole/install-logs/latest.diagnostics.txt
```

Use these when an install fails or when you need to compare a fresh install against `hb doctor --verbose`.
