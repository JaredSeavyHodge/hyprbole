# Browser

Hyprbole uses Brave Origin Nightly as the default browser.

Flags are written to `~/.config/brave-origin-nightly-flags.conf`, which the system launcher (`/usr/bin/brave-origin-nightly`) reads on every launch. This avoids a wrapper-specific desktop entry — the system `brave-origin-nightly.desktop` is used directly, so uninstalling Brave cleanly removes the entry.

## Required Flags

The Hyprbole refresh script always writes:

```text
--password-store=gnome-libsecret
--ozone-platform-hint=auto
```

`gnome-libsecret` makes Chromium use GNOME Keyring through Secret Service. `ozone-platform-hint=auto` chooses Wayland in Wayland sessions while remaining compatible with X11 sessions.

## Extra Flags

Add your own flags to `~/.config/brave-origin-nightly-flags.conf`. Use one per line. The refresh script preserves existing custom flags when it rewrites the file.

## Theme Policy

Hyprbole writes the current browser theme policy to:

```text
~/.config/hyprbole/current/browser-policy.json
```

The managed Brave policy is linked at:

```text
/etc/brave/policies/managed/color.json
```

To reapply it:

```bash
hb browser setup-theme-policy
```

If you do not want Hyprbole theme changes to update the browser policy, set this in `~/.config/hyprbole/settings.toml`:

```toml
skip_browser_theme_changes = true
```

## Fractional Scaling

Chromium fractional-scaling flags are not enabled globally because they are monitor, compositor, and Chromium-build dependent. They can improve odd scaling on some wide or high-DPI setups, but may also cause blurry text, odd hit testing, or UI sizing issues.

To experiment, add flags to `~/.config/brave-origin-nightly-flags.conf`:

```text
--force-device-scale-factor=1.25
--gtk-version=4
--enable-features=WaylandPerSurfaceScale,WaylandUiScale
```

Close all Brave windows and relaunch after changing flags.

## Repair

Run:

```bash
hyprbole-refresh-browser-launchers
```

or use:

```bash
hb doctor --fix
```

Then close every Brave window and launch again.
