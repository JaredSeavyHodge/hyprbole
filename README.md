# Hyprbole Experiment

This branch is a fresh Rust-first experiment for a Hyprland-native desktop shell. It intentionally drops the previous Bash installer, config templates, themes, and desktop-layer scripts so the new architecture can evolve without legacy pollution.

Hyprland remains the compositor. Hyprbole is the shell layer around it: state, actions, panels, settings, control center, notifications, widgets, and eventually typed local IPC.

## Current Scope

- Rust workspace only
- Hyprland IPC helper code in `crates/hyprbole-core`
- session-local daemon prototype in `crates/hyprbole-daemon`
- visible shell UI prototype in `crates/hyprbole-ui`
- Rust launcher/control entrypoint in `crates/hbctl`
- architecture docs in `docs/`
- opencode reviewer agents scoped to the shell experiment

## Repository Layout

- `crates/hyprbole-core/`: typed state/action and system adapter library
- `crates/hyprbole-daemon/`: session-local Hyprbole daemon prototype
- `crates/hyprbole-ui/`: visible shell UI prototype
- `crates/hbctl/`: Rust command/control entrypoint
- `docs/shell-architecture.md`: target shell architecture
- `docs/architecture.md`: current branch architecture summary
- `docs/implementation-plan.md`: near-term milestones
- `.opencode/agent/`: targeted review agents for shell runtime and docs

## Run The Prototype

Start the daemon in the foreground:

```bash
cargo run -p hbctl -- daemon --foreground
```

Check the daemon from another terminal:

```bash
cargo run -p hbctl -- ping
```

Inspect daemon state and shell ownership settings:

```bash
cargo run -p hbctl -- status
cargo run -p hbctl -- state
cargo run -p hbctl -- binds
cargo run -p hbctl -- events
cargo run -p hbctl -- events --after 12
cargo run -p hbctl -- events --follow
cargo run -p hbctl -- logs daemon
cargo run -p hbctl -- logs quick --path
cargo run -p hbctl -- logs list
cargo run -p hbctl -- ownership capabilities
cargo run -p hbctl -- settings
cargo run -p hbctl -- ui-settings
cargo run -p hbctl -- theme
cargo run -p hbctl -- notifications
cargo run -p hbctl -- ownership
```

Launch or control the layer-shell bar:

```bash
cargo run -p hbctl -- bar
cargo run -p hbctl -- bar --toggle
cargo run -p hbctl -- bar --restart
```

Change whether Hyprbole controls runtime keybindings or respects user config:

```bash
cargo run -p hbctl -- settings set keybindings runtime
cargo run -p hbctl -- settings set keybindings respect
cargo run -p hbctl -- ownership monitors respect
cargo run -p hbctl -- ownership workspaces respect
cargo run -p hbctl -- reconcile monitors
cargo run -p hbctl -- reconcile workspaces
cargo run -p hbctl -- ui-settings set bar.edge bottom
cargo run -p hbctl -- ui-settings set bar.height 44
cargo run -p hbctl -- ui-settings set bar.margin 4
cargo run -p hbctl -- ui-settings set bar.padding 16
cargo run -p hbctl -- ui-settings set bar.font_size 12
cargo run -p hbctl -- ui-settings set bar.radius 10
cargo run -p hbctl -- ui-settings set bar.opacity 96
cargo run -p hbctl -- ui-settings set bar.monitor focused
cargo run -p hbctl -- ui-settings set bar.widgets workspaces,focused_window,daemon_status,audio,brightness,layout,quick_toggle
cargo run -p hbctl -- ui-settings reset bar
cargo run -p hbctl -- ui-settings set osd.edge bottom
cargo run -p hbctl -- ui-settings set osd.timeout_ms 2500
cargo run -p hbctl -- ui-settings set osd.font_size 14
cargo run -p hbctl -- ui-settings set osd.radius 18
cargo run -p hbctl -- ui-settings set osd.opacity 94
cargo run -p hbctl -- ui-settings reset osd
cargo run -p hbctl -- theme mode dark
cargo run -p hbctl -- theme mode light
cargo run -p hbctl -- notifications dnd toggle
cargo run -p hbctl -- notifications clear
```

When runtime keybindings are enabled, Hyprbole reconciles Super mouse window drag/resize binds and `Super+Alt+Space` for `hbctl launcher`.

```bash
cargo run -p hbctl -- ui
```

Open the daemon-backed control center:

```bash
cargo run -p hbctl -- control
```

Launch the daemon-backed OSD surface:

```bash
cargo run -p hbctl -- osd
cargo run -p hbctl -- osd --foreground
cargo run -p hbctl -- osd --toggle
cargo run -p hbctl -- osd --restart
```

