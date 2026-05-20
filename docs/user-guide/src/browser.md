# Browser

Hyprbole uses Brave Origin Nightly as the default browser.

The package provides `/usr/bin/brave-origin-nightly`, but Hyprbole owns normal browser startup through:

```text
~/.local/share/hyprbole/bin/hyprbole-launch-brave-origin-nightly
```

The generated desktop entry is:

```text
~/.local/share/applications/hyprbole-brave-origin-nightly.desktop
```

Normal Hyprbole browser shortcuts, menus, and default-browser launches should go through this desktop entry or wrapper instead of the package launcher directly.

## Required Flags

The Hyprbole launcher always passes:

```text
--password-store=gnome-libsecret
--ozone-platform-hint=auto
```

`gnome-libsecret` makes Chromium use GNOME Keyring through Secret Service. `ozone-platform-hint=auto` chooses Wayland in Wayland sessions while remaining compatible with X11 sessions.

## Extra Flags

Keep the package wrapper fallback file single-flag safe:

```text
~/.config/brave-origin-nightly-flags.conf
```

Put Hyprbole-specific extra Brave flags here:

```text
~/.config/hyprbole/brave-origin-nightly-flags.conf
```

Use one flag per line. Fully close Brave before testing changes because Chromium keeps one browser process alive for all windows.

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

To experiment, add flags to `~/.config/hyprbole/brave-origin-nightly-flags.conf`:

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
hb verify
```

or use:

```bash
hb doctor --fix
```

Then close every Brave window and launch it again from the Hyprbole launcher, desktop entry, or browser keybind.
