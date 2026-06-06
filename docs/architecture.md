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

`hyprbole-ui` is the visible prototype. It currently uses egui/eframe to render a normal preview window, daemon-backed control center, and dev launcher window; GTK4 plus gtk4-layer-shell for the default launcher experiment; and custom layer-shell surfaces for the bar, optional `--layer` launcher, quick settings popup, and OSD while we validate shell layout, widget vocabulary, and basic interactions.

`hbctl` is the Rust control entrypoint. It builds and launches the UI with:

```bash
cargo run -p hbctl -- ui
```

It launches the current layer-shell bar prototype with:

```bash
cargo run -p hbctl -- bar
cargo run -p hbctl -- bar --toggle
cargo run -p hbctl -- bar --restart
```

`hbctl bar` is single-instance guarded through a runtime `pid:start_time` identity file validated against `/proc`, with stale-file and pidfile-less prototype-process recovery. `--toggle` stops the current bar when present or opens it when absent; `--restart` replaces a running bar.

It launches the daemon-backed control center with:

```bash
cargo run -p hbctl -- control
```

It launches the app launcher with:

```bash
cargo run -p hbctl -- launcher
```

`hbctl launcher` opens the GTK4 plus gtk4-layer-shell launcher by default for CSS-level theming evaluation. `hbctl launcher --layer [--prompt TEXT] [--placeholder TEXT] [--lines N]` opens the compact custom layer-shell launcher backed by standard `.desktop` entries; it focuses the filter on open, hides app rows until typing starts, and supports `--toggle` and `--restart` for non-stdin lifecycle control. `hbctl launcher --layer --stdin --foreground [--prompt TEXT] [--placeholder TEXT] [--lines N]` provides a dmenu-style generic list and prints the selected line to stdout. `hbctl launcher --dev` preserves the older egui app-search window.

It launches compact daemon-backed quick settings with:

```bash
cargo run -p hbctl -- quick
cargo run -p hbctl -- quick --toggle
cargo run -p hbctl -- quick --restart
cargo run -p hbctl -- quick --dev
```

`hbctl quick` is layer-shell by default and currently covers audio, brightness, layout, monitor summary, and session lock controls. Quick settings is single-instance guarded through a runtime `pid:start_time` identity file validated against `/proc`, with stale-file and pidfile-less prototype-process recovery. `hbctl quick --toggle` opens quick settings if absent and requests dismissal of the running quick popup if present. `hbctl quick --restart` waits for the targeted popup identity to exit, then opens a fresh quick popup. `hbctl quick --dev` keeps the older guarded egui quick settings window available for development comparisons.

It launches daemon-backed OSD feedback with:

```bash
cargo run -p hbctl -- osd
cargo run -p hbctl -- osd --foreground
cargo run -p hbctl -- osd --toggle
cargo run -p hbctl -- osd --restart
```

`hbctl osd` is single-instance guarded through a runtime `pid:start_time` identity file validated against `/proc`, with stale-file and pidfile-less prototype-process recovery. `--toggle` stops the current OSD when present or opens it when absent; `--restart` replaces a running OSD.

It can also exercise the core Hyprland IPC path with:

```bash
cargo run -p hbctl -- bind-mouse-ipc
```

When the daemon is running, `hbctl bind-mouse-ipc` calls the daemon. Its direct core Hyprland IPC fallback is a prototype/debug path only; normal shell controls should use typed daemon IPC.

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
cargo run -p hbctl -- logs launcher --path
cargo run -p hbctl -- logs list
cargo run -p hbctl -- surfaces
cargo run -p hbctl -- surfaces --plain
cargo run -p hbctl -- surfaces bar
cargo run -p hbctl -- surfaces restart bar
cargo run -p hbctl -- surfaces stop quick
cargo run -p hbctl -- settings
cargo run -p hbctl -- settings set keybindings runtime
cargo run -p hbctl -- ui-settings
cargo run -p hbctl -- ui-settings set bar.edge bottom
cargo run -p hbctl -- ui-settings set bar.font_size 12
cargo run -p hbctl -- ui-settings set bar.radius 10
cargo run -p hbctl -- ui-settings set bar.opacity 96
cargo run -p hbctl -- ui-settings reset bar
cargo run -p hbctl -- ui-settings set osd.timeout_ms 2500
cargo run -p hbctl -- ui-settings set osd.font_size 14
cargo run -p hbctl -- ui-settings set osd.radius 18
cargo run -p hbctl -- ui-settings set osd.opacity 94
cargo run -p hbctl -- ui-settings reset osd
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

