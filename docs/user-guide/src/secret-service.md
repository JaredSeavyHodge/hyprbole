# Secret Service

Hyprbole uses GNOME Keyring and libsecret for app secret storage. This is what Chromium/Electron apps use when launched with `--password-store=gnome-libsecret`.

## What Hyprbole Configures

Install and `hyprbole doctor --fix` run the internal repair script:

```text
~/.local/share/hyprbole/bin/hyprbole-setup-secret-service
```

It does the following:

- Creates a passwordless `Default_keyring`.
- Writes `~/.local/share/keyrings/default` to select `Default_keyring`.
- Backs up an encrypted `login.keyring` if one exists.
- Removes SDDM `auth` and `password` GNOME Keyring PAM hooks.
- Keeps SDDM session hooks so the daemon starts on login.
- Restarts the user keyring daemon when possible.

## Why This Matters

An encrypted `login.keyring` can make Chromium-based apps hang or show keyring warnings outside GNOME. Hyprbole uses a passwordless default keyring so apps can store secrets without a hidden unlock prompt.

## Check It

```bash
hyprbole doctor
hyprbole verify
```

## Repair It

```bash
hyprbole doctor --fix
```

Then relaunch affected apps such as Brave or VS Code.

## Files

| Path | Purpose |
| --- | --- |
| `~/.local/share/keyrings/Default_keyring.keyring` | Hyprbole default keyring |
| `~/.local/share/keyrings/default` | Selects the default keyring |
| `/etc/pam.d/sddm` | SDDM password login PAM hooks |
| `/etc/pam.d/sddm-autologin` | SDDM autologin PAM hooks |
