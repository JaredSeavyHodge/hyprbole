use std::env;
use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::audio::AudioAction;
use crate::audio::AudioSnapshot;
use crate::brightness::BrightnessAction;
use crate::brightness::BrightnessSnapshot;
use crate::compositor::CompositorAction;
use crate::notifications::{NotificationAction, NotificationSnapshot};
use crate::settings::{BarSettings, OsdSettings, OwnershipMode, ShellSettings, ShellSubsystem};
use crate::theme::{ThemeMode, ThemeSnapshot};

pub const SOCKET_FILE_NAME: &str = "hyprbole.sock";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonRequest {
    Ping,
    BindMouse,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShellRequest {
    Ping,
    StatusGet,
    StateGet,
    SettingsGet,
    SettingsSetOwnership {
        subsystem: ShellSubsystem,
        mode: OwnershipMode,
    },
    SettingsSetAutostart {
        apps: Vec<crate::settings::AutostartApp>,
    },
    UiSettingsGet,
    ThemeGet,
    ThemeSetMode {
        mode: ThemeMode,
    },
    ThemeReset,
    NotificationsGet,
    BarSettingsSet {
        settings: BarSettings,
    },
    BarSettingsSetField {
        field: String,
        value: String,
    },
    BarSettingsReset,
    OsdSettingsSet {
        settings: OsdSettings,
    },
    OsdSettingsSetField {
        field: String,
        value: String,
    },
    OsdSettingsReset,
    AutostartRun {
        name: String,
    },
    BindsGet,
    EventsSubscribe {
        after: Option<u64>,
    },
    EventsFollow {
        after: Option<u64>,
    },
    Reconcile {
        subsystem: Option<ShellSubsystem>,
    },
    ActionCall {
        action: ShellAction,
    },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "domain", rename_all = "snake_case")]