For foreground logs:

```bash
cargo run -p hbctl -- ui --foreground
```

The UI is daemon-first. Direct `hyprctl`/`wpctl`/`loginctl` fallback reads and actions are disabled by default and can be enabled only for debugging with:

```bash
cargo run -p hbctl -- ui --debug-direct-fallback
```

The UI includes a normal preview window, a daemon-backed control center, native layer-shell quick settings/bar/OSD surfaces, a custom `--layer` launcher surface, and a GTK4 layer-shell launcher experiment as the default `hbctl launcher` path. `hbctl control` opens the control center for ownership settings, persisted bar settings, notification DND status, current layout and monitor-mode state, ownership capability metadata, daemon PID/uptime, socket and settings paths, event-buffer state, recent events, structured reconcile severity, last save/action status, and event-stream status. `hbctl quick` opens common daemon-backed controls for audio, brightness, layout, monitor summary, and session lock; repeated launches are guarded by a runtime identity file so bar clicks do not spawn unbounded surfaces. `hbctl quick --toggle` opens quick settings if absent and requests dismissal of the running quick popup if present; `hbctl quick --restart` waits for the targeted popup identity to exit, then opens a fresh quick popup. Use `hbctl quick --dev` for the older egui development window. `hbctl status` prints the same daemon health snapshot as JSON, including protocol/build info, recent events, and structured last-action metadata.

Current surface targets are `quick`, `bar`, `osd`, `launcher`, and `all`.

For development sessions, `hbctl dev restart-all` restarts the daemon if it is already running, starts it if it is not, reconciles runtime-owned state, then restarts all managed layer surfaces (`quick`, `bar`, `osd`, and the custom layer launcher). It does not manage the default GTK launcher window.

`hbctl surfaces [quick|bar|osd|launcher]` prints JSON status for the quick, bar, OSD, and layer launcher surfaces, including running state, PID identity, pidfile path, pidfile health (`pid_file_state`, `pid_file_exists`, `stale_pid_file`, `invalid_pid_file`, `unreadable_pid_file`), log path, and log health (`log_state`, `log_exists`, `log_readable`, `log_size_bytes`); add `--plain` for a compact table with pidfile and log health columns. `pid_file_state` is one of `missing`, `ok`, `stale`, `invalid`, or `unreadable`; `log_state` is one of `missing`, `unreadable`, `empty`, or `ok`. `hbctl surfaces start|stop|restart quick|bar|osd|launcher|all` provides one lifecycle entrypoint that delegates to the same guarded launch/stop paths as the individual commands. `hbctl surfaces doctor [quick|bar|osd|launcher|all] [--json|--commands] [--unhealthy-only] [--actionable-only] [--zero-ok] [--warnings-only|--errors-only]` is a read-only diagnostic summary that reports health, severity, issue labels, recommended next actions, and structured `recommended_command` values when a safe follow-up command exists; it exits non-zero when any selected surface is unhealthy unless `--zero-ok` is set. Doctor also flags missing logs for running surfaces and unreadable or empty logs whenever present. `--unhealthy-only` filters healthy rows from plain and JSON output while preserving the total selected issue count; `--actionable-only` filters output to rows with a safe `recommended_command`; `--commands` prints only displayed safe recommended commands and is mutually exclusive with `--json`; `--zero-ok` preserves output but returns success even when selected surfaces are unhealthy; `--warnings-only` and `--errors-only` are mutually exclusive severity filters that preserve the total selected issue count. Doctor JSON includes pre-filter `issues`, `warnings`, `errors`, and `actionable`, plus post-filter `displayed`, `zero_ok`, `output_mode`, `active_filters`, `selected`, `healthy_count`, displayed `surface_names`, displayed `issue_labels`, `command_count`, `has_commands`, `severity_filter`, and displayed-row `recommended_commands`; plain output starts with the same summary counts and reports when filters match no rows. `hbctl surfaces check quick|bar|osd|launcher|all running|stopped|healthy|pidfile-ok|pidfile-missing|pidfile-stale|pidfile-invalid|pidfile-unreadable [--quiet] [--json]` performs an immediate read-only assertion for scripts. `hbctl surfaces wait quick|bar|osd|launcher|all running|stopped|healthy|pidfile-ok|pidfile-missing|pidfile-stale|pidfile-invalid|pidfile-unreadable [--timeout-ms N] [--interval-ms N] [--quiet] [--json]` waits for the same predicates without mutating surfaces; defaults are `--timeout-ms 3000` and `--interval-ms 50`. `healthy` means running surfaces have an `ok` pidfile and stopped surfaces have a `missing` pidfile, while the `pidfile-*` checks assert the exact pidfile state. Check and wait JSON reports emit `target`, `expected`, `passed`, and matching `surfaces`; wait reports also include `elapsed_ms`. `hbctl surfaces clean quick|bar|osd|all` removes parseable stale surface pidfiles only; live, missing, invalid, and unreadable pidfiles are left untouched. Add `--dry-run` to preview stale cleanup without removing files.