`hbctl surfaces` is a local runtime inspection helper. It prints JSON status for quick, bar, OSD, and the layer launcher, including running state, PID identity, pidfile path, pidfile health, log path, and log health. `pid_file_state` is one of `missing`, `ok`, `stale`, `invalid`, or `unreadable`, with explicit JSON booleans for stale, invalid, and unreadable pidfiles. `log_state` is one of `missing`, `unreadable`, `empty`, or `ok`, with JSON fields for existence, readability, and size. Pass `quick`, `bar`, `osd`, or `launcher` to filter to one surface; add `--plain` for a compact human-readable table with pidfile and log health columns. `hbctl surfaces start|stop|restart quick|bar|osd|launcher|all` delegates to the same guarded lifecycle paths as the individual surface commands. `hbctl surfaces doctor [quick|bar|osd|launcher|all] [--json|--commands] [--unhealthy-only] [--actionable-only] [--zero-ok] [--warnings-only|--errors-only]` is a read-only diagnostic summary with health, severity, issue labels, recommendations, structured safe follow-up commands, and embedded surface status; it exits non-zero when any selected surface is unhealthy unless `--zero-ok` is set. Doctor flags missing logs for running surfaces and unreadable or empty logs whenever present. `--unhealthy-only` filters healthy rows while preserving the total selected issue count; `--actionable-only` filters output to rows with a safe `recommended_command`; `--commands` prints only displayed safe recommended commands and is mutually exclusive with `--json`; `--zero-ok` preserves output but returns success even when selected surfaces are unhealthy; `--warnings-only` and `--errors-only` are mutually exclusive severity filters that preserve the total selected issue count. Doctor JSON includes pre-filter `issues`, `warnings`, `errors`, and `actionable`, plus post-filter `displayed`, `zero_ok`, `output_mode`, `active_filters`, `selected`, `healthy_count`, displayed `surface_names`, displayed `issue_labels`, `command_count`, `has_commands`, `severity_filter`, and displayed-row `recommended_commands`; plain output starts with the same summary counts and reports when filters match no rows. `hbctl surfaces check quick|bar|osd|launcher|all running|stopped|healthy|pidfile-ok|pidfile-missing|pidfile-stale|pidfile-invalid|pidfile-unreadable [--quiet] [--json]` performs an immediate read-only assertion for scripts; `healthy` means running surfaces have an `ok` pidfile and stopped surfaces have a `missing` pidfile, while `pidfile-*` checks assert the exact pidfile state. `hbctl surfaces wait quick|bar|osd|launcher|all running|stopped|healthy|pidfile-ok|pidfile-missing|pidfile-stale|pidfile-invalid|pidfile-unreadable [--timeout-ms N] [--interval-ms N] [--quiet] [--json]` waits for the same predicates without mutating surfaces; defaults are `--timeout-ms 3000` and `--interval-ms 50`. `--json` emits `target`, `expected`, `passed`, and matching `surfaces`; wait reports also include `elapsed_ms`. `hbctl surfaces clean quick|bar|osd|launcher|all [--dry-run]` removes parseable stale pidfiles only and leaves live, missing, invalid, or unreadable pidfiles alone. The layer launcher writes `launcher.pid` while running; egui and GTK launcher prototype windows remain outside `hbctl surfaces` lifecycle control.

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

The control center is daemon-only. It does not enable direct fallback behavior, and ownership/UI-setting changes flow through typed daemon settings requests. It shows structured reconcile severity and per-subsystem reconcile rows from daemon status, and includes panes for persisted bar and OSD surface settings.

Background launches route stdout/stderr to `$XDG_RUNTIME_DIR/hyprbole/<name>.log`; `hbctl logs` reads those runtime logs.

For surface doctor log diagnostics, missing logs are only treated as issues for running surfaces; unreadable or empty logs are reported whenever present.

Ownership capability metadata is reported by the daemon. The UI and CLI use it to distinguish implemented runtime reconciliation, partial runtime adapters, observe-only ownership, and reserved ownership modes.

Keybinding reconciliation can mutate runtime IPC state when owned by Hyprbole. Monitor and workspace reconciliation is currently observe-only; it reports ownership and status without rewriting user Hyprland config or compositor state.

Shell UI settings are Hyprbole-owned settings, not Hyprland ownership modes. The current `ui.bar` settings are persisted in the Hyprbole settings file and drive the native layer-shell bar's enablement, edge, height, margin, padding, bitmap font size, rounded section radius, opacity, monitor selector, and widget visibility/order. The current `ui.osd` settings drive OSD enablement, top/bottom edge, width, height, margin, visible timeout, bitmap font size, rounded card radius, and opacity. The bar applies margin/padding, style settings, ordered system widgets, and tokenized colors from daemon snapshots. The OSD applies its surface settings, style settings, and tokenized colors from daemon snapshots. Monitor targeting resolves focused, primary, and named targets by matching daemon monitor snapshots to Wayland output names when available; missing matches fall back safely and are logged by the bar runtime.

Theme settings are Hyprbole-owned. The current `theme.mode` setting supports `system`, `light`, and `dark`; `system` is currently dark-aligned until real OS-theme detection exists, and `hbctl theme reset` restores the default dark mode. The daemon exposes effective built-in tokens in shell snapshots, and the bar, quick, and OSD surfaces consume those color tokens for core background/text colors. Theme packs and external application projection remain future work.

The OSD surface is daemon-backed, event-driven, and configured through persisted `ui.osd` settings. It refreshes daemon state on a background worker, defers action display until daemon `ui.osd` settings are loaded, shows transiently for action events when enabled, redraws transparent when idle, and renders recent action status plus current audio/brightness summaries from daemon snapshots rather than reading system state directly. Current OSD fields are `enabled`, `edge`, `width`, `height`, `margin`, `timeout_ms`, `font_size`, `radius`, and `opacity`.

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
