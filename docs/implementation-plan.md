# Implementation Plan

## Done In This Branch

- Removed legacy Bash/config/install/theme surfaces from the experiment branch.
- Added Rust workspace.
- Added `hyprbole-core` with a direct Hyprland IPC client for early action experiments.
- Added `hyprbole-daemon` with a minimal Unix socket request loop.
- Added `hyprbole-ui` visible prototype window.
- Added background refresh/action workers so the UI remains responsive.
- Added request sequencing so stale worker replies cannot overwrite newer state.
- Added `hbctl ui` as the Rust launcher/control entrypoint.
- Added `hbctl daemon`, `hbctl ping`, `hbctl bind-mouse-ipc`, and `hbctl shutdown` as early daemon/client test commands.
- Added typed JSON daemon requests/responses for state, actions, settings, and reconciliation.
- Added per-subsystem ownership settings with `respect_user_config`, `runtime_owned`, and `persisted_owned` modes.
- Added daemon startup/reload/reconnect reconciliation for runtime-owned keybindings.
- Added `hbctl state`, `hbctl settings`, `hbctl settings set`, and `hbctl reconcile`.
- Added `hbctl status` for daemon PID, uptime, socket/settings paths, event buffer state, structured reconcile status, structured last-action metadata, and compatibility save/action status strings.
- Added runtime log paths and `hbctl logs` for daemon/UI/control/quick/bar/OSD background launches, including log listing, paths, and follow mode.
- Added `hbctl binds`, `hbctl events`, and `hbctl events --follow` for keybinding and daemon event inspection.
- Added `hbctl bar` as the layer-shell bar prototype launcher.
- Added `hbctl ownership` as shorthand for per-subsystem ownership settings.
- Added `hbctl control` as a daemon-backed control center launcher.
- Added `hbctl quick` as a compact daemon-backed quick settings launcher.
- Added `hbctl osd` as a daemon-backed layer-shell OSD launcher.
- Added a layer-shell quick settings popup as the default `hbctl quick` surface, with `hbctl quick --dev` preserving the older egui development window.
- Added quick settings single-instance guarding via runtime identity file; `hbctl quick --toggle` opens quick settings when absent and requests dismissal of the running quick settings process when present.
- Added daemon-backed ownership controls and daemon/event status to the control center.
- Added daemon status details and structured reconcile severity to the control center.
- Added protocol/build status, recent events, and scoped reconcile controls to the control center.
- Added structured audio and brightness snapshots while keeping display summaries.
- Added typed notification/DND state, persisted DND settings, bounded in-memory notification history, and `hbctl notifications` inspection/action commands.
- Added typed theme mode/tokens, persisted theme settings, `hbctl theme` inspection/action commands, and a control-center Appearance pane.
- Added ownership capability metadata and `hbctl ownership capabilities`.
- Added reserved ownership warnings for subsystems without runtime reconcilers.
- Added observe-only monitor/workspace reconciliation reporting.
- Added bitmap text rendering to the layer-shell bar prototype for workspace, active-window, audio, brightness, current layout, action status, and daemon/reconcile severity status.
- Added daemon-backed bar interactions for workspace focus/scroll, audio mute, brightness scroll, and quick settings launch.
- Added explicit bar hit-region routing and tests for workspace, window, status, and system regions.
- Added bar action throttling for rapid workspace scroll, brightness scroll, mute, and quick toggle events.
- Added persisted `ui.bar` shell settings for bar enablement, edge, height, margin, padding, monitor selector, and widget visibility/order.
- Added `hbctl ui-settings` for daemon-backed shell UI settings inspection, bar field updates, and bar reset.
- Added a control-center Bar pane for editing persisted bar enablement, edge, height, and widget visibility.
- Updated the layer-shell bar to render and expose hit regions from persisted bar settings instead of hardcoded widget visibility.
- Expanded the control-center Bar pane with monitor, margin, padding, widget order controls, and widget reset.
- Updated the layer-shell bar to apply configured margin/padding, ordered system widgets, and more daemon-provided theme colors.
- Added bar monitor targeting resolution for focused, primary, and named monitors by matching daemon monitor names to Wayland output names, with safe fallback logging.
- Added daemon event classification helpers so surfaces can distinguish snapshot refreshes from status refreshes.
- Added a native layer-shell OSD surface for recent daemon action feedback plus audio/brightness summaries, with background daemon refresh and idle auto-hide.
- Added `hyprbole-core::hyprland_state` as the daemon's Hyprland snapshot adapter.
- Gated UI action fallbacks behind `--debug-direct-fallback`.
- Added structured daemon logs and slow shell-command profiling for prototype adapters.
- Added daemon cached snapshots with monitor, brightness, reconcile status, and recent event state.
- Added settings load/save tests and corrupt settings safe fallback coverage.
- Added `FileSettingsStore` as the atomic structured-file settings boundary, with tests for store round-trips and write-failure preservation of existing settings.
- Added daemon ownership persistence tests for save success/failure behavior.
- Added daemon status, structured reconcile, and observe-only reconciliation regression tests.
- Added a mock-backed daemon runtime test harness for handler-level state, settings, UI settings, bar persistence, action/event, and keybinding request coverage without live Hyprland.
- Added `hbctl ui-settings` parser tests so command request construction is covered without a live daemon.
- Added a minimal `wlr-layer-shell` smoke surface via `hbctl layer-spike`.
- Scoped opencode reviewers to shell runtime and shell docs.

## Current Prototype

Run:

```bash
cargo run -p hbctl -- ui
```

The visible UI can show Hyprland state, focus workspaces, adjust output volume, toggle output mute, adjust brightness, toggle notification DND, switch runtime layout/monitor mode through daemon IPC, refresh state, request a session lock, open a daemon-backed control center, open compact layer-shell quick settings, and open a layer-shell OSD. The control center can edit persisted bar settings, widget order, and theme mode. The layer-shell bar can focus/scroll workspaces, show configured active-window/audio/brightness/notifications/layout/action/status/clock widgets, mute audio, adjust brightness by scroll, apply margin/padding and daemon-provided theme color tokens, and toggle the single-instance quick settings popup from its configured status region. The layer-shell quick popup and OSD also consume daemon-provided color tokens for core background/text colors. The OSD follows daemon events, refreshes daemon state asynchronously, auto-hides when idle, and renders recent action status plus current audio/brightness summaries.

## Next Milestones

1. Extend the mock daemon harness to cover future monitor/workspace runtime reconcilers before enabling mutation.
2. Continue hardening layer-shell lifecycle behavior across output hotplug and compositor reconnects.
3. Expand theme tokens cautiously only where shell surfaces consume them; `hbctl theme reset` currently returns to default dark mode, and external app projection remains later work.
4. Add monitor/workspace runtime reconcilers only after observe-only behavior and mock-backed tests are proven safe.
5. Replace egui only after widget/state contracts are proven.

## Guardrails

- Do not reintroduce the legacy `bin/`, `config/`, `default/`, `install/`, `themes/`, or migration trees in this branch.
- Do not add REST as the product API.
- Do not add a plugin system yet.
- Do not treat egui as the final shell runtime.