The layer launcher writes `launcher.pid` while running; egui and GTK launcher prototype windows remain outside `hbctl surfaces` lifecycle control.

Background launches write logs under `$XDG_RUNTIME_DIR/hyprbole/`; inspect them with `hbctl logs daemon`, `hbctl logs ui`, `hbctl logs control`, `hbctl logs quick`, `hbctl logs bar`, `hbctl logs osd`, or `hbctl logs launcher`. Use `hbctl logs list`, `hbctl logs --path`, or `hbctl logs --follow` for log paths and following. `hbctl ui-settings` prints Hyprbole-owned shell UI settings; `hbctl ui-settings set bar.<field> <value>` and `hbctl ui-settings set osd.<field> <value>` update validated persisted surface settings through the daemon, while `hbctl ui-settings reset bar|osd` restores defaults. `hbctl notifications` prints daemon-owned notification/DND state; `hbctl notifications dnd on|off|toggle` persists DND, `hbctl notifications clear` clears bounded in-memory history, and `hbctl notifications push <summary> [body]` is a prototype/manual injection path.

`hbctl bar` launches the current `wlr-layer-shell` bar prototype, backed by daemon snapshots and the daemon event stream. Bar launches are single-instance guarded with `bar.pid`; use `hbctl bar --toggle` to stop/open and `hbctl bar --restart` to replace a running bar. It renders compact themed bitmap workspace, active-window, audio/brightness summaries, notifications, current layout, clock, daemon/action status, and daemon/reconcile severity text according to persisted `ui.bar.widgets`. The bar applies persisted height, margin, padding, bitmap font size, rounded section radius, opacity, widget order, and theme colors at runtime. The bar routes explicit hit regions only for visible widgets: workspaces focus/scroll workspaces, the system region toggles mute and adjusts brightness by scroll, and the status region toggles quick settings. Bar scroll/mute/quick actions are lightly throttled to avoid daemon action spam. `hbctl layer-spike` remains an alias for the same prototype while it is still a low-level rendering spike.

`hbctl osd` opens a single-instance lightweight layer-shell OSD that follows daemon events, refreshes daemon snapshots on a background worker, shows transiently for daemon action events, and renders recent action status plus audio/brightness summaries. Use `hbctl osd --toggle` to stop/open it and `hbctl osd --restart` to replace the running OSD. The OSD consumes persisted `ui.osd` settings for enablement, top/bottom edge, width, height, margin, visible timeout, bitmap font size, rounded card radius, and opacity. The control center includes an OSD pane for the same settings. `hbctl launcher` opens the GTK4 plus gtk4-layer-shell launcher by default for CSS-level theming evaluation. `hbctl launcher --layer [--prompt TEXT] [--placeholder TEXT] [--lines N]` opens the compact custom layer-shell launcher with focused search, hidden app rows until typing starts, and `.desktop` app launching; use `--toggle` or `--restart` for non-stdin lifecycle control. `hbctl launcher --layer --stdin --foreground [--prompt TEXT] [--placeholder TEXT] [--lines N]` reads newline-separated entries from stdin and prints the selected line. `hbctl launcher --dev` opens the older egui launcher window. `hbctl theme` prints Hyprbole-owned theme mode and effective built-in tokens. `hbctl theme mode system|light|dark` persists the mode through daemon IPC; `system` currently resolves to dark tokens until real OS-theme detection exists. `hbctl theme reset` restores the default dark mode. The control center also includes an Appearance pane for theme mode, launcher context, and native bar/quick/OSD surfaces read color tokens from daemon snapshots.

For doctor log diagnostics, missing logs are only treated as issues for running surfaces; unreadable or empty logs are reported whenever present.

OSD action display is deferred until daemon `ui.osd` settings have loaded, so first-launch events do not ignore `osd.enabled=false` or custom edge/size/timeout values. Quick/bar/OSD instance files store `pid:start_time` identities and are validated against `/proc`; stale files are recovered and pidfile-less old prototype processes are detected as a migration guard. Current OSD fields are `osd.enabled`, `osd.edge`, `osd.width`, `osd.height`, `osd.margin`, `osd.timeout_ms`, `osd.font_size`, `osd.radius`, and `osd.opacity`.

## Current UI Behavior

The prototype window can:

