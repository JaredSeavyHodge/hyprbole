# Architecture

This branch is a clean Hyprbole experiment. It no longer contains the previous post-install desktop layer, Bash command tree, copied user configs, installer, or shipped theme packs.

## Current Components

```text
Cargo workspace
  crates/hyprbole-core
  crates/hyprbole-daemon
  crates/hyprbole-ui
  crates/hbctl
```

`hyprbole-core` owns reusable shell contracts and adapters. It currently contains typed daemon protocol models, shell ownership settings, persisted shell UI/bar settings, typed theme mode/tokens, typed notification/DND state, audio/brightness helpers, the Hyprland state snapshot adapter, and the direct Hyprland IPC socket client used for runtime bind, layout, and monitor-mode actions.

`hyprbole-daemon` is the first session-local service prototype. It listens on `$XDG_RUNTIME_DIR/hyprbole/hyprbole.sock` and handles typed JSON requests for ping, daemon health status, cached state snapshots, keybinding inspection, recent event snapshots, settings, UI settings, theme mode/tokens, notification/DND state, ownership changes, structured reconciliation, actions, and shutdown. Daemon status includes protocol/build info, recent events, and structured last-action metadata. Mutating actions emit structured `action` events and keep the older `last_action_status` string for simple clients. Event helpers classify whether a record should refresh shell snapshots, daemon status, or both. It also keeps legacy line-protocol compatibility for the narrow `ping`, `hyprland.bind_mouse`, and `shutdown` test commands.

`hyprbole-ui` is the visible prototype. It currently uses egui/eframe to render a normal preview window and daemon-backed control center, plus layer-shell surfaces for the bar, quick settings popup, and OSD while we validate shell layout, widget vocabulary, and basic interactions.

`hbctl` is the Rust control entrypoint. It builds and launches the UI with:

```bash
cargo run -p hbctl -- ui
```

It launches the current layer-shell bar prototype with:

```bash
cargo run -p hbctl -- bar
```

It launches the daemon-backed control center with:

```bash
cargo run -p hbctl -- control
```

It launches compact daemon-backed quick settings with:

```bash
cargo run -p hbctl -- quick
cargo run -p hbctl -- quick --toggle
cargo run -p hbctl -- quick --dev
```

`hbctl quick` is layer-shell by default. `hbctl quick --toggle` opens quick settings if absent and requests dismissal of the running quick popup if present. `hbctl quick --dev` keeps the older guarded egui quick settings window available for development comparisons.

It launches daemon-backed OSD feedback with:

```bash
cargo run -p hbctl -- osd
```

It can also exercise the core Hyprland IPC path with:

```bash
cargo run -p hbctl -- bind-mouse-ipc
```

When the daemon is running, `hbctl bind-mouse-ipc` calls the daemon. If the daemon is unavailable, it temporarily falls back to the direct core Hyprland IPC helper.

It can inspect and change daemon settings with:

```bash
cargo run -p hbctl -- state
cargo run -p hbctl -- status
cargo run -p hbctl -- binds
cargo run -p hbctl -- events
cargo run -p hbctl -- events --after 12
cargo run -p hbctl -- events --follow
cargo run -p hbctl -- logs daemon
cargo run -p hbctl -- logs quick --path
cargo run -p hbctl -- logs bar --follow
cargo run -p hbctl -- logs osd --path
cargo run -p hbctl -- logs list
cargo run -p hbctl -- settings
cargo run -p hbctl -- settings set keybindings runtime
cargo run -p hbctl -- ui-settings
cargo run -p hbctl -- ui-settings set bar.edge bottom
cargo run -p hbctl -- ui-settings reset bar
cargo run -p hbctl -- theme
cargo run -p hbctl -- theme mode light
cargo run -p hbctl -- theme reset
cargo run -p hbctl -- notifications
cargo run -p hbctl -- notifications dnd toggle
cargo run -p hbctl -- ownership capabilities
cargo run -p hbctl -- ownership monitors respect
cargo run -p hbctl -- reconcile
cargo run -p hbctl -- reconcile monitors
cargo run -p hbctl -- reconcile workspaces
```

## Target Components

The target architecture is:

