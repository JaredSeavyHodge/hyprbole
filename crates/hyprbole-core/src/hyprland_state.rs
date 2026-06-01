use std::process::Command;

use serde::Deserialize;

use crate::daemon::{
    MonitorModeSnapshot, MonitorSnapshot, ShellSnapshot, WindowSnapshot, WorkspaceSnapshot,
};

pub const DEFAULT_WORKSPACE_COUNT: i64 = 5;

pub fn snapshot() -> ShellSnapshot {
    let workspaces = load_workspaces().unwrap_or_default();
    let windows = load_windows().unwrap_or_default();
    let monitors = load_monitors().unwrap_or_default();
    let monitor_count = monitors.len();
    let active_window = load_active_window().unwrap_or_else(|| "unavailable".to_string());
    ShellSnapshot {
        hyprland_available: !workspaces.is_empty() || monitor_count > 0,
        current_layout: load_current_layout(),
        workspaces,
        windows,
        monitors,
        monitor_count,
        active_window,
        ..ShellSnapshot::default()
    }
}

#[derive(Deserialize)]
struct HyprWorkspace {
    id: i64,
    name: String,
}

#[derive(Deserialize)]
struct HyprWindow {
    #[serde(default)]
    title: String,
}

#[derive(Deserialize)]
struct HyprMonitor {
    id: i64,
    name: String,
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default, rename = "refreshRate")]
    refresh_rate: f32,
    #[serde(default)]
    scale: f32,
    #[serde(default)]
    focused: bool,
    #[serde(default, rename = "activeWorkspace")]
    active_workspace: Option<HyprMonitorWorkspace>,
    #[serde(default, rename = "availableModes")]
    available_modes: Vec<String>,
}

#[derive(Deserialize)]
struct HyprMonitorWorkspace {
    id: i64,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct HyprActiveWindow {
    #[serde(default)]
    title: String,
    #[serde(default, rename = "class")]
    app_class: String,
}

#[derive(Deserialize)]
struct HyprOption {
    #[serde(default)]
    str: String,
}

fn load_workspaces() -> Option<Vec<WorkspaceSnapshot>> {
    let mut workspaces: Vec<HyprWorkspace> =
        serde_json::from_str(&command_output("hyprctl", &["workspaces", "-j"])?).ok()?;
    let active_ids = load_active_workspace_ids();
    let focused_id = load_focused_workspace_id();
    workspaces.sort_by_key(|workspace| workspace.id);
    let workspaces = workspaces
        .into_iter()
        .map(|workspace| WorkspaceSnapshot {
            focused: focused_id == Some(workspace.id),
            active: active_ids.contains(&workspace.id),
            occupied: true,
            id: workspace.id,
            name: workspace.name,
        })
        .collect();
    Some(normalize_workspaces(workspaces, DEFAULT_WORKSPACE_COUNT))
}

pub fn normalize_workspaces(
    mut workspaces: Vec<WorkspaceSnapshot>,
    default_count: i64,
) -> Vec<WorkspaceSnapshot> {
    for id in 1..=default_count {
        if workspaces.iter().any(|workspace| workspace.id == id) {
            continue;
        }
        workspaces.push(WorkspaceSnapshot {
            id,
            name: id.to_string(),
            focused: false,
            active: false,
            occupied: false,
        });
    }
    workspaces.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.name.cmp(&right.name))
    });
    workspaces
}

fn load_focused_workspace_id() -> Option<i64> {
    let workspace: HyprWorkspace =
        serde_json::from_str(&command_output("hyprctl", &["activeworkspace", "-j"])?).ok()?;
    Some(workspace.id)
}

fn load_active_workspace_ids() -> Vec<i64> {
    let Some(output) = command_output("hyprctl", &["monitors", "-j"]) else {
        return Vec::new();
    };
    let Ok(monitors) = serde_json::from_str::<Vec<HyprMonitor>>(&output) else {
        return Vec::new();
    };
    monitors
        .into_iter()
        .filter_map(|monitor| monitor.active_workspace.map(|workspace| workspace.id))
        .collect()
}

fn load_windows() -> Option<Vec<WindowSnapshot>> {
    let windows: Vec<HyprWindow> =
        serde_json::from_str(&command_output("hyprctl", &["clients", "-j"])?).ok()?;
    Some(
        windows
            .into_iter()
            .map(|window| WindowSnapshot {
                title: window.title,
            })
            .collect(),
    )
}

