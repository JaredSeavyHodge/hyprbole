# Install

Hyprbole runs after base Arch Linux installation. `archinstall` owns disk layout, users, locale, networking, and the base system. Hyprbole owns the desktop layer.

Run the installer as your normal user:

```bash
./install.sh
```

Do not run the installer with `sudo`. It will ask for sudo when system changes are needed.

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
