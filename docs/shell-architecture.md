# Hyprbole Architecture

This document defines the experimental direction for a Hyprland-first desktop shell. This branch intentionally removed the legacy Bash/config/install/theme desktop-layer surface so the Rust shell can be developed without compatibility constraints.

The target is a lightweight desktop shell, not a full desktop environment and not a general GTK/Qt replacement. Hyprland remains the compositor. Hyprbole owns the shell surfaces, desktop state, settings, theme model, and typed control path around it.

## Reference Model

Noctalia v4 validates a useful product shape:

- service-first architecture
- major UI modules for bar, dock, launcher, settings, notifications, OSD, lock screen, and widgets
- reusable shell widgets
- compositor-specific backends behind a common compositor facade
- local IPC command targets instead of REST
- quick control center separate from deeper settings
- theme tokens and templates projected into external apps

Hyprbole should borrow that shape, but not the v4 implementation constraint of keeping core logic inside Quickshell/QML. Noctalia's v5 rewrite away from Qt reinforces the same concern: a long-lived, lightweight Wayland shell benefits from owning more of the event loop, state model, rendering path, and IPC boundary.

## Architecture Goal

Hyprbole should be split into four main pieces:

```text
hyprbole-core
  Rust library crate
  settings and state store
  Hyprland IPC adapter
  audio/network/bluetooth/power/system adapters
  notifications and shell state
  typed state and action contracts

hyprbole-daemon
  session-local service process
  owns live state and event subscriptions
  exposes typed local IPC

hyprbole-ui
  Rust Wayland shell process
  layer-shell and normal window surfaces
  declarative shell widgets
  bar, launcher, quick settings, settings, OSD, notifications

hbctl
  CLI client for typed shell IPC
  keybind/menu/debug integration point
```

The daemon is the desktop brain. The core crate owns reusable contracts and adapters. The UI process is a renderer and interaction layer. CLI/menu/keybind integrations call typed shell actions instead of reaching directly into implementation details.

## Core Daemon

The shell core should be a session-local Rust process. It should not be a system service unless a future feature has a concrete system-level need.

Responsibilities:

- detect and connect to the active Hyprland session
- maintain normalized workspaces, windows, monitors, current layout, current monitor modes, focused window, and compositor events
- expose typed actions for compositor operations such as focus workspace, focus window, close window, runtime layout/mode changes, launch command, and DPMS
- track shell settings and desired state
- persist settings in an atomic structured file store by default, with SQLite reserved for future query/history-heavy domains if needed
- integrate with PipeWire/WirePlumber for audio
- integrate with NetworkManager, BlueZ, UPower, logind, and power-profiles-daemon where available
- own notification history, do-not-disturb state, and notification actions if Hyprbole replaces a separate notification daemon
- expose app/default-role data from `.desktop`, MIME, and XDG settings
- publish state snapshots and event streams

The core must expose typed state and typed actions. It must not expose arbitrary shell command execution as the default UI API.

## IPC Model

REST should not be the primary shell API.

Use a Unix socket under `$XDG_RUNTIME_DIR/hyprbole/hyprbole.sock` for local typed IPC. Development-only debug transports may exist, but the product contract should be command/event oriented:

```text
state.get audio
state.get hyprland.workspaces
action.call audio.set-volume { target = "default-output", value = 0.42 }
action.call hyprland.focus-workspace { id = 3 }
events.subscribe ["audio", "hyprland", "theme"]
```

Initial messages may use JSON for inspection and speed of development. The current prototype exposes request/response JSON for state, actions, settings, keybinding inspection, recent event snapshots, and a JSON-lines event follow mode with bounded replay by event id. If a requested cursor is older than the retained event buffer, the daemon emits a `gap` event so clients know they missed records. Mutating actions also emit structured `action` events and update daemon `last_action` metadata so thin surfaces can show failures without parsing display strings. Event records expose helper classification for snapshot refreshes versus status refreshes so UI surfaces do not need to blindly reload for every record. The protocol can move to MessagePack, CBOR, or another compact typed format once contracts stabilize.

Security rules:

- local Unix socket only by default
- no arbitrary shell strings in core action APIs
- action schemas validate all inputs
- mutating actions are explicit and allowlisted
- future third-party UI/plugin access should be capability-scoped

## UI Runtime

Hyprbole should not adopt GTK, Qt, Tauri, or Quickshell as the core shell identity. Those can remain useful for experiments, but the target shell should use a Rust-native, shell-specific UI runtime.

Candidate building blocks for early prototypes:

These are examples, not commitments:

