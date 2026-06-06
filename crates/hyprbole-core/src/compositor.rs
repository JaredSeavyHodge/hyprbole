use std::{env, fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::hyprland::{HyprlandIpc, HyprlandIpcError};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub fn as_hyprland(self) -> &'static str {
        match self {
            Self::Left => "l",
            Self::Right => "r",
            Self::Up => "u",
            Self::Down => "d",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompositorAction {
    FocusWorkspace {
        name: String,
    },
    FocusNextWorkspace,
    FocusPreviousWorkspace,
    SetLayout {
        name: String,
    },
    SetMonitorMode {
        monitor: String,
        mode: String,
        position: String,
        scale: f32,
    },
    RegisterRuntimeKeybindings,
    UnregisterRuntimeKeybindings,
    RegisterMouseWindowControls,
    UnregisterMouseWindowControls,
}

#[derive(Debug)]
pub enum CompositorError {
    Hyprland(HyprlandIpcError),
    InvalidAction(String),
}

impl fmt::Display for CompositorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hyprland(err) => write!(f, "{err}"),
            Self::InvalidAction(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for CompositorError {}

impl From<HyprlandIpcError> for CompositorError {
    fn from(value: HyprlandIpcError) -> Self {
        Self::Hyprland(value)
    }
}

pub trait CompositorBackend {
    fn dispatch(&self, action: CompositorAction) -> Result<(), CompositorError>;
}

#[derive(Debug, Clone)]
pub struct HyprlandIpcBackend {
    ipc: HyprlandIpc,
}

impl HyprlandIpcBackend {
    pub fn from_env() -> Result<Self, CompositorError> {
        Ok(Self {
            ipc: HyprlandIpc::from_env()?,
        })
    }
}

impl CompositorBackend for HyprlandIpcBackend {
    fn dispatch(&self, action: CompositorAction) -> Result<(), CompositorError> {
        match action {
            CompositorAction::FocusWorkspace { name } => {
                self.ipc.eval_ok(&format!(
                    "hl.dispatch(hl.dsp.focus({{ workspace = {:?} }}))",
                    name
                ))?;
            }
            CompositorAction::FocusNextWorkspace => {
                self.ipc
                    .eval_ok(r#"hl.dispatch(hl.dsp.focus({ workspace = "r+1" }))"#)?;
            }
            CompositorAction::FocusPreviousWorkspace => {
                self.ipc
                    .eval_ok(r#"hl.dispatch(hl.dsp.focus({ workspace = "r-1" }))"#)?;
            }
            CompositorAction::SetLayout { name } => {
                if !matches!(name.as_str(), "dwindle" | "master" | "scrolling") {
                    return Err(CompositorError::InvalidAction(format!(
                        "unsupported layout `{name}`"
                    )));
                }
                self.ipc.eval_ok(&set_layout_eval(&name))?;
            }
            CompositorAction::SetMonitorMode {
                monitor,
                mode,
                position,
                scale,
            } => {
                if !safe_keyword_value(&monitor)
                    || !safe_monitor_mode(&mode)
                    || !safe_monitor_position(&position)
                    || !safe_monitor_scale(scale)
                {
                    return Err(CompositorError::InvalidAction(
                        "invalid monitor mode payload".to_string(),
                    ));
                }
                self.ipc
                    .eval_ok(&set_monitor_mode_eval(&monitor, &mode, &position, scale))?;
            }
            CompositorAction::RegisterMouseWindowControls => {
                register_mouse_window_controls(&self.ipc)?;
            }
            CompositorAction::RegisterRuntimeKeybindings => {
                register_mouse_window_controls(&self.ipc)?;
                register_launcher_keybinding(&self.ipc)?;
            }
            CompositorAction::UnregisterMouseWindowControls => {
                unregister_mouse_window_controls(&self.ipc)?;
            }
            CompositorAction::UnregisterRuntimeKeybindings => {
                unregister_mouse_window_controls(&self.ipc)?;
                unregister_launcher_keybinding(&self.ipc)?;
            }
        }
        Ok(())
    }
}

fn register_mouse_window_controls(ipc: &HyprlandIpc) -> Result<(), HyprlandIpcError> {
    for lua in [
        r#"hl.unbind("SUPER + mouse:272")"#,
        r#"hl.bind("SUPER + mouse:272", hl.dsp.window.drag(), { mouse = true })"#,
        r#"hl.unbind("SUPER + mouse:273")"#,
        r#"hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })"#,
    ] {
        ipc.eval_ok(lua)?;
    }
    Ok(())
}

fn unregister_mouse_window_controls(ipc: &HyprlandIpc) -> Result<(), HyprlandIpcError> {
    for lua in [
        r#"hl.unbind("SUPER + mouse:272")"#,
        r#"hl.unbind("SUPER + mouse:273")"#,
    ] {
        ipc.eval_ok(lua)?;
    }
    Ok(())
}

fn register_launcher_keybinding(ipc: &HyprlandIpc) -> Result<(), HyprlandIpcError> {
    unregister_launcher_keybinding(ipc)?;
    let command = lua_string(&format!("{} launcher &", launcher_command()));
    ipc.eval_ok(&format!(
        r#"hl.bind("SUPER + ALT + SPACE", function() os.execute({command}) end)"#
    ))
}

fn unregister_launcher_keybinding(ipc: &HyprlandIpc) -> Result<(), HyprlandIpcError> {
    ipc.eval_ok(r#"hl.unbind("SUPER + ALT + SPACE")"#)
}

fn launcher_command() -> String {
    let local = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/bin/hbctl"));
    if let Some(path) = local.filter(|path| path.exists()) {
        return path.display().to_string();
    }
    "hbctl".to_string()
}

fn lua_string(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn safe_keyword_value(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn safe_monitor_mode(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, 'x' | '@' | '.' | 'H' | 'z'))
}

fn safe_monitor_position(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, 'x' | '-'))
}

fn safe_monitor_scale(value: f32) -> bool {
    value.is_finite() && value > 0.0 && value <= 10.0
}

fn set_layout_eval(name: &str) -> String {
    format!("hl.config({{ general = {{ layout = {name:?} }} }})")
}

fn set_monitor_mode_eval(monitor: &str, mode: &str, position: &str, scale: f32) -> String {
    format!(
        "hl.monitor({{ output = {monitor:?}, mode = {mode:?}, position = {position:?}, scale = {scale} }})"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_mode_validator_accepts_hyprland_labels() {
        assert!(safe_monitor_mode("2535x1373@75.00Hz"));
        assert!(!safe_monitor_mode("2535x1373@75.00Hz,auto,1"));
        assert!(!safe_monitor_mode("2535x1373@75.00Hz;shutdown"));
        assert!(safe_monitor_position("0x0"));
        assert!(safe_monitor_position("-1920x0"));
        assert!(!safe_monitor_position("auto"));
        assert!(safe_monitor_scale(1.25));
        assert!(!safe_monitor_scale(0.0));
    }

    #[test]
    fn layout_eval_uses_lua_config_api() {
        assert_eq!(
            set_layout_eval("dwindle"),
            "hl.config({ general = { layout = \"dwindle\" } })"
        );
    }

    #[test]
    fn lua_string_escapes_launcher_command() {
        assert_eq!(
            lua_string(r#"/tmp/a\"b/hbctl launcher &"#),
            "\"/tmp/a\\\\\\\"b/hbctl launcher &\""
        );
    }

    #[test]
    fn monitor_mode_eval_uses_lua_monitor_api() {
        assert_eq!(
            set_monitor_mode_eval("Virtual-1", "2535x1373@75.00Hz", "-1920x0", 1.25),
            "hl.monitor({ output = \"Virtual-1\", mode = \"2535x1373@75.00Hz\", position = \"-1920x0\", scale = 1.25 })"
        );
    }
}
