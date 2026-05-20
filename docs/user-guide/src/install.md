# Install

Hyprbole runs after base Arch Linux installation. `archinstall` owns disk layout, users, locale, networking, and the base system. Hyprbole owns the desktop layer.

Run the installer as your normal user:

```bash
./install.sh
```

Do not run the installer with `sudo`. It will ask for sudo when system changes are needed.

The installer is interactive. It asks for your Git full name and email, then uses sudo for package installation and system setup.

## Before Running Install

- Boot into the fresh Arch system as your normal user.
- Make sure networking is working.
- Make sure `sudo` works for your user.
- Make sure the Hyprbole checkout is a Git checkout, not a copied directory without `.git`.

If sudo rejects the correct password several times, Arch's default PAM lockout can temporarily block authentication. Wait for the lockout to expire or reset it from a root shell before rerunning install.

## What Install Does

- Installs official Arch packages.
- Installs `yay`.
- Installs the curated AUR package set.
- Configures Git identity from prompts.
- Deploys missing user config from `config/`.
- Generates current theme/runtime state.
- Configures SDDM, services, browser launchers, and Secret Service.
- Runs verification at the end.

## Rerunning Install

`install.sh` is designed to be safe to run more than once. Rerunning install is a supported repair path for Hyprbole-owned defaults.

Repeat runs should not overwrite existing user-owned config copied from `config/`. They may intentionally reapply Hyprbole-owned state such as browser launchers, service enables, generated theme files, SDDM assets, browser policy links, and Secret Service setup.

## After Install

Run:

```bash
hyprbole doctor
hyprbole verify
```

If supported checks fail, run:

```bash
hyprbole doctor --fix
```

Then log out and choose the `Hyprland (uwsm)` session in SDDM if it is not already selected.

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

Use these when an install fails or when you need to compare a fresh install against `hyprbole doctor --verbose`.