- `smithay-client-toolkit` for Wayland client and layer-shell integration
- `calloop` for event loop integration
- `taffy` for layout
- `cosmic-text` for text shaping and rendering
- `vello`, `femtovg`, or `wgpu` for rendering
- `accesskit` later for accessibility
- `serde` for widget descriptions, settings, and IPC data

The runtime should support two surface families:

- layer-shell surfaces for bar, dock, quick settings, OSD, notifications, lock/session overlays, and desktop widgets
- normal windows for settings, inspectors, and development tools

Do not build a universal app toolkit first. Build the shell widgets needed for desktop surfaces.

## Widget Model

Start with a small reusable widget set. The exact names and coverage should be proven by the first shell surfaces rather than treated as a stable public toolkit:

- `Panel`
- `Bar`
- `Button`
- `IconButton`
- `Toggle`
- `Slider`
- `TextInput`
- `SearchBox`
- `List`
- `Grid`
- `Card`
- `Popover`
- `Menu`
- `Notification`
- `WorkspaceStrip`
- `QuickToggle`
- `SettingsRow`

Widgets should bind to typed state paths and call typed actions. They should consume theme tokens directly rather than each surface maintaining its own stylesheet vocabulary.

External declarative UI files are desirable, but they should come after the first Rust component model proves the state/action/widget contracts. A simple RON/TOML/JSON widget description can be introduced later for user customization and plugins.

## Settings And Control Center

Keep two concepts separate:

- Control Center: quick status, toggles, and transient controls
- Settings: deeper configuration and persistence

Early control-center cards:

- audio output/input/volume/mute
- network and Bluetooth quick toggles
- power profile and battery
- brightness
- notifications and do-not-disturb
- theme/dark mode
- workspace/session actions

Early settings panels:

- appearance and theme tokens
- bar layout and widgets
- launcher behavior
- notifications
- audio preferences
- keyboard/input shortcuts that are shell-owned
- default apps and app roles
- autostart entries
- power and idle behavior

Each compositor/system setting must declare ownership. The current prototype uses three runtime modes:

- `respect_user_config`: observe and report state, but do not mutate runtime config
- `runtime_owned`: apply runtime IPC state and reconcile it after reload/reconnect
- `persisted_owned`: Hyprbole-owned persisted intent projected into runtime state

This mode is per subsystem, not global. Keybindings can be runtime-owned while monitors and appearance remain user-config-owned. The current implementation can reconcile runtime keybindings, has partial runtime action/status adapters for audio and brightness, and performs observe-only reconciliation for monitors and workspaces with structured severity/results. The stored modes for appearance, notifications, and power remain scaffolding for later slices.

Hyprbole-owned shell UI settings are separate from Hyprland ownership. The current prototype persists `ui.bar` settings in `$XDG_CONFIG_HOME/hyprbole/settings.json` for bar enablement, monitor selector, top/bottom edge, height, margin, padding, and widget visibility/order. These settings drive the native layer-shell bar directly and do not rewrite user Hyprland config. The bar currently applies edge, height, margin, padding, ordered system widgets, and tokenized colors from daemon snapshots. Monitor targeting resolves focused, primary, and named targets by matching daemon monitor snapshots to Wayland output names when available; unresolved targets fall back safely and are logged. The same settings file now stores `theme.mode`, and the daemon exposes effective built-in theme tokens in state snapshots for shell surfaces.

The preferred settings store is a small structured file written atomically with temp-file plus rename semantics. This keeps the shell lightweight, inspectable, and reliable without adding a service dependency. SQLite can be introduced later for notification history, launcher/app indexes, search ranking, or other query-heavy data. External Redis/KVP services are not part of the core settings path because they add operational dependency for little benefit in a session-local shell.

This prevents the GUI from pretending it can safely own every Hyprland or Linux desktop setting.

## Hyprland Integration

Hyprbole should integrate with Hyprland through IPC for live state and actions. It should not depend on rewriting arbitrary user Hyprland config files as the primary mechanism.

The first compositor backend can be Hyprland-only, but it should still be shaped as a facade so future backends or fallback protocols remain possible.

Normalized compositor state:

- monitors
- workspaces
- windows/toplevels
- active window
- focused monitor
- display scale and geometry
- compositor events

Typed compositor actions:

- focus workspace
- focus window
- close window
- launch command through a validated shell-owned path
- turn monitors on/off
- logout/session actions where Hyprland owns them

Persistent Hyprland config remains user-owned unless a future feature explicitly introduces a Hyprbole-managed profile. If a setting can only be persisted through config, label it as config-owned or runtime-reapplied rather than silently editing user files.