pub enum ShellAction {
    Compositor { action: CompositorAction },
    Audio { action: AudioAction },
    Brightness { action: BrightnessAction },
    Notifications { action: NotificationAction },
    SessionLock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ShellResponse {
    Ok {
        message: String,
    },
    State {
        snapshot: ShellSnapshot,
    },
    Settings {
        settings: ShellSettings,
    },
    UiSettings {
        settings: crate::settings::UiSettings,
    },
    Theme {
        snapshot: ThemeSnapshot,
    },
    Binds {
        keybindings: Vec<KeybindingSnapshot>,
    },
    Notifications {
        snapshot: NotificationSnapshot,
    },
    Events {
        events: Vec<ShellEventRecord>,
    },
    Status {
        daemon: DaemonStatus,
    },
    Error {
        message: String,
    },
}

impl ShellResponse {
    pub fn ok(message: impl Into<String>) -> Self {
        Self::Ok {
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(
            self,
            Self::Ok { .. }
                | Self::State { .. }
                | Self::Settings { .. }
                | Self::UiSettings { .. }
                | Self::Theme { .. }
                | Self::Binds { .. }
                | Self::Notifications { .. }
                | Self::Events { .. }
                | Self::Status { .. }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub protocol_version: u32,
    pub build_version: String,
    pub pid: u32,
    pub uptime_seconds: u64,
    pub socket_path: String,
    pub settings_path: String,
    pub event_count: usize,
    pub next_event_id: u64,
    pub recent_events: Vec<ShellEventRecord>,
    pub ownership_capabilities: Vec<SubsystemCapability>,
    pub last_hyprland_event: Option<String>,
    pub last_reconcile_status: Option<String>,
    pub reconcile: Option<ReconcileStatus>,
    pub last_settings_save_status: Option<String>,
    pub last_action_status: Option<String>,
    #[serde(default)]
    pub last_action: Option<ActionStatus>,
    #[serde(default)]
    pub autostart: Vec<AutostartAppStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStatus {
    pub domain: String,
    pub name: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutostartAppStatus {
    pub name: String,
    pub command: String,
    pub enabled: bool,
    pub running: bool,
    pub pid: Option<u32>,
    pub last_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileStatus {
    pub severity: ReconcileSeverity,
    pub summary: String,
    pub results: Vec<SubsystemReconcileStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconcileSeverity {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemReconcileStatus {
    pub subsystem: ShellSubsystem,
    pub severity: ReconcileSeverity,
    pub implemented: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemCapability {
    pub subsystem: ShellSubsystem,
    pub runtime_reconcile: CapabilitySupport,
    pub persisted_ownership: CapabilitySupport,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySupport {
    Implemented,
    Partial,
    ObserveOnly,
    Reserved,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShellSnapshot {
    pub hyprland_available: bool,
    #[serde(default)]
    pub current_layout: Option<String>,
    pub workspaces: Vec<WorkspaceSnapshot>,
    pub windows: Vec<WindowSnapshot>,
    pub monitors: Vec<MonitorSnapshot>,
    pub monitor_count: usize,
    pub active_window: String,
    pub audio_summary: String,
    pub audio: AudioSnapshot,
    pub brightness_summary: String,
    pub brightness: BrightnessSnapshot,
    #[serde(default)]
    pub notifications: NotificationSnapshot,
    #[serde(default = "default_theme_snapshot")]
    pub theme: ThemeSnapshot,
    pub last_hyprland_event: Option<String>,
    pub last_reconcile_status: Option<String>,
    pub reconcile: Option<ReconcileStatus>,
    pub settings: ShellSettings,
}

fn default_theme_snapshot() -> ThemeSnapshot {
    crate::theme::snapshot(&crate::theme::ThemeSettings::default())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub id: i64,
    pub name: String,
    pub focused: bool,
    pub active: bool,
    #[serde(default)]
    pub occupied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSnapshot {
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSnapshot {
    pub id: i64,
    pub name: String,
    pub focused: bool,
    #[serde(default)]
    pub position: String,
    #[serde(default = "default_monitor_scale")]
    pub scale: f32,
    #[serde(default)]
    pub current_mode: Option<MonitorModeSnapshot>,
    pub active_workspace: Option<String>,
    #[serde(default)]
    pub available_modes: Vec<MonitorModeSnapshot>,
}

fn default_monitor_scale() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorModeSnapshot {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: f32,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingSnapshot {
    pub id: String,
    pub description: String,
    pub expected: String,
    pub active: bool,
    pub source: KeybindingSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeybindingSource {
    HyprboleRuntime,
    UserConfig,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellEventRecord {
    pub id: u64,
    pub event: ShellEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShellEvent {
    Gap {
        after: u64,
        oldest: u64,
    },
    Hyprland {
        raw: String,
    },
    Reconcile {
        status: String,
    },
    Action {
        status: ActionStatus,
    },
    StateChanged {
        reason: String,
        #[serde(default)]
        domain: Option<String>,
    },
}

impl ShellEvent {
    pub fn domain(&self) -> Option<&str> {
        match self {
            Self::Gap { .. } => None,
            Self::Hyprland { .. } => Some("compositor"),
            Self::Reconcile { .. } => Some("reconcile"),
            Self::Action { status } => Some(status.domain.as_str()),
            Self::StateChanged { domain, .. } => domain.as_deref(),
        }
    }

    pub fn refreshes_snapshot(&self) -> bool {
        matches!(self, Self::Gap { .. } | Self::Hyprland { .. })
            || matches!(
                self.domain(),
                Some("compositor" | "audio" | "brightness" | "settings" | "autostart" | "theme")
                    | Some("notifications")
            )
    }

    pub fn refreshes_status(&self) -> bool {
        matches!(
            self,
            Self::Gap { .. } | Self::Action { .. } | Self::Reconcile { .. }
        ) || matches!(
            self.domain(),
            Some("settings" | "autostart" | "session" | "reconcile") | Some("notifications")
        )
    }
}

impl DaemonRequest {
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Ping => "ping",
            Self::BindMouse => "hyprland.bind_mouse",
            Self::Shutdown => "shutdown",
        }
    }

    pub fn parse(input: &str) -> Result<Self, DaemonProtocolError> {
        match input.trim() {
            "ping" => Ok(Self::Ping),
            "hyprland.bind_mouse" => Ok(Self::BindMouse),
            "shutdown" => Ok(Self::Shutdown),
            command => Err(DaemonProtocolError::UnknownCommand(command.to_owned())),
        }
    }
}

#[derive(Debug)]
pub enum DaemonProtocolError {
    UnknownCommand(String),
}

impl fmt::Display for DaemonProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(command) => write!(f, "unknown daemon command `{command}`"),
        }
    }
}

impl std::error::Error for DaemonProtocolError {}

#[derive(Debug)]
pub enum DaemonClientError {
    MissingEnvironment(&'static str),
    CreateSocketDir {
        path: PathBuf,
        source: std::io::Error,
    },
    Connect {
        path: PathBuf,
        source: std::io::Error,
    },
    Write(std::io::Error),
    Shutdown(std::io::Error),
    Read(std::io::Error),
    Rejected(String),
}

impl fmt::Display for DaemonClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnvironment(name) => write!(f, "{name} is not set"),
            Self::CreateSocketDir { path, source } => {
                write!(f, "create socket dir {}: {source}", path.display())
            }
            Self::Connect { path, source } => write!(f, "connect {}: {source}", path.display()),
            Self::Write(source) => write!(f, "write request: {source}"),
            Self::Shutdown(source) => write!(f, "shutdown write side: {source}"),
            Self::Read(source) => write!(f, "read response: {source}"),
            Self::Rejected(reply) => write!(f, "daemon rejected request: {}", reply.trim()),
        }
    }
}

impl std::error::Error for DaemonClientError {}

pub type DaemonClientResult<T> = Result<T, DaemonClientError>;

#[derive(Debug, Clone)]
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    pub fn from_env() -> DaemonClientResult<Self> {
        Ok(Self {
            socket_path: socket_path_from_env()?,
        })
    }

    pub fn send(&self, request: DaemonRequest) -> DaemonClientResult<String> {
        let mut stream = UnixStream::connect(&self.socket_path).map_err(|source| {
            DaemonClientError::Connect {
                path: self.socket_path.clone(),
                source,
            }
        })?;

        let wire = format!("{}\n", request.as_wire());
        stream
            .write_all(wire.as_bytes())
            .map_err(DaemonClientError::Write)?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(DaemonClientError::Shutdown)?;

        let mut reply = String::new();
        stream
            .read_to_string(&mut reply)
            .map_err(DaemonClientError::Read)?;

        if reply.trim_start().starts_with("ok") {
            Ok(reply)
        } else {
            Err(DaemonClientError::Rejected(reply))
        }
    }

    pub fn send_shell(&self, request: &ShellRequest) -> DaemonClientResult<ShellResponse> {
        let mut stream = UnixStream::connect(&self.socket_path).map_err(|source| {
            DaemonClientError::Connect {
                path: self.socket_path.clone(),
                source,
            }
        })?;

        let wire = serde_json::to_string(request)
            .map_err(|err| DaemonClientError::Rejected(err.to_string()))?
            + "\n";
        stream
            .write_all(wire.as_bytes())
            .map_err(DaemonClientError::Write)?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(DaemonClientError::Shutdown)?;

        let mut reply = String::new();
        stream
            .read_to_string(&mut reply)
            .map_err(DaemonClientError::Read)?;
        serde_json::from_str::<ShellResponse>(&reply)
            .map_err(|err| DaemonClientError::Rejected(format!("{err}: {reply}")))
    }

    pub fn follow_events(&self, after: Option<u64>) -> DaemonClientResult<DaemonEventStream> {
        let mut stream = UnixStream::connect(&self.socket_path).map_err(|source| {
            DaemonClientError::Connect {
                path: self.socket_path.clone(),
                source,
            }
        })?;
        let wire = serde_json::to_string(&ShellRequest::EventsFollow { after })
            .map_err(|err| DaemonClientError::Rejected(err.to_string()))?
            + "\n";
        stream
            .write_all(wire.as_bytes())
            .map_err(DaemonClientError::Write)?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(DaemonClientError::Shutdown)?;
        Ok(DaemonEventStream {
            reader: BufReader::new(stream),
            buffer: String::new(),
        })
    }
}

pub struct DaemonEventStream {
    reader: BufReader<UnixStream>,
    buffer: String,
}

impl Iterator for DaemonEventStream {
    type Item = DaemonClientResult<ShellEventRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.clear();
        match self.reader.read_line(&mut self.buffer) {
            Ok(0) => None,
            Ok(_) => Some(
                serde_json::from_str::<ShellEventRecord>(&self.buffer)
                    .map_err(|err| DaemonClientError::Rejected(format!("{err}: {}", self.buffer))),
            ),
            Err(err) => Some(Err(DaemonClientError::Read(err))),
        }
    }
}

pub fn socket_path_from_env() -> DaemonClientResult<PathBuf> {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or(DaemonClientError::MissingEnvironment("XDG_RUNTIME_DIR"))?;
    Ok(runtime_dir.join("hyprbole").join(SOCKET_FILE_NAME))
}

pub fn ensure_socket_dir() -> DaemonClientResult<PathBuf> {
    let socket_path = socket_path_from_env()?;
    let Some(socket_dir) = socket_path.parent() else {
        return Ok(socket_path);
    };
    std::fs::create_dir_all(socket_dir).map_err(|source| DaemonClientError::CreateSocketDir {
        path: socket_dir.to_path_buf(),
        source,
    })?;
    Ok(socket_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_request_round_trips_json() {
        let request = ShellRequest::StatusGet;
        let json = serde_json::to_string(&request).unwrap();
        let decoded: ShellRequest = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, ShellRequest::StatusGet));
    }

    #[test]
    fn bar_settings_field_request_round_trips_json() {
        let request = ShellRequest::BarSettingsSetField {
            field: "bar.height".to_string(),
            value: "40".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        let decoded: ShellRequest = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            decoded,
            ShellRequest::BarSettingsSetField { field, value }
                if field == "bar.height" && value == "40"
        ));
    }

    #[test]
    fn osd_settings_field_request_round_trips_json() {
        let request = ShellRequest::OsdSettingsSetField {
            field: "osd.timeout_ms".to_string(),
            value: "2500".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        let decoded: ShellRequest = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            decoded,
            ShellRequest::OsdSettingsSetField { field, value }
                if field == "osd.timeout_ms" && value == "2500"
        ));
    }

    #[test]
    fn notifications_get_request_round_trips_json() {
        let request = ShellRequest::NotificationsGet;
        let json = serde_json::to_string(&request).unwrap();
        let decoded: ShellRequest = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, ShellRequest::NotificationsGet));
    }

    #[test]
    fn theme_set_mode_request_round_trips_json() {
        let request = ShellRequest::ThemeSetMode {
            mode: ThemeMode::Light,
        };
        let json = serde_json::to_string(&request).unwrap();
        let decoded: ShellRequest = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            decoded,
            ShellRequest::ThemeSetMode {
                mode: ThemeMode::Light
            }
        ));
    }

    #[test]
    fn reconcile_request_round_trips_json() {
        let request = ShellRequest::Reconcile {
            subsystem: Some(ShellSubsystem::Monitors),
        };
        let json = serde_json::to_string(&request).unwrap();
        let decoded: ShellRequest = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            decoded,
            ShellRequest::Reconcile {
                subsystem: Some(ShellSubsystem::Monitors)
            }
        ));
    }