fn load_monitors() -> Option<Vec<MonitorSnapshot>> {
    let monitors: Vec<HyprMonitor> =
        serde_json::from_str(&command_output("hyprctl", &["monitors", "-j"])?).ok()?;
    Some(
        monitors
            .into_iter()
            .map(|monitor| MonitorSnapshot {
                id: monitor.id,
                name: monitor.name,
                focused: monitor.focused,
                position: monitor_position(monitor.x, monitor.y),
                scale: monitor_scale(monitor.scale),
                current_mode: current_monitor_mode(
                    monitor.width,
                    monitor.height,
                    monitor.refresh_rate,
                ),
                active_workspace: monitor.active_workspace.map(|workspace| {
                    if workspace.name.is_empty() {
                        workspace.id.to_string()
                    } else {
                        workspace.name
                    }
                }),
                available_modes: monitor
                    .available_modes
                    .into_iter()
                    .filter_map(|mode| parse_monitor_mode(&mode))
                    .collect(),
            })
            .collect(),
    )
}

fn monitor_position(x: i32, y: i32) -> String {
    format!("{x}x{y}")
}

fn monitor_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

fn current_monitor_mode(width: u32, height: u32, refresh_hz: f32) -> Option<MonitorModeSnapshot> {
    if width == 0 || height == 0 || refresh_hz <= 0.0 {
        return None;
    }
    let label = format!("{}x{}@{:.2}Hz", width, height, refresh_hz);
    Some(MonitorModeSnapshot {
        width,
        height,
        refresh_hz,
        label,
    })
}

fn parse_monitor_mode(input: &str) -> Option<MonitorModeSnapshot> {
    let (size, refresh) = input.split_once('@')?;
    let (width, height) = size.split_once('x')?;
    let width = width.parse().ok()?;
    let height = height.parse().ok()?;
    let refresh_hz = refresh.trim_end_matches("Hz").parse().ok()?;
    Some(MonitorModeSnapshot {
        width,
        height,
        refresh_hz,
        label: input.to_string(),
    })
}

fn load_active_window() -> Option<String> {
    let active: HyprActiveWindow =
        serde_json::from_str(&command_output("hyprctl", &["activewindow", "-j"])?).ok()?;
    if active.title.is_empty() && active.app_class.is_empty() {
        return Some("none".to_string());
    }
    Some(
        format!("{} {}", active.app_class, active.title)
            .trim()
            .to_string(),
    )
}

fn load_current_layout() -> Option<String> {
    let option: HyprOption = serde_json::from_str(&command_output(
        "hyprctl",
        &["-j", "getoption", "general:layout"],
    )?)
    .ok()?;
    let layout = option.str.trim();
    if layout.is_empty() {
        None
    } else {
        Some(layout.to_string())
    }
}

fn command_output(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_workspaces_adds_default_unoccupied_workspaces() {
        let workspaces = normalize_workspaces(
            vec![WorkspaceSnapshot {
                id: 3,
                name: "3".to_string(),
                focused: true,
                active: true,
                occupied: true,
            }],
            5,
        );

        assert_eq!(
            workspaces
                .iter()
                .map(|workspace| workspace.name.as_str())
                .collect::<Vec<_>>(),
            ["1", "2", "3", "4", "5"]
        );
        assert!(!workspaces[0].occupied);
        assert!(workspaces[2].focused);
        assert!(workspaces[2].occupied);
    }

    #[test]
    fn normalize_workspaces_keeps_extra_workspaces() {
        let input = (1..=14)
            .map(|id| WorkspaceSnapshot {
                id,
                name: id.to_string(),
                focused: id == 14,
                active: true,
                occupied: true,
            })
            .collect::<Vec<_>>();

        let workspaces = normalize_workspaces(input, 5);

        assert_eq!(workspaces.len(), 14);
        assert_eq!(
            workspaces.last().map(|workspace| workspace.name.as_str()),
            Some("14")
        );
        assert!(workspaces.last().is_some_and(|workspace| workspace.focused));
    }

    #[test]
    fn current_monitor_mode_formats_hyprland_label() {
        let mode = current_monitor_mode(2535, 1373, 74.998).expect("mode");
        assert_eq!(mode.label, "2535x1373@75.00Hz");
        assert!(current_monitor_mode(0, 1373, 74.998).is_none());
    }

    #[test]
    fn monitor_position_and_scale_preserve_current_geometry() {
        assert_eq!(monitor_position(-1920, 0), "-1920x0");
        assert_eq!(monitor_scale(1.25), 1.25);
        assert_eq!(monitor_scale(0.0), 1.0);
    }
}
