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
cargo run -p hbctl -- ui-settings set bar.monitor focused
cargo run -p hbctl -- ui-settings set bar.widgets workspaces,focused_window,daemon_status,audio,brightness,layout,quick_toggle
cargo run -p hbctl -- ui-settings reset bar
cargo run -p hbctl -- theme mode dark
cargo run -p hbctl -- theme mode light
cargo run -p hbctl -- notifications dnd toggle
cargo run -p hbctl -- notifications clear
```

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
```

For foreground logs:

```bash
cargo run -p hbctl -- ui --foreground
```

The UI is daemon-first. Direct `hyprctl`/`wpctl`/`loginctl` fallback reads and actions are disabled by default and can be enabled only for debugging with:

```bash
cargo run -p hbctl -- ui --debug-direct-fallback
```

The UI includes a normal preview window, a daemon-backed control center, and native layer-shell quick settings, bar, and OSD surfaces. `hbctl control` opens the control center for ownership settings, persisted bar settings, notification DND status, current layout and monitor-mode state, ownership capability metadata, daemon PID/uptime, socket and settings paths, event-buffer state, recent events, structured reconcile severity, last save/action status, and event-stream status. `hbctl quick` opens common daemon-backed controls for audio, brightness, notification DND, layout, monitor summary, and session lock; repeated launches are guarded by a runtime identity file so bar clicks do not spawn unbounded surfaces. `hbctl quick --toggle` opens quick settings if absent and requests dismissal of the running quick popup if present. Use `hbctl quick --dev` for the older egui development window. `hbctl status` prints the same daemon health snapshot as JSON, including protocol/build info, recent events, and structured last-action metadata. Background launches write logs under `$XDG_RUNTIME_DIR/hyprbole/`; inspect them with `hbctl logs daemon`, `hbctl logs ui`, `hbctl logs control`, `hbctl logs quick`, `hbctl logs bar`, or `hbctl logs osd`. Use `hbctl logs list`, `hbctl logs --path`, or `hbctl logs --follow` for log paths and following. `hbctl ui-settings` prints Hyprbole-owned shell UI settings; `hbctl ui-settings set bar.<field> <value>` updates validated persisted bar settings through the daemon and `hbctl ui-settings reset bar` restores bar defaults. `hbctl notifications` prints daemon-owned notification/DND state; `hbctl notifications dnd on|off|toggle` persists DND, `hbctl notifications clear` clears bounded in-memory history, and `hbctl notifications push <summary> [body]` is a prototype/manual injection path. `hbctl bar` launches the current `wlr-layer-shell` bar prototype, backed by daemon snapshots and the daemon event stream. It renders bitmap workspace, active-window, audio/brightness summaries, notifications, current layout, clock, daemon/action status, and daemon/reconcile severity text according to persisted `ui.bar.widgets`. The bar routes explicit hit regions only for visible widgets: workspaces focus/scroll workspaces, the system region toggles mute and adjusts brightness by scroll, and the status region toggles quick settings. Bar scroll/mute/quick actions are lightly throttled to avoid daemon action spam. `hbctl layer-spike` remains an alias for the same prototype while it is still a low-level rendering spike.

`hbctl osd` opens a lightweight layer-shell OSD that follows daemon events, refreshes daemon snapshots on a background worker, shows transiently for daemon action events, and renders recent action status plus audio/brightness summaries. `hbctl theme` prints daemon-owned theme mode and effective built-in tokens. `hbctl theme mode system|light|dark` persists the mode through daemon IPC; `system` currently resolves to dark tokens until real OS-theme detection exists. `hbctl theme reset` restores the default dark mode. The control center now includes an Appearance pane for the same setting, and native bar/quick/OSD surfaces read color tokens from daemon snapshots.

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
- inspect and change persisted theme mode through the control center Appearance pane
- inspect notification DND/unread state and toggle DND through daemon IPC

Direct `hyprctl`, `wpctl`, and `loginctl` fallback calls are temporary prototype paths. The normal control path is `hyprbole-ui`/`hbctl` to the daemon over typed JSON IPC, with runtime actions implemented behind `hyprbole-core` adapters.

## Ownership Model

Hyprbole observes desktop state broadly, but only mutates subsystems that are shell-owned. Ownership is tracked per subsystem in `$XDG_CONFIG_HOME/hyprbole/settings.json`, falling back to `$HOME/.config/hyprbole/settings.json` when `XDG_CONFIG_HOME` is unset. `hbctl control` and `hbctl ownership capabilities` can inspect ownership capability metadata through daemon IPC. The current prototype reconciles runtime keybindings. Audio and brightness have partial runtime action/status adapters. Monitor and workspace reconciliation is observe-only and reports current ownership without changing user config. The control center and CLI reject runtime/persisted ownership choices for subsystems without runtime support.

Hyprbole-owned shell UI preferences live in the same settings file under `ui`. The current `ui.bar` settings are persisted Hyprbole settings, not projected Hyprland config. They cover bar enablement, monitor selector, top/bottom edge, height, margin, padding, and widget visibility/order. The bar applies edge, height, margin, padding, widget visibility/order, and theme colors at runtime from daemon snapshots. Monitor targeting now resolves `focused`, `primary`, and named monitors against daemon monitor snapshots and Wayland output names when available; if no match exists, the bar falls back safely and logs the fallback in `hbctl logs bar`.

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