    #[test]
    fn shell_snapshot_round_trips_json() {
        let snapshot = ShellSnapshot::default();
        let json = serde_json::to_string(&snapshot).unwrap();
        let decoded: ShellSnapshot = serde_json::from_str(&json).unwrap();
        assert!(!decoded.hyprland_available);
        assert!(decoded.current_layout.is_none());
    }

    #[test]
    fn shell_event_classifies_refresh_domains() {
        let action = ShellEvent::Action {
            status: ActionStatus {
                domain: "audio".to_string(),
                name: "toggle_mute".to_string(),
                ok: true,
                message: "ok".to_string(),
            },
        };
        assert_eq!(action.domain(), Some("audio"));
        assert!(action.refreshes_snapshot());
        assert!(action.refreshes_status());

        let state = ShellEvent::StateChanged {
            reason: "settings_changed".to_string(),
            domain: Some("settings".to_string()),
        };
        assert_eq!(state.domain(), Some("settings"));
        assert!(state.refreshes_snapshot());
        assert!(state.refreshes_status());

        let theme = ShellEvent::StateChanged {
            reason: "theme_changed".to_string(),
            domain: Some("theme".to_string()),
        };
        assert!(theme.refreshes_snapshot());

        let hyprland = ShellEvent::Hyprland {
            raw: "workspace>>1".to_string(),
        };
        assert_eq!(hyprland.domain(), Some("compositor"));
        assert!(hyprland.refreshes_snapshot());
    }
}