- show Hyprland workspace/window/monitor snapshots from the Hyprbole daemon
- show current Hyprland layout and current monitor modes
- apply runtime-only Hyprland layout and monitor-mode changes through the daemon
- focus Hyprland workspaces
- show output volume from `wpctl`
- show brightness and reconcile status from daemon snapshots when available
- adjust output volume and toggle output mute
- open a compact quick settings window for common controls
- open a native OSD surface for daemon action feedback
- request a session lock through `loginctl lock-session`
- refresh state through background workers without freezing the UI
- open a daemon-only control center for ownership settings, appearance, and daemon/event status
- edit persisted bar monitor target, edge, height, margin, padding, enablement, widget visibility, and widget order through the control center
- edit persisted OSD enablement, edge, size, margin, and timeout through the control center
- inspect and change persisted theme mode through the control center Appearance pane
- inspect notification DND/unread state and toggle DND through daemon IPC

Direct `hyprctl`, `wpctl`, and `loginctl` fallback calls are temporary prototype paths. The normal control path is `hyprbole-ui`/`hbctl` to the daemon over typed JSON IPC, with runtime actions implemented behind `hyprbole-core` adapters.

## Ownership Model

Hyprbole observes desktop state broadly, but only mutates subsystems that are shell-owned. Ownership is tracked per subsystem in `$XDG_CONFIG_HOME/hyprbole/settings.json`, falling back to `$HOME/.config/hyprbole/settings.json` when `XDG_CONFIG_HOME` is unset. `hbctl control` and `hbctl ownership capabilities` can inspect ownership capability metadata through daemon IPC. The current prototype reconciles runtime keybindings. Audio and brightness have partial runtime action/status adapters. Monitor and workspace reconciliation is observe-only and reports current ownership without changing user config. The control center and CLI reject runtime/persisted ownership choices for subsystems without runtime support.

Hyprbole-owned shell UI preferences live in the same settings file under `ui`. The current `ui.bar` settings are persisted Hyprbole settings, not projected Hyprland config. They cover bar enablement, monitor selector, top/bottom edge, height, margin, padding, bitmap font size, rounded section radius, opacity, and widget visibility/order. The bar applies edge, height, margin, padding, font size, radius, opacity, widget visibility/order, and theme colors at runtime from daemon snapshots. Monitor targeting now resolves `focused`, `primary`, and named monitors against daemon monitor snapshots and Wayland output names when available; if no match exists, the bar falls back safely and logs the fallback in `hbctl logs bar`. The current `ui.osd` settings cover OSD enablement, edge, width, height, margin, visible timeout, bitmap font size, radius, and opacity; the OSD applies them from daemon snapshots without rewriting Hyprland config.

Notification preferences also live in the settings file. The current prototype owns DND state and bounded in-memory notification history; replacing a real DBus notification daemon is future work.

Theme preferences live in the settings file under `theme`. The current prototype supports `system`, `light`, and `dark` modes, with `system` currently dark-aligned, and exposes built-in effective tokens to shell surfaces through daemon snapshots. Theme packs and projection into external applications are future work.

The current settings store is a lightweight structured file written atomically. This keeps preferences inspectable and reliable without adding a Redis/KVP service dependency. SQLite remains a possible later fit for query-heavy data such as notification history or launcher/app indexes, not the default shell settings path.

Initial modes are:

- `respect_user_config`: observe and report state without runtime reconciliation
- `runtime_owned`: apply runtime IPC state and re-apply after reload/reconnect where implemented
- `persisted_owned`: reserved for Hyprbole-owned persisted settings projected into runtime state where implemented

Keybindings default to `runtime_owned` so the daemon can register Super mouse drag/resize binds on startup and after Hyprland config reload events. Set keybindings to `respect` if user Hyprland config should be the only source of keybind changes. Use `hbctl binds` to compare expected Hyprbole runtime binds against active Hyprland binds. Use `hbctl events --follow` for a JSON-lines stream of daemon events, and `--after <id>` to replay events after a known cursor. If the daemon's bounded event buffer has rotated past a cursor, it emits a `gap` event. Mutating daemon actions also emit structured `action` events and update `last_action` in daemon status while preserving the older `last_action_status` string for simple clients. UI surfaces now use event domain helpers to decide whether a daemon event should refresh shell snapshots, status, or both.

## Direction

The target split is:

- `hyprbole-core`: Rust library for state, settings, Hyprland IPC, system adapters, and typed local IPC contracts
- `hyprbole-daemon`: session-local Hyprbole service process
- `hyprbole-ui`: Rust shell UI process for layer-shell and normal-window surfaces
- `hbctl`: CLI/debug/keybind client for typed shell actions

The product API should be typed local IPC over a Unix socket, not REST. Quickshell/QML, GTK, Qt, Tauri, and egui are not the long-term shell identity.
