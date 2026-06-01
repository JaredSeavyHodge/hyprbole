# Hyprbole Shell Agent Guide

## Purpose

This branch defines a Rust-first Hyprland shell experiment. The previous Bash installer, desktop defaults, config templates, theme packs, and route-first command surface were intentionally removed from this branch.

## Current Architecture

- `crates/hyprbole-ui`: visible prototype UI
- `crates/hbctl`: Rust command/control entrypoint
- `crates/hyprbole-core`: typed state, actions, settings, Hyprland IPC, system adapters, and typed local IPC contracts
- `crates/hyprbole-daemon`: session-local Hyprbole service process
- `docs/shell-architecture.md`: product architecture direction

## Product Direction

- Build a lightweight Hyprland-native shell, not a full desktop environment.
- Mimic Noctalia's service/module/widget split, but implement the core in Rust.
- Keep the shell UI runtime shell-specific; do not build a general GTK/Qt replacement first.
- Use typed Unix-socket IPC as the product control path; REST/HTTP may exist only as explicitly marked debug transport.
- Avoid Quickshell/QML, GTK, Qt, Tauri, or egui as the long-term foundation until evidence proves otherwise. egui is currently only a fast visible prototype renderer.
- Keep direct `hyprctl`, `wpctl`, and `loginctl` calls temporary; move them behind `hyprbole-core` typed actions and the Hyprbole daemon.

## Editing Guidance

- Prefer Rust for shell runtime, CLI, state, and UI work.
- Do not add new Bash scripts, `bin/` routes, installer stages, copied config templates, or theme packs in this experiment branch unless explicitly requested.
- Keep changes small and directly tied to the shell architecture.
- Do not introduce a plugin system before core state/action/widget contracts stabilize.
- Do not rewrite user Hyprland config as the main persistence mechanism.
- Keep prototype docs clear about what is visible/testable versus production-ready.

## Review Guidance

- Runtime changes should be reviewed for UI responsiveness, stale async state, command/action safety, architecture boundaries, and visible testability.
- Docs changes should be reviewed for drift between README, architecture docs, implementation plan, and actual crates.
- When `.opencode/agent/*.md` or `.opencode/opencode.jsonc` changes, remind the user to restart opencode.