- `hyprbole-core`: Rust library for typed state, actions, Hyprland IPC, system adapters, notifications, and typed local IPC contracts
- `hyprbole-daemon`: session-local service process that owns runtime state and action handling
- `hyprbole-ui`: shell UI process for layer-shell surfaces and normal settings windows
- `hbctl`: CLI/keybind/debug client for shell actions

## Runtime Model

The shell should be event-driven and compositor-first.

- Hyprland IPC provides compositor state and actions.
- PipeWire/WirePlumber owns audio state and actions.
- NetworkManager, BlueZ, UPower, logind, and power-profiles-daemon own their respective domains.
- Hyprbole normalizes those domains into typed state, typed actions, and events.

The UI must not become the system integration layer. Direct snapshot reads and action fallbacks inside `hyprbole-ui` are temporary prototype shortcuts gated behind `--debug-direct-fallback`; normal UI operation should use `hyprbole-core` and `hyprbole-daemon`.

The control center is daemon-only. It does not enable direct fallback behavior, and ownership changes flow through typed daemon settings requests. It shows structured reconcile severity and per-subsystem reconcile rows from daemon status.

Background launches route stdout/stderr to `$XDG_RUNTIME_DIR/hyprbole/<name>.log`; `hbctl logs` reads those runtime logs.

Ownership capability metadata is reported by the daemon. The UI and CLI use it to distinguish implemented runtime reconciliation, partial runtime adapters, observe-only ownership, and reserved ownership modes.

Keybinding reconciliation can mutate runtime IPC state when owned by Hyprbole. Monitor and workspace reconciliation is currently observe-only; it reports ownership and status without rewriting user Hyprland config or compositor state.

Shell UI settings are Hyprbole-owned settings, not Hyprland ownership modes. The current `ui.bar` settings are persisted in the Hyprbole settings file and drive the native layer-shell bar's enablement, edge, height, margin, padding, monitor selector, and widget visibility/order. The bar applies margin/padding, ordered system widgets, and tokenized colors from daemon snapshots. Monitor targeting resolves focused, primary, and named targets by matching daemon monitor snapshots to Wayland output names when available; missing matches fall back safely and are logged by the bar runtime.

Theme settings are Hyprbole-owned. The current `theme.mode` setting supports `system`, `light`, and `dark`; `system` is currently dark-aligned until real OS-theme detection exists, and `hbctl theme reset` restores the default dark mode. The daemon exposes effective built-in tokens in shell snapshots, and the bar, quick, and OSD surfaces consume those color tokens for core background/text colors. Theme packs and external application projection remain future work.

The OSD surface is daemon-backed and event-driven. It refreshes daemon state on a background worker, shows transiently for action events, redraws transparent when idle, and renders recent action status plus current audio/brightness summaries from daemon snapshots rather than reading system state directly.

Notification settings are also Hyprbole-owned. The daemon currently owns persisted DND state plus bounded in-memory notification history and manual/debug notification injection; full DBus notification server replacement remains future work.

The settings store is intentionally lightweight: `FileSettingsStore` persists a structured file with atomic temp-file plus rename semantics. This is the default for shell preferences and persisted intent. SQLite is reserved for future query/history-heavy domains, and external Redis/KVP services are not part of the core settings architecture.

Daemon request handling has a narrow runtime seam for tests. Production uses the real Hyprland/settings/action adapters; tests can inject a mock runtime for state refresh, action dispatch, keybinding inspection, and settings saves so handler-level IPC behavior can be validated without a live compositor or mutating the real settings file.

## IPC Model

The product shell API should be a local typed protocol over a Unix socket under `$XDG_RUNTIME_DIR/hyprbole/hyprbole.sock`.

REST is not the primary architecture. Any HTTP/debug transport must be explicitly marked as development-only.

## UI Model

The long-term UI runtime should be Rust-native and shell-specific. Candidate low-level pieces are listed in `docs/shell-architecture.md`, but they are examples, not commitments.

The current egui UI is intentionally temporary. It is useful because it produces a visible test window quickly, but it is not the final shell renderer.

## Non-Goals

- Bash route compatibility
- install/bootstrap tooling
- theme-template expansion
- general-purpose app toolkit
- large plugin system
- arbitrary Hyprland config rewriting