Runtime-owned compositor state should be reconciled by the daemon where the subsystem has an implemented reconciler:

- once on daemon startup
- after Hyprland socket2 reconnects
- after config reload events such as `configreloaded` / `config.reloaded`

The first implemented runtime-owned keybinding reconciliation registers Super mouse drag/resize binds through Hyprland IPC. If keybindings are changed to `respect_user_config`, the daemon releases those runtime binds and then skips future reconciliation.

## Theming

Theme tokens are the source of truth. The current prototype has a small built-in token contract with persisted `system`, `light`, and `dark` modes; `system` currently resolves to dark tokens until real OS-theme detection exists. Theme packs and external-app projection remain future work.

Token groups:

- colors
- typography
- spacing
- radii
- borders
- shadows
- animation timing
- density
- icon and cursor choices
- light/dark mode

The shell UI consumes tokens directly. Adapters may project tokens into GTK, Qt, terminal, browser, and compositor settings later, but template sprawl should not outrun the core shell.

## Plugin Strategy

Do not start with a large plugin system.

The first extension point should be internal registries:

- bar widgets
- control-center cards
- settings panels
- launcher providers

Only after state/action/widget contracts stabilize should Hyprbole expose third-party plugins. Plugin access must be capability-scoped and should not get arbitrary command execution by default.

## First Milestones

1. Build a visible `hyprbole-ui` prototype window to validate the visual direction and widget vocabulary. Done.
2. Define `hyprbole-core`, `hyprbole-daemon`, `hyprbole-ui`, and `hbctl` process boundaries.
3. Implement a Hyprland state/action backend in Rust.
4. Add typed Unix-socket IPC and `hbctl` for inspection and keybind/menu integration.
5. Persist shell settings in the Hyprbole settings store; the current prototype uses atomic `settings.json`, while SQLite remains a possible later store for query/history-heavy data.
6. Build one layer-shell bar surface with text, button, workspace strip, and quick toggle widgets.
7. Build a quick settings panel with audio and session actions.
8. Build a layer-shell OSD surface for daemon action feedback and audio/brightness summaries.
9. Add theme tokens consumed directly by the shell UI. Initial persisted mode and built-in daemon-provided tokens are implemented.
10. Add a settings window using the same widget set.
11. Consider replacing external bar/launcher workflows only after the shell is stable enough to own those roles.

## Current Prototype

The first visible prototype is `crates/hyprbole-ui` and can be launched through the Rust shell control entrypoint:

```bash
cargo run -p hbctl -- ui
```

The preview and control surfaces are intentionally normal test windows. The bar is a layer-shell smoke surface, quick settings is a single-instance layer-shell popup, and the OSD is a lightweight transient layer-shell action feedback surface. `hbctl quick --toggle` opens or requests dismissal of that popup using the runtime identity/dismiss files. The older egui quick settings window remains available with `hbctl quick --dev` for development comparisons. This keeps the current renderer useful for validating layout, visual language, interactive shell actions, and module shape quickly; the long-term runtime target remains a shell-specific Rust Wayland renderer.

The current prototype can refresh shell state, focus Hyprland workspaces, adjust output volume, toggle output mute, adjust brightness, toggle notification DND, switch runtime layout/monitor mode, toggle quick settings from the bar, show daemon action feedback in the OSD, edit persisted bar settings and widget order, edit persisted theme mode, and request session lock through daemon IPC. The bar renders daemon-backed workspace, active-window, audio, brightness, notification/DND, layout, clock, action, and daemon/reconcile status text according to persisted `ui.bar.widgets`, applies configured margin/padding, and bar/quick/OSD consume daemon-provided color tokens for core text/background colors. The OSD follows daemon events, refreshes snapshots/status on a background worker, auto-hides when idle, and renders recent action status plus current audio/brightness summaries from snapshots. Bar hit regions route visible workspace, status, and system interactions explicitly, and rapid scroll/toggle actions are lightly throttled client-side before daemon dispatch. Refreshes and actions run through background workers so surfaces remain responsive. Direct UI fallback paths are debug-only. The daemon handler path now has a mockable runtime seam for tests so state/settings/action/event behavior can be covered without a live Hyprland session. Notification history/DND state exists, but replacing a DBus notification daemon is future work.

## Non-Goals For The Experiment

- full distro installer replacement
- general-purpose GTK/Qt replacement
- Quickshell/QML shell runtime as the long-term foundation
- REST as the primary shell API
- arbitrary user Hyprland config rewriting
- large third-party plugin system before core contracts stabilize
- theme-template expansion before shell surfaces and tokens are stable
