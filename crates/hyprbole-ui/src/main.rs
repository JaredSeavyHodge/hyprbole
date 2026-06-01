use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use std::{fs::OpenOptions, io::Write};

use eframe::egui::{self, Color32, CornerRadius, FontId, Margin, RichText, Stroke, Vec2};
use hyprbole_core::settings::{
    BarMonitor, BarSettings, BarWidget, OwnershipMode, ShellSettings, ShellSubsystem, WidgetMove,
};
use hyprbole_core::theme::ThemeMode;
use serde::Deserialize;

mod bar;
mod osd;
mod quick;

const CONTROL_MIN_WIDTH: f32 = 640.0;
const CONTROL_MIN_HEIGHT: f32 = 520.0;
const CONTROL_SIDEBAR_WIDTH: f32 = 160.0;
const CONTROL_PANE_MIN_WIDTH: f32 = 420.0;
const CONTROL_MONITOR_MODE_LIMIT: usize = 8;
const COMPOSITOR_CONFIG_APPLY_SUPPORTED: bool = true;
#[cfg(test)]
const CONTROL_LAYOUT_GUTTER: f32 = 60.0;

fn main() -> eframe::Result {
    if std::env::args().any(|arg| arg == "--bar" || arg == "--layer-spike") {
        if let Err(err) = bar::run() {
            eprintln!("bar failed: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--osd") {
        if let Err(err) = osd::run() {
            eprintln!("osd failed: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }
    let control_panel = std::env::args().any(|arg| arg == "--control");
    let quick_settings = std::env::args().any(|arg| arg == "--quick");
    let quick_dev = std::env::args().any(|arg| arg == "--quick-dev");
    let debug_direct_fallback = std::env::args().any(|arg| arg == "--debug-direct-fallback");

    if quick_settings {
        let Some(_guard) = QuickInstanceGuard::acquire() else {
            return Ok(());
        };
        if !quick_dev {
            if let Err(err) = quick::run() {
                eprintln!("quick failed: {err}");
                std::process::exit(1);
            }
            return Ok(());
        }
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Hyprbole Quick Settings")
                .with_inner_size([420.0, 420.0])
                .with_min_inner_size([360.0, 360.0]),
            ..Default::default()
        };

        return eframe::run_native(
            "Hyprbole Quick Settings",
            options,
            Box::new(|cc| {
                install_style(&cc.egui_ctx);
                Ok(Box::new(QuickSettings::default()))
            }),
        );
    }

    if control_panel {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Hyprbole Control Center")
                .with_inner_size([900.0, 680.0])
                .with_min_inner_size([CONTROL_MIN_WIDTH, CONTROL_MIN_HEIGHT]),
            ..Default::default()
        };

        return eframe::run_native(
            "Hyprbole Control Center",
            options,
            Box::new(|cc| {
                install_style(&cc.egui_ctx);
                Ok(Box::new(ControlPanel::default()))
            }),
        );
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Hyprbole Shell Preview")
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([760.0, 520.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Hyprbole Shell Preview",
        options,
        Box::new(|cc| {
            install_style(&cc.egui_ctx);
            if debug_direct_fallback {
                eprintln!("debug direct fallback enabled for hyprbole-ui");
            }
            Ok(Box::new(ShellPreview::default()))
        }),
    )
}

struct QuickInstanceGuard {
    path: PathBuf,
}

impl QuickInstanceGuard {
    fn acquire() -> Option<Self> {
        let path = quick_instance_path()?;
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    let identity = quick_process_identity(std::process::id())?;
                    if file.write_all(identity.as_bytes()).is_ok() {
                        return Some(Self { path });
                    }
                    let _ = std::fs::remove_file(&path);
                    return None;
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    let Some(identity) = std::fs::read_to_string(&path)
                        .ok()
                        .and_then(|value| parse_quick_identity(&value))
                    else {
                        if std::fs::remove_file(&path).is_err() {
                            return None;
                        }
                        continue;
                    };
                    if quick_process_alive(identity.0)
                        && quick_process_identity(identity.0).as_deref() == Some(&identity.1)
                    {
                        return None;
                    }
                    if std::fs::remove_file(&path).is_err() {
                        return None;
                    }
                }
                Err(_) => return None,
            }
        }
    }
}

impl Drop for QuickInstanceGuard {
    fn drop(&mut self) {
        if let Ok(pid) = std::fs::read_to_string(&self.path) {
            if quick_process_identity(std::process::id()).as_deref() == Some(pid.trim()) {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }
}

fn quick_instance_path() -> Option<PathBuf> {
    let runtime = hyprbole_core::runtime::ensure_runtime_dir().ok()?;
    Some(runtime.join("quick.pid"))
}

fn quick_dismiss_path() -> Option<PathBuf> {
    let runtime = hyprbole_core::runtime::ensure_runtime_dir().ok()?;
    Some(runtime.join("quick.dismiss"))
}

fn process_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

fn quick_process_identity(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_name = stat.rsplit_once(") ")?.1;
    let fields = after_name.split_whitespace().collect::<Vec<_>>();
    let start_time = fields.get(19)?;
    Some(format!("{pid}:{start_time}"))
}

fn parse_quick_identity(value: &str) -> Option<(u32, String)> {
    let value = value.trim();
    let (pid, _start_time) = value.split_once(':')?;
    Some((pid.parse().ok()?, value.to_string()))
}

fn quick_process_alive(pid: u32) -> bool {
    process_alive(pid)
        && std::fs::read(format!("/proc/{pid}/cmdline"))
            .map(|cmdline| cmdline_is_quick_settings(&cmdline))
            .unwrap_or(false)
}

fn cmdline_is_quick_settings(cmdline: &[u8]) -> bool {
    let args = cmdline
        .split(|byte| *byte == 0)
        .filter_map(|arg| std::str::from_utf8(arg).ok())
        .collect::<Vec<_>>();
    args.first().is_some_and(|arg| arg.ends_with("hyprbole-ui")) && args.contains(&"--quick")
}

struct ControlPanel {
    state: ControlState,
    status: String,
    event_status: String,
    socket_path: String,
    last_refresh: Option<Instant>,
    pending: bool,
    event_refresh_requested: bool,
    pending_since: Option<Instant>,
    tx: Sender<ControlMessage>,
    rx: Receiver<ControlMessage>,
    next_request_id: u64,
    latest_request_id: u64,
    autostart_name: String,
    autostart_command: String,
    selected_section: ControlSection,
}

impl eframe::App for ControlPanel {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(message) = self.rx.try_recv() {
            match message {
                ControlMessage::Loaded { request_id, result } => {
                    if request_id != self.latest_request_id {
                        continue;
                    }
                    self.pending = false;
                    self.pending_since = None;
                    self.last_refresh = Some(Instant::now());
                    match result {
                        Ok(state) => {
                            let last_event_id = self.state.last_event_id;
                            self.state = state;
                            self.state.last_event_id = last_event_id;
                            self.status = "Connected to Hyprbole daemon.".to_string();
                        }
                        Err(err) => {
                            self.mark_disconnected();
                            self.status = err;
                        }
                    }
                }
                ControlMessage::Saved { request_id, result } => {
                    if request_id != self.latest_request_id {
                        continue;
                    }
                    self.pending = false;
                    self.pending_since = None;
                    self.last_refresh = Some(Instant::now());
                    match result {
                        Ok((state, message)) => {
                            let last_event_id = self.state.last_event_id;
                            self.state = state;
                            self.state.last_event_id = last_event_id;
                            self.status = message;
                        }
                        Err(err) => {
                            self.mark_disconnected();
                            self.status = err;
                        }
                    }
                }
                ControlMessage::EventStatus(mut status) => {
                    if !self.state.connected && status == "event stream connected" {
                        status = "event stream open; waiting for daemon state".to_string();
                    }
                    self.event_status = status;
                }
                ControlMessage::DaemonEvent { id, label } => {
                    self.state.last_event_id = Some(id);
                    self.event_status = format!("event #{id}: {label}");
                    if !self.pending {
                        self.spawn_load();
                    } else {
                        self.event_refresh_requested = true;
                    }
                }
            }
        }

        if self.event_refresh_requested && !self.pending {
            self.event_refresh_requested = false;
            self.spawn_load();
        }

        if self
            .last_refresh
            .is_none_or(|last| last.elapsed() > Duration::from_secs(4))
            && !self.pending
        {
            self.spawn_load();
        }

        if self
            .pending_since
            .is_some_and(|started| started.elapsed() > Duration::from_secs(10))
        {
            self.pending = false;
            self.pending_since = None;
            self.latest_request_id = self.next_request_id + 1;
            self.mark_disconnected();
            self.status = "Timed out waiting for daemon response.".to_string();
        }

        let mut ownership_change = None;
        let mut reconcile_request = None;
        let mut refresh_requested = false;
        let mut control_action = None;
        let mut autostart_save = None;
        let mut bar_setting_change = None;
        let mut theme_mode_change = None;
        let mut autostart_name = self.autostart_name.clone();
        let mut autostart_command = self.autostart_command.clone();
        let mut selected_section = self.selected_section;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Palette::BG))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                control_titlebar(ui, self, &mut refresh_requested);
                ui.add_space(8.0);
                ui.horizontal_top(|ui| {
                    control_sidebar(ui, &mut selected_section);
                    ui.separator();
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            control_section_view(
                                ui,
                                self,
                                selected_section,
                                &mut ownership_change,
                                &mut reconcile_request,
                                &mut control_action,
                                &mut autostart_name,
                                &mut autostart_command,
                                &mut autostart_save,
                                &mut bar_setting_change,
                                &mut theme_mode_change,
                            );
                            ui.add_space(12.0);
                            control_bottom_bar(ui, self);
                        });
                    });
                });
            });

        if refresh_requested {
            self.spawn_load();
        }
        if let Some((subsystem, mode)) = ownership_change {
            self.spawn_set_ownership(subsystem, mode);
        }
        if let Some(subsystem) = reconcile_request {
            self.spawn_reconcile(subsystem);
        }
        if let Some(action) = control_action {
            self.spawn_action(action);
        }
        self.autostart_name = autostart_name;
        self.autostart_command = autostart_command;
        self.selected_section = selected_section;
        if let Some(apps) = autostart_save {
            self.spawn_set_autostart(apps);
        }
        if let Some((field, value)) = bar_setting_change {
            self.spawn_set_bar_setting(field, value);
        }
        if let Some(mode) = theme_mode_change {
            self.spawn_set_theme_mode(mode);
        }

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

impl Default for ControlPanel {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        spawn_control_event_listener(tx.clone());
        let socket_path = hyprbole_core::daemon::socket_path_from_env()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|err| format!("unavailable: {err}"));
        Self {
            state: ControlState::default(),
            status: "Connecting to Hyprbole daemon...".to_string(),
            event_status: "event stream connecting".to_string(),
            socket_path,
            last_refresh: None,
            pending: false,
            event_refresh_requested: false,
            pending_since: None,
            tx,
            rx,
            next_request_id: 0,
            latest_request_id: 0,
            autostart_name: String::new(),
            autostart_command: String::new(),
            selected_section: ControlSection::Overview,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlSection {
    Overview,
    Workspaces,
    Keybindings,
    Layout,
    Monitors,
    Autostart,
    Bar,
    Appearance,
    Ownership,
    Diagnostics,
}

impl ControlPanel {
    fn begin_request(&mut self) -> u64 {
        self.next_request_id += 1;
        self.latest_request_id = self.next_request_id;
        self.latest_request_id
    }

    fn spawn_load(&mut self) {
        let request_id = self.begin_request();
        self.pending = true;
        self.pending_since = Some(Instant::now());
        let tx = self.tx.clone();
        thread::spawn(move || {
            let _ = tx.send(ControlMessage::Loaded {
                request_id,
                result: ControlState::load(),
            });
        });
    }

    fn spawn_set_ownership(&mut self, subsystem: ShellSubsystem, mode: OwnershipMode) {
        let request_id = self.begin_request();
        self.pending = true;
        self.pending_since = Some(Instant::now());
        self.status = format!("Setting {subsystem} ownership to {mode}...");
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = set_control_ownership(subsystem, mode);
            let _ = tx.send(ControlMessage::Saved { request_id, result });
        });
    }

    fn spawn_reconcile(&mut self, subsystem: Option<ShellSubsystem>) {
        let request_id = self.begin_request();
        self.pending = true;
        self.pending_since = Some(Instant::now());
        self.status = match subsystem {
            Some(subsystem) => format!("Reconciling {subsystem}..."),
            None => "Reconciling all supported subsystems...".to_string(),
        };
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = run_control_reconcile(subsystem);
            let _ = tx.send(ControlMessage::Saved { request_id, result });
        });
    }

    fn spawn_action(&mut self, action: ControlAction) {
        let request_id = self.begin_request();
        self.pending = true;
        self.pending_since = Some(Instant::now());
        self.status = action.pending_status();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = run_control_action(action);
            let _ = tx.send(ControlMessage::Saved { request_id, result });
        });
    }

    fn spawn_set_autostart(&mut self, apps: Vec<hyprbole_core::settings::AutostartApp>) {
        let request_id = self.begin_request();
        self.pending = true;
        self.pending_since = Some(Instant::now());
        self.status = "Saving autostart apps...".to_string();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = set_control_autostart(apps);
            let _ = tx.send(ControlMessage::Saved { request_id, result });
        });
    }

    fn spawn_set_bar_setting(&mut self, field: String, value: String) {
        let request_id = self.begin_request();
        self.pending = true;
        self.pending_since = Some(Instant::now());
        self.status = format!("Saving {field}...");
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = set_control_bar_setting(&field, &value);
            let _ = tx.send(ControlMessage::Saved { request_id, result });
        });
    }

    fn spawn_set_theme_mode(&mut self, mode: ThemeMode) {
        let request_id = self.begin_request();
        self.pending = true;
        self.pending_since = Some(Instant::now());
        self.status = format!("Saving theme mode {mode}...");
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = set_control_theme_mode(mode);
            let _ = tx.send(ControlMessage::Saved { request_id, result });
        });
    }

    fn mark_disconnected(&mut self) {
        let last_event_id = self.state.last_event_id;
        self.state = ControlState {
            last_event_id,
            ..ControlState::default()
        };
        self.event_status = "event stream disconnected".to_string();
    }
}

#[derive(Clone)]
struct ControlState {
    connected: bool,
    settings: ShellSettings,
    daemon_status: Option<hyprbole_core::daemon::DaemonStatus>,
    current_layout: Option<String>,
    workspaces: Vec<Workspace>,
    monitors: Vec<hyprbole_core::daemon::MonitorSnapshot>,
    keybindings: Vec<hyprbole_core::daemon::KeybindingSnapshot>,
    autostart_status: Vec<hyprbole_core::daemon::AutostartAppStatus>,
    audio_summary: String,
    audio_available: bool,
    brightness_summary: String,
    brightness_available: bool,
    notifications: hyprbole_core::notifications::NotificationSnapshot,
    last_reconcile_status: Option<String>,
    last_event_id: Option<u64>,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            connected: false,
            settings: ShellSettings::default(),
            daemon_status: None,
            current_layout: None,
            workspaces: Vec::new(),
            monitors: Vec::new(),
            keybindings: Vec::new(),
            autostart_status: Vec::new(),
            audio_summary: "Audio status unavailable".to_string(),
            audio_available: false,
            brightness_summary: "Brightness status unavailable".to_string(),
            brightness_available: false,
            notifications: hyprbole_core::notifications::NotificationSnapshot::default(),
            last_reconcile_status: None,
            last_event_id: None,
        }
    }
}

impl ControlState {
    fn load() -> Result<Self, String> {
        let client = hyprbole_core::daemon::DaemonClient::from_env()
            .map_err(|err| format!("Daemon unavailable: {err}"))?;
        let status = match client
            .send_shell(&hyprbole_core::daemon::ShellRequest::StatusGet)
            .map_err(|err| format!("Daemon request failed: {err}"))?
        {
            hyprbole_core::daemon::ShellResponse::Status { daemon } => Some(daemon),
            hyprbole_core::daemon::ShellResponse::Error { message } => return Err(message),
            _ => None,
        };
        let response = client
            .send_shell(&hyprbole_core::daemon::ShellRequest::StateGet)
            .map_err(|err| format!("Daemon request failed: {err}"))?;
        let keybindings = match client
            .send_shell(&hyprbole_core::daemon::ShellRequest::BindsGet)
            .map_err(|err| format!("Daemon request failed: {err}"))?
        {
            hyprbole_core::daemon::ShellResponse::Binds { keybindings } => keybindings,
            hyprbole_core::daemon::ShellResponse::Error { message } => return Err(message),
            _ => Vec::new(),
        };
        match response {
            hyprbole_core::daemon::ShellResponse::State { snapshot } => Ok(Self {
                connected: true,
                settings: snapshot.settings,
                autostart_status: status
                    .as_ref()
                    .map(|status| status.autostart.clone())
                    .unwrap_or_default(),
                daemon_status: status,
                current_layout: snapshot.current_layout,
                workspaces: snapshot
                    .workspaces
                    .into_iter()
                    .map(|workspace| Workspace {
                        id: workspace.id,
                        name: Some(workspace.name),
                        focused: workspace.focused,
                        active: workspace.active,
                        occupied: workspace.occupied,
                    })
                    .collect(),
                monitors: snapshot.monitors,
                keybindings,
                audio_summary: snapshot.audio_summary,
                audio_available: snapshot.audio.available,
                brightness_summary: snapshot.brightness_summary,
                brightness_available: snapshot.brightness.available,
                notifications: snapshot.notifications,
                last_reconcile_status: snapshot.last_reconcile_status,
                last_event_id: None,
            }),
            hyprbole_core::daemon::ShellResponse::Error { message } => Err(message),
            hyprbole_core::daemon::ShellResponse::Status { .. } => {
                Err("Daemon returned status instead of state.".to_string())
            }
            _ => Err("Daemon returned an unexpected response.".to_string()),
        }
    }

    fn socket_path(&self, fallback: &str) -> String {
        self.daemon_status
            .as_ref()
            .map(|status| status.socket_path.clone())
            .unwrap_or_else(|| fallback.to_string())
    }

    fn settings_path(&self) -> String {
        self.daemon_status
            .as_ref()
            .map(|status| status.settings_path.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn pid(&self) -> String {
        self.daemon_status
            .as_ref()
            .map(|status| status.pid.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn protocol_version(&self) -> String {
        self.daemon_status
            .as_ref()
            .map(|status| status.protocol_version.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn build_version(&self) -> String {
        self.daemon_status
            .as_ref()
            .map(|status| status.build_version.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn uptime(&self) -> String {
        self.daemon_status
            .as_ref()
            .map(|status| format!("{}s", status.uptime_seconds))
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn event_buffer(&self) -> String {
        self.daemon_status
            .as_ref()
            .map(|status| {
                format!(
                    "{} events; next #{}",
                    status.event_count, status.next_event_id
                )
            })
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn last_settings_save_status(&self) -> Option<String> {
        self.daemon_status
            .as_ref()
            .and_then(|status| status.last_settings_save_status.clone())
    }

    fn last_action_status(&self) -> Option<String> {
        let status = self.daemon_status.as_ref()?;
        if let Some(action) = &status.last_action {
            return Some(format!(
                "{} {}.{}: {}",
                if action.ok { "ok" } else { "error" },
                action.domain,
                action.name,
                action.message
            ));
        }
        status.last_action_status.clone()
    }

    fn reconcile_severity(&self) -> String {
        self.daemon_status
            .as_ref()
            .and_then(|status| status.reconcile.as_ref())
            .map(|reconcile| format!("{:?}", reconcile.severity).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn reconcile_rows(&self) -> Vec<(String, String)> {
        self.daemon_status
            .as_ref()
            .and_then(|status| status.reconcile.as_ref())
            .map(|reconcile| {
                reconcile
                    .results
                    .iter()
                    .map(|result| {
                        (
                            format!("{} reconcile", result.subsystem),
                            format!(
                                "{}{}: {}",
                                format!("{:?}", result.severity).to_lowercase(),
                                if result.implemented { "" } else { " reserved" },
                                result.message
                            ),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn recent_event_rows(&self) -> Vec<(String, String)> {
        self.daemon_status
            .as_ref()
            .map(|status| {
                status
                    .recent_events
                    .iter()
                    .take(5)
                    .map(|event| (format!("Event #{}", event.id), event_label(&event.event)))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn capability_rows(&self) -> Vec<(String, String)> {
        self.daemon_status
            .as_ref()
            .map(|status| {
                status
                    .ownership_capabilities
                    .iter()
                    .map(|capability| {
                        (
                            format!("{} capability", capability.subsystem),
                            format!(
                                "runtime={:?}; persisted={:?}",
                                capability.runtime_reconcile, capability.persisted_ownership
                            )
                            .to_lowercase(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn ownership_mode_supported(&self, subsystem: ShellSubsystem, mode: OwnershipMode) -> bool {
        if mode == OwnershipMode::RespectUserConfig {
            return true;
        }
        let support = self
            .daemon_status
            .as_ref()
            .and_then(|status| {
                status
                    .ownership_capabilities
                    .iter()
                    .find(|capability| capability.subsystem == subsystem)
            })
            .map(|capability| match mode {
                OwnershipMode::RespectUserConfig => {
                    hyprbole_core::daemon::CapabilitySupport::Implemented
                }
                OwnershipMode::RuntimeOwned => capability.runtime_reconcile,
                OwnershipMode::PersistedOwned => capability.persisted_ownership,
            });
        support.is_some_and(|support| {
            matches!(
                support,
                hyprbole_core::daemon::CapabilitySupport::Implemented
                    | hyprbole_core::daemon::CapabilitySupport::Partial
            )
        })
    }
}

enum ControlMessage {
    Loaded {
        request_id: u64,
        result: Result<ControlState, String>,
    },
    Saved {
        request_id: u64,
        result: Result<(ControlState, String), String>,
    },
    EventStatus(String),
    DaemonEvent {
        id: u64,
        label: String,
    },
}

#[derive(Clone)]
enum ControlAction {
    FocusWorkspace(String),
    SetLayout(String),
    SetMonitorMode {
        monitor: String,
        mode: String,
        position: String,
        scale: f32,
    },
    RunAutostart(String),
    VolumeUp,
    VolumeDown,
    MuteOutput,
    BrightnessUp,
    BrightnessDown,
    ToggleDnd,
    ClearNotifications,
    Lock,
}

impl ControlAction {
    fn pending_status(&self) -> String {
        match self {
            Self::FocusWorkspace(workspace) => format!("Focusing workspace {workspace}..."),
            Self::SetLayout(layout) => format!("Switching layout to {layout}..."),
            Self::SetMonitorMode { monitor, mode, .. } => {
                format!("Setting {monitor} to {mode}...")
            }
            Self::RunAutostart(name) => format!("Launching {name}..."),
            Self::VolumeUp => "Raising volume...".to_string(),
            Self::VolumeDown => "Lowering volume...".to_string(),
            Self::MuteOutput => "Toggling mute...".to_string(),
            Self::BrightnessUp => "Increasing brightness...".to_string(),
            Self::BrightnessDown => "Decreasing brightness...".to_string(),
            Self::ToggleDnd => "Toggling do-not-disturb...".to_string(),
            Self::ClearNotifications => "Clearing notification history...".to_string(),
            Self::Lock => "Requesting session lock...".to_string(),
        }
    }
}

struct QuickSettings {
    state: ControlState,
    status: String,
    pending: bool,
    last_refresh: Option<Instant>,
    tx: Sender<QuickMessage>,
    rx: Receiver<QuickMessage>,
}

enum QuickMessage {
    Loaded(Result<ControlState, String>),
    Action(Result<(ControlState, String), String>),
    DaemonEvent,
}

impl Default for QuickSettings {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        spawn_quick_event_listener(tx.clone());
        Self {
            state: ControlState::default(),
            status: "Connecting to Hyprbole daemon...".to_string(),
            pending: false,
            last_refresh: None,
            tx,
            rx,
        }
    }
}

impl eframe::App for QuickSettings {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if quick_dismiss_requested() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        while let Ok(message) = self.rx.try_recv() {
            match message {
                QuickMessage::Loaded(Ok(state)) => {
                    self.pending = false;
                    self.last_refresh = Some(Instant::now());
                    self.state = state;
                    self.status = "Connected to Hyprbole daemon.".to_string();
                }
                QuickMessage::Loaded(Err(err)) | QuickMessage::Action(Err(err)) => {
                    self.pending = false;
                    self.last_refresh = Some(Instant::now());
                    self.state = ControlState::default();
                    self.status = err;
                }
                QuickMessage::Action(Ok((state, message))) => {
                    self.pending = false;
                    self.last_refresh = Some(Instant::now());
                    self.state = state;
                    self.status = message;
                }
                QuickMessage::DaemonEvent => {
                    if !self.pending {
                        self.spawn_load();
                    }
                }
            }
        }

        if self
            .last_refresh
            .is_none_or(|last| last.elapsed() > Duration::from_secs(3))
            && !self.pending
        {
            self.spawn_load();
        }

        let mut action = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Palette::BG))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.heading(RichText::new("Quick Settings").color(Palette::TEXT));
                ui.label(muteda(&self.status));
                ui.add_space(10.0);
                native_group(ui, |ui| {
                    if let Some(action) = self.state.last_action_status() {
                        setting_row(ui, "Last action", &action);
                        ui.separator();
                    }
                    native_control_actions(
                        ui,
                        "Audio",
                        &self.state.audio_summary,
                        self.state.connected && self.state.audio_available && !self.pending,
                        &[
                            ("Vol -", ControlAction::VolumeDown),
                            ("Mute", ControlAction::MuteOutput),
                            ("Vol +", ControlAction::VolumeUp),
                        ],
                        &mut action,
                    );
                    ui.separator();
                    native_control_actions(
                        ui,
                        "Brightness",
                        &self.state.brightness_summary,
                        self.state.connected && self.state.brightness_available && !self.pending,
                        &[
                            ("Dim", ControlAction::BrightnessDown),
                            ("Brighten", ControlAction::BrightnessUp),
                        ],
                        &mut action,
                    );
                    ui.separator();
                    let notification_summary = format!(
                        "DND {}; {} unread",
                        if self.state.notifications.dnd_enabled {
                            "on"
                        } else {
                            "off"
                        },
                        self.state.notifications.unread_count
                    );
                    native_control_actions(
                        ui,
                        "Notifications",
                        &notification_summary,
                        self.state.connected && !self.pending,
                        &[("DND", ControlAction::ToggleDnd)],
                        &mut action,
                    );
                });
                native_group(ui, |ui| {
                    setting_row(
                        ui,
                        "Layout",
                        self.state.current_layout.as_deref().unwrap_or("unknown"),
                    );
                    ui.horizontal_wrapped(|ui| {
                        for layout in ["dwindle", "master", "scrolling"] {
                            let selected = self.state.current_layout.as_deref() == Some(layout);
                            if ui
                                .add_enabled(
                                    compositor_config_apply_enabled(
                                        self.state.connected,
                                        self.pending,
                                        selected,
                                    ),
                                    egui::Button::new(layout),
                                )
                                .clicked()
                            {
                                action = Some(ControlAction::SetLayout(layout.to_string()));
                            }
                        }
                    });
                    ui.separator();
                    let monitor_summary = self
                        .state
                        .monitors
                        .first()
                        .and_then(|monitor| monitor.current_mode.as_ref())
                        .map(|mode| mode.label.as_str())
                        .unwrap_or("no monitor mode");
                    setting_row(ui, "Display", monitor_summary);
                });
                native_group(ui, |ui| {
                    native_control_actions(
                        ui,
                        "Session",
                        "Lock the current login session",
                        self.state.connected && !self.pending,
                        &[("Lock", ControlAction::Lock)],
                        &mut action,
                    );
                });
            });

        if let Some(action) = action {
            self.spawn_action(action);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

fn quick_dismiss_requested() -> bool {
    let Some(path) = quick_dismiss_path() else {
        return false;
    };
    if !path.exists() {
        return false;
    }
    let requested_identity = std::fs::read_to_string(&path).ok();
    let _ = std::fs::remove_file(path);
    requested_identity.as_deref().map(str::trim)
        == quick_process_identity(std::process::id()).as_deref()
}

#[cfg(test)]
fn dismiss_request_targets_current_process(value: &str) -> bool {
    Some(value.trim()) == quick_process_identity(std::process::id()).as_deref()
}

impl QuickSettings {
    fn spawn_load(&mut self) {
        self.pending = true;
        let tx = self.tx.clone();
        thread::spawn(move || {
            let _ = tx.send(QuickMessage::Loaded(ControlState::load()));
        });
    }

    fn spawn_action(&mut self, action: ControlAction) {
        self.pending = true;
        self.status = action.pending_status();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let _ = tx.send(QuickMessage::Action(run_control_action(action)));
        });
    }
}

fn spawn_quick_event_listener(tx: Sender<QuickMessage>) {
    thread::spawn(move || {
        loop {
            let Ok(client) = hyprbole_core::daemon::DaemonClient::from_env() else {
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            let Ok(events) = client.follow_events(None) else {
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            for event in events {
                let Ok(event) = event else {
                    break;
                };
                if event.event.refreshes_snapshot() || event.event.refreshes_status() {
                    if tx.send(QuickMessage::DaemonEvent).is_err() {
                        return;
                    }
                }
            }
            thread::sleep(Duration::from_millis(250));
        }
    });
}

const CONTROL_SUBSYSTEMS: &[ShellSubsystem] = &[
    ShellSubsystem::Keybindings,
    ShellSubsystem::Monitors,
    ShellSubsystem::Workspaces,
    ShellSubsystem::Audio,
    ShellSubsystem::Brightness,
    ShellSubsystem::Appearance,
    ShellSubsystem::Notifications,
    ShellSubsystem::Power,
];

const OWNERSHIP_MODES: &[OwnershipMode] = &[
    OwnershipMode::RespectUserConfig,
    OwnershipMode::RuntimeOwned,
    OwnershipMode::PersistedOwned,
];

struct ShellPreview {
    snapshot: Snapshot,
    last_refresh: Option<Instant>,
    status: String,
    tx: Sender<AppMessage>,
    rx: Receiver<AppMessage>,
    refresh_pending: bool,
    action_pending: bool,
    pending_since: Option<Instant>,
    event_refresh_requested: bool,
    last_event_refresh: Option<Instant>,
    next_request_id: u64,
    latest_request_id: u64,
}

impl eframe::App for ShellPreview {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(message) = self.rx.try_recv() {
            match message {
                AppMessage::Snapshot {
                    request_id,
                    snapshot,
                } => {
                    if request_id != self.latest_request_id {
                        continue;
                    }
                    self.snapshot = snapshot;
                    self.last_refresh = Some(Instant::now());
                    self.refresh_pending = false;
                    self.pending_since = None;
                }
                AppMessage::ActionDone {
                    request_id,
                    status,
                    snapshot,
                } => {
                    if request_id != self.latest_request_id {
                        continue;
                    }
                    self.status = status;
                    self.snapshot = snapshot;
                    self.last_refresh = Some(Instant::now());
                    self.refresh_pending = false;
                    self.action_pending = false;
                    self.pending_since = None;
                }
                AppMessage::DaemonEvent => {
                    self.event_refresh_requested = true;
                }
            }
        }

        if self.event_refresh_requested
            && !self.refresh_pending
            && !self.action_pending
            && self
                .last_event_refresh
                .is_none_or(|last| last.elapsed() > Duration::from_millis(150))
        {
            self.event_refresh_requested = false;
            self.last_event_refresh = Some(Instant::now());
            self.spawn_refresh();
        }

        if self
            .last_refresh
            .is_none_or(|last| last.elapsed() > Duration::from_secs(3))
            && !self.refresh_pending
            && !self.action_pending
        {
            self.spawn_refresh();
        }

        if self
            .pending_since
            .is_some_and(|started| started.elapsed() > Duration::from_secs(10))
        {
            self.refresh_pending = false;
            self.action_pending = false;
            self.pending_since = None;
            self.latest_request_id = self.next_request_id + 1;
            self.status = "Timed out waiting for daemon response.".to_string();
        }

        let mut pending_action = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Palette::BG))
            .show(ctx, |ui| {
                ui.add_space(18.0);
                header(ui, &self.snapshot, &mut pending_action);
                ui.add_space(18.0);

                egui::Grid::new("shell-grid")
                    .num_columns(if ui.available_width() > 860.0 { 2 } else { 1 })
                    .spacing([18.0, 18.0])
                    .show(ui, |ui| {
                        shell_card(ui, "Workspaces", "Hyprland compositor facade", |ui| {
                            workspace_strip(ui, &self.snapshot.workspaces, &mut pending_action);
                            ui.add_space(10.0);
                            ui.label(muteda(format!(
                                "{} windows tracked on {} monitor{}{}",
                                self.snapshot.windows.len(),
                                self.snapshot.monitor_count,
                                if self.snapshot.monitor_count == 1 {
                                    ""
                                } else {
                                    "s"
                                },
                                self.snapshot
                                    .windows
                                    .first()
                                    .map(|window| format!("; first: {}", window.title))
                                    .unwrap_or_default()
                            )));
                        });

                        shell_card(ui, "Audio & Brightness", "System module targets", |ui| {
                            ui.label(RichText::new(&self.snapshot.audio_summary).strong());
                            ui.label(muteda(&self.snapshot.brightness_summary));
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                quick_action(
                                    ui,
                                    "Volume -",
                                    true,
                                    UiAction::VolumeDown,
                                    &mut pending_action,
                                );
                                quick_action(
                                    ui,
                                    "Volume +",
                                    true,
                                    UiAction::VolumeUp,
                                    &mut pending_action,
                                );
                                quick_action(
                                    ui,
                                    "Mute",
                                    true,
                                    UiAction::MuteOutput,
                                    &mut pending_action,
                                );
                            });
                        });
                        ui.end_row();

                        shell_card(
                            ui,
                            "Control Center",
                            "Quick status and typed actions",
                            |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    quick_action(
                                        ui,
                                        "Lock",
                                        true,
                                        UiAction::Lock,
                                        &mut pending_action,
                                    );
                                    quick_action(
                                        ui,
                                        "Refresh",
                                        true,
                                        UiAction::Refresh,
                                        &mut pending_action,
                                    );
                                    quick_badge(ui, "Wi-Fi", true);
                                    quick_badge(ui, "Bluetooth", false);
                                    quick_badge(ui, "Dark", true);
                                });
                            },
                        );

                        shell_card(ui, "Settings", "Persistent shell-owned state", |ui| {
                            setting_row(ui, "Renderer", "temporary egui prototype");
                            setting_row(ui, "IPC", "Hyprbole daemon JSON protocol");
                            setting_row(ui, "Keybindings", &self.snapshot.keybindings_ownership);
                            setting_row(ui, "Hyprland config", "user-owned by default");
                            setting_row(ui, "Migration", "no Waybar replacement yet");
                        });
                        ui.end_row();
                    });

                ui.add_space(18.0);
                bottom_bar(ui, &self.snapshot, &self.status);
            });

        if let Some(action) = pending_action {
            self.spawn_action(action);
        }

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

impl Default for ShellPreview {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        spawn_daemon_event_listener(tx.clone());
        Self {
            snapshot: Snapshot::default(),
            last_refresh: None,
            status: "Ready. Click a workspace or audio/session action.".to_string(),
            tx,
            rx,
            refresh_pending: false,
            action_pending: false,
            pending_since: None,
            event_refresh_requested: false,
            last_event_refresh: None,
            next_request_id: 0,
            latest_request_id: 0,
        }
    }
}

fn spawn_daemon_event_listener(tx: Sender<AppMessage>) {
    thread::spawn(move || {
        loop {
            let Ok(client) = hyprbole_core::daemon::DaemonClient::from_env() else {
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            let Ok(events) = client.follow_events(None) else {
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            for event in events {
                let Ok(event) = event else {
                    break;
                };
                if event.event.refreshes_snapshot() || event.event.refreshes_status() {
                    if tx.send(AppMessage::DaemonEvent).is_err() {
                        return;
                    }
                }
            }
            thread::sleep(Duration::from_millis(250));
        }
    });
}

fn spawn_control_event_listener(tx: Sender<ControlMessage>) {
    thread::spawn(move || {
        loop {
            let client = match hyprbole_core::daemon::DaemonClient::from_env() {
                Ok(client) => client,
                Err(err) => {
                    if tx
                        .send(ControlMessage::EventStatus(format!(
                            "event stream waiting: {err}"
                        )))
                        .is_err()
                    {
                        return;
                    }
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };
            let events = match client.follow_events(None) {
                Ok(events) => events,
                Err(err) => {
                    if tx
                        .send(ControlMessage::EventStatus(format!(
                            "event stream reconnecting: {err}"
                        )))
                        .is_err()
                    {
                        return;
                    }
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };
            if tx
                .send(ControlMessage::EventStatus(
                    "event stream connected".to_string(),
                ))
                .is_err()
            {
                return;
            }
            for event in events {
                match event {
                    Ok(event) => {
                        if !event.event.refreshes_snapshot() && !event.event.refreshes_status() {
                            continue;
                        }
                        let label = event_label(&event.event);
                        if tx
                            .send(ControlMessage::DaemonEvent {
                                id: event.id,
                                label,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(err) => {
                        if tx
                            .send(ControlMessage::EventStatus(format!(
                                "event stream reconnecting: {err}"
                            )))
                            .is_err()
                        {
                            return;
                        }
                        break;
                    }
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}

fn event_label(event: &hyprbole_core::daemon::ShellEvent) -> String {
    match event {
        hyprbole_core::daemon::ShellEvent::Gap { after, oldest } => {
            format!("gap after {after}; oldest {oldest}")
        }
        hyprbole_core::daemon::ShellEvent::Hyprland { raw } => format!("hyprland {raw}"),
        hyprbole_core::daemon::ShellEvent::Reconcile { status } => {
            format!("reconcile {status}")
        }
        hyprbole_core::daemon::ShellEvent::Action { status } => {
            format!(
                "action {}.{} {}",
                status.domain, status.name, status.message
            )
        }
        hyprbole_core::daemon::ShellEvent::StateChanged { reason, domain } => {
            if let Some(domain) = domain {
                format!("state changed {domain}.{reason}")
            } else {
                format!("state changed {reason}")
            }
        }
    }
}

impl ShellPreview {
    fn begin_request(&mut self) -> u64 {
        self.next_request_id += 1;
        self.latest_request_id = self.next_request_id;
        self.latest_request_id
    }

    fn spawn_refresh(&mut self) {
        let request_id = self.begin_request();
        self.refresh_pending = true;
        self.pending_since = Some(Instant::now());
        let tx = self.tx.clone();
        thread::spawn(move || {
            let _ = tx.send(AppMessage::Snapshot {
                request_id,
                snapshot: Snapshot::load(),
            });
        });
    }

    fn spawn_action(&mut self, action: UiAction) {
        if self.action_pending {
            self.status = "Action already running.".to_string();
            return;
        }

        self.action_pending = true;
        self.pending_since = Some(Instant::now());
        self.status = action.pending_status();
        let request_id = self.begin_request();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let status = action.run();
            let snapshot = Snapshot::load();
            let _ = tx.send(AppMessage::ActionDone {
                request_id,
                status,
                snapshot,
            });
        });
    }
}

fn install_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.window_corner_radius = CornerRadius::same(18);
    style.visuals.panel_fill = Palette::BG;
    style.visuals.widgets.noninteractive.bg_fill = Palette::SURFACE;
    style.visuals.widgets.inactive.bg_fill = Palette::SURFACE_STRONG;
    style.visuals.widgets.hovered.bg_fill = Palette::ACCENT_DIM;
    style.visuals.widgets.active.bg_fill = Palette::ACCENT;
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    ctx.set_style(style);
}

fn header(ui: &mut egui::Ui, snapshot: &Snapshot, pending_action: &mut Option<UiAction>) {
    egui::Frame::new()
        .fill(Palette::SURFACE)
        .stroke(Stroke::new(1.0, Palette::BORDER))
        .corner_radius(CornerRadius::same(26))
        .inner_margin(Margin::same(24))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Hyprbole Shell")
                            .font(FontId::proportional(34.0))
                            .strong()
                            .color(Palette::TEXT),
                    );
                    ui.label(muteda(
                        "Rust shell prototype: service/module/widget split, compositor-first state, typed actions next.",
                    ));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Refresh").clicked() {
                        *pending_action = Some(UiAction::Refresh);
                    }

                    let status = if snapshot.hyprland_available {
                        "Hyprland connected"
                    } else {
                        "Hyprland unavailable"
                    };
                    ui.label(
                        RichText::new(status)
                            .color(if snapshot.hyprland_available {
                                Palette::SUCCESS
                            } else {
                                Palette::WARNING
                            })
                            .strong(),
                    );
                });
            });
        });
}

fn control_titlebar(ui: &mut egui::Ui, panel: &ControlPanel, refresh_requested: &mut bool) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Settings").color(Palette::TEXT));
        ui.label(muteda("Hyprbole shell"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    control_refresh_enabled(panel.pending),
                    egui::Button::new("Refresh"),
                )
                .clicked()
            {
                *refresh_requested = true;
            }
            let status = if panel.state.connected {
                "Connected"
            } else {
                "Offline - refresh to retry"
            };
            ui.label(
                RichText::new(status)
                    .strong()
                    .color(if panel.state.connected {
                        Palette::SUCCESS
                    } else {
                        Palette::WARNING
                    }),
            );
        });
    });
}

fn control_refresh_enabled(pending: bool) -> bool {
    !pending
}

#[cfg(test)]
fn control_minimum_layout_fits() -> bool {
    CONTROL_SIDEBAR_WIDTH + CONTROL_PANE_MIN_WIDTH + CONTROL_LAYOUT_GUTTER <= CONTROL_MIN_WIDTH
}

fn control_sidebar(ui: &mut egui::Ui, selected: &mut ControlSection) {
    ui.vertical(|ui| {
        ui.set_width(CONTROL_SIDEBAR_WIDTH);
        ui.add_space(8.0);
        for (section, label) in [
            (ControlSection::Overview, "Overview"),
            (ControlSection::Workspaces, "Workspaces"),
            (ControlSection::Keybindings, "Keybindings"),
            (ControlSection::Layout, "Tiling Layout"),
            (ControlSection::Monitors, "Displays"),
            (ControlSection::Autostart, "Startup Apps"),
            (ControlSection::Bar, "Bar"),
            (ControlSection::Appearance, "Appearance"),
            (ControlSection::Ownership, "Ownership"),
            (ControlSection::Diagnostics, "Diagnostics"),
        ] {
            let response = ui.selectable_label(*selected == section, label);
            if response.clicked() {
                *selected = section;
            }
        }
    });
}

fn control_section_view(
    ui: &mut egui::Ui,
    panel: &ControlPanel,
    section: ControlSection,
    ownership_change: &mut Option<(ShellSubsystem, OwnershipMode)>,
    reconcile_request: &mut Option<Option<ShellSubsystem>>,
    control_action: &mut Option<ControlAction>,
    autostart_name: &mut String,
    autostart_command: &mut String,
    autostart_save: &mut Option<Vec<hyprbole_core::settings::AutostartApp>>,
    bar_setting_change: &mut Option<(String, String)>,
    theme_mode_change: &mut Option<ThemeMode>,
) {
    ui.set_min_width(CONTROL_PANE_MIN_WIDTH);
    match section {
        ControlSection::Overview => {
            native_overview_pane(ui, panel, reconcile_request, control_action)
        }
        ControlSection::Workspaces => native_workspaces_pane(ui, panel, control_action),
        ControlSection::Keybindings => {
            native_keybindings_pane(ui, panel, ownership_change, reconcile_request)
        }
        ControlSection::Layout => native_layout_pane(ui, panel, control_action),
        ControlSection::Monitors => native_monitors_pane(ui, panel, control_action),
        ControlSection::Autostart => native_autostart_pane(
            ui,
            panel,
            autostart_name,
            autostart_command,
            autostart_save,
            control_action,
        ),
        ControlSection::Bar => native_bar_pane(ui, panel, bar_setting_change),
        ControlSection::Appearance => native_appearance_pane(ui, panel, theme_mode_change),
        ControlSection::Ownership => {
            native_ownership_pane(ui, panel, ownership_change, reconcile_request)
        }
        ControlSection::Diagnostics => native_diagnostics_pane(ui, panel),
    }
}

fn native_pane_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add_space(8.0);
    ui.heading(RichText::new(title).color(Palette::TEXT));
    ui.label(muteda(subtitle));
    ui.add_space(12.0);
}

fn native_group(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Palette::SURFACE)
        .stroke(Stroke::new(1.0, Palette::BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, body);
    ui.add_space(12.0);
}

fn native_overview_pane(
    ui: &mut egui::Ui,
    panel: &ControlPanel,
    reconcile_request: &mut Option<Option<ShellSubsystem>>,
    control_action: &mut Option<ControlAction>,
) {
    native_pane_header(ui, "Overview", "Session-local Hyprbole service state.");
    if !panel.state.connected {
        native_group(ui, |ui| {
            ui.label(
                RichText::new("Daemon offline")
                    .strong()
                    .color(Palette::WARNING),
            );
            ui.label(muteda(
                "The control panel keeps a manual Refresh path available so you can retry without closing this window.",
            ));
        });
    }
    native_group(ui, |ui| {
        setting_row(
            ui,
            "Daemon",
            if panel.state.connected {
                "connected"
            } else {
                "offline"
            },
        );
        setting_row(ui, "Reconcile", &panel.state.reconcile_severity());
        setting_row(ui, "Events", &panel.state.event_buffer());
        setting_row(ui, "Uptime", &panel.state.uptime());
        if ui
            .add_enabled(
                panel.state.connected && !panel.pending,
                egui::Button::new("Reconcile all"),
            )
            .clicked()
        {
            *reconcile_request = Some(None);
        }
    });
    native_group(ui, |ui| {
        native_control_actions(
            ui,
            "Audio",
            &panel.state.audio_summary,
            panel.state.audio_available && panel.state.connected && !panel.pending,
            &[
                ("Vol -", ControlAction::VolumeDown),
                ("Mute", ControlAction::MuteOutput),
                ("Vol +", ControlAction::VolumeUp),
            ],
            control_action,
        );
        ui.separator();
        native_control_actions(
            ui,
            "Brightness",
            &panel.state.brightness_summary,
            panel.state.brightness_available && panel.state.connected && !panel.pending,
            &[
                ("Dim", ControlAction::BrightnessDown),
                ("Brighten", ControlAction::BrightnessUp),
            ],
            control_action,
        );
        ui.separator();
        let notification_summary = format!(
            "DND {}; {} unread",
            if panel.state.notifications.dnd_enabled {
                "on"
            } else {
                "off"
            },
            panel.state.notifications.unread_count
        );
        native_control_actions(
            ui,
            "Notifications",
            &notification_summary,
            panel.state.connected && !panel.pending,
            &[
                ("DND", ControlAction::ToggleDnd),
                ("Clear", ControlAction::ClearNotifications),
            ],
            control_action,
        );
        ui.separator();
        native_control_actions(
            ui,
            "Session",
            "Lock the current login session",
            panel.state.connected && !panel.pending,
            &[("Lock", ControlAction::Lock)],
            control_action,
        );
    });
}

fn native_control_actions(
    ui: &mut egui::Ui,
    title: &str,
    summary: &str,
    enabled: bool,
    actions: &[(&str, ControlAction)],
    control_action: &mut Option<ControlAction>,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).strong().color(Palette::TEXT));
            ui.label(muteda(summary));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (label, action) in actions.iter().rev() {
                if ui.add_enabled(enabled, egui::Button::new(*label)).clicked() {
                    *control_action = Some(action.clone());
                }
            }
        });
    });
}

fn workspace_state_label(workspace: &Workspace) -> &'static str {
    if workspace.focused {
        "focused"
    } else if workspace.occupied {
        "occupied"
    } else {
        "empty"
    }
}

fn workspace_state_color(workspace: &Workspace) -> Color32 {
    if workspace.focused {
        Palette::SUCCESS
    } else if workspace.occupied {
        Palette::ACCENT
    } else {
        Palette::MUTED
    }
}

fn monitor_visible_mode_count(total: usize) -> usize {
    total.min(CONTROL_MONITOR_MODE_LIMIT)
}

fn compositor_config_apply_enabled(connected: bool, pending: bool, selected: bool) -> bool {
    COMPOSITOR_CONFIG_APPLY_SUPPORTED && connected && !pending && !selected
}

fn monitor_mode_is_current(
    monitor: &hyprbole_core::daemon::MonitorSnapshot,
    mode: &hyprbole_core::daemon::MonitorModeSnapshot,
) -> bool {
    monitor
        .current_mode
        .as_ref()
        .is_some_and(|current| current.label == mode.label)
}

fn visible_monitor_modes(
    monitor: &hyprbole_core::daemon::MonitorSnapshot,
) -> Vec<&hyprbole_core::daemon::MonitorModeSnapshot> {
    let mut modes = monitor
        .available_modes
        .iter()
        .take(CONTROL_MONITOR_MODE_LIMIT)
        .collect::<Vec<_>>();
    if let Some(current) = &monitor.current_mode {
        let already_visible = modes.iter().any(|mode| mode.label == current.label);
        if !already_visible {
            if let Some(mode) = monitor
                .available_modes
                .iter()
                .find(|mode| mode.label == current.label)
            {
                modes.push(mode);
            }
        }
    }
    modes
}

fn autostart_validation_message(
    apps: &[hyprbole_core::settings::AutostartApp],
    name: &str,
    command: &str,
) -> Option<&'static str> {
    let name = name.trim();
    let command = command.trim();
    if name.is_empty() || command.is_empty() {
        return None;
    }
    if apps.iter().any(|app| app.name.trim() == name) {
        return Some("A startup app with this name already exists.");
    }
    if apps.iter().any(|app| app.command.trim() == command) {
        return Some("A startup app with this command already exists.");
    }
    None
}

fn native_workspaces_pane(
    ui: &mut egui::Ui,
    panel: &ControlPanel,
    control_action: &mut Option<ControlAction>,
) {
    native_pane_header(ui, "Workspaces", "Focus normalized Hyprland workspaces.");
    native_group(ui, |ui| {
        if panel.state.workspaces.is_empty() {
            ui.label(muteda("No workspace state is available yet."));
            ui.label(muteda(
                "Start the daemon or refresh after Hyprland emits workspace state.",
            ));
            return;
        }
        for workspace in &panel.state.workspaces {
            ui.horizontal(|ui| {
                let label = workspace.name.as_deref().unwrap_or("workspace");
                ui.label(RichText::new(label).strong().color(Palette::TEXT));
                let state = workspace_state_label(workspace);
                ui.label(RichText::new(state).color(workspace_state_color(workspace)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            panel.state.connected && !panel.pending && !workspace.focused,
                            egui::Button::new("Focus"),
                        )
                        .clicked()
                    {
                        *control_action = Some(ControlAction::FocusWorkspace(label.to_string()));
                    }
                });
            });
            ui.separator();
        }
    });
}

fn native_keybindings_pane(
    ui: &mut egui::Ui,
    panel: &ControlPanel,
    ownership_change: &mut Option<(ShellSubsystem, OwnershipMode)>,
    reconcile_request: &mut Option<Option<ShellSubsystem>>,
) {
    native_pane_header(ui, "Keybindings", "Runtime bind health and ownership.");
    native_group(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            reconcile_button(
                ui,
                ShellSubsystem::Keybindings,
                panel.pending,
                panel.state.connected,
                reconcile_request,
            );
            if ui
                .add_enabled(
                    panel.state.connected && !panel.pending,
                    egui::Button::new("Runtime owned"),
                )
                .clicked()
            {
                *ownership_change =
                    Some((ShellSubsystem::Keybindings, OwnershipMode::RuntimeOwned));
            }
            if ui
                .add_enabled(
                    panel.state.connected && !panel.pending,
                    egui::Button::new("Release binds"),
                )
                .clicked()
            {
                *ownership_change = Some((
                    ShellSubsystem::Keybindings,
                    OwnershipMode::RespectUserConfig,
                ));
            }
        });
        ui.separator();
        if panel.state.keybindings.is_empty() {
            ui.label(muteda("No keybinding state is available yet."));
            return;
        }
        for bind in &panel.state.keybindings {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&bind.description)
                        .strong()
                        .color(Palette::TEXT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(muteda(&bind.expected));
                    ui.label(
                        RichText::new(if bind.active { "active" } else { "missing" }).color(
                            if bind.active {
                                Palette::SUCCESS
                            } else {
                                Palette::WARNING
                            },
                        ),
                    );
                });
            });
        }
    });
}

fn native_layout_pane(
    ui: &mut egui::Ui,
    panel: &ControlPanel,
    control_action: &mut Option<ControlAction>,
) {
    native_pane_header(ui, "Tiling Layout", "Switch Hyprland runtime layout.");
    native_group(ui, |ui| {
        setting_row(
            ui,
            "Current layout",
            panel.state.current_layout.as_deref().unwrap_or("unknown"),
        );
        ui.label(muteda(
            "Applies runtime layout state through daemon IPC; it does not persist Hyprland config.",
        ));
        ui.separator();
        for (label, layout) in [
            ("Dwindle", "dwindle"),
            ("Master", "master"),
            ("Scrolling", "scrolling"),
        ] {
            let selected = panel
                .state
                .current_layout
                .as_deref()
                .is_some_and(|current| current == layout);
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).strong().color(Palette::TEXT));
                if selected {
                    ui.label(RichText::new("current").color(Palette::SUCCESS));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            compositor_config_apply_enabled(
                                panel.state.connected,
                                panel.pending,
                                selected,
                            ),
                            egui::Button::new("Apply"),
                        )
                        .clicked()
                    {
                        *control_action = Some(ControlAction::SetLayout(layout.to_string()));
                    }
                });
            });
            ui.separator();
        }
    });
}

fn native_monitors_pane(
    ui: &mut egui::Ui,
    panel: &ControlPanel,
    control_action: &mut Option<ControlAction>,
) {
    native_pane_header(ui, "Displays", "Resolution and refresh-rate selection.");
    if panel.state.monitors.is_empty() {
        native_group(ui, |ui| {
            ui.label(muteda("No monitor state is available."));
            ui.label(muteda(
                "Refresh after the daemon receives Hyprland monitor state.",
            ));
        });
        return;
    }
    for monitor in &panel.state.monitors {
        native_group(ui, |ui| {
            ui.label(RichText::new(&monitor.name).strong().color(Palette::TEXT));
            ui.label(muteda(format!(
                "active workspace: {}",
                monitor.active_workspace.as_deref().unwrap_or("none")
            )));
            ui.label(muteda(format!(
                "current mode: {}",
                monitor
                    .current_mode
                    .as_ref()
                    .map(|mode| mode.label.as_str())
                    .unwrap_or("unknown")
            )));
            ui.label(muteda(format!(
                "geometry: position {}, scale {:.2}",
                if monitor.position.is_empty() {
                    "unknown"
                } else {
                    &monitor.position
                },
                monitor.scale
            )));
            if monitor.focused {
                ui.label(RichText::new("focused display").color(Palette::SUCCESS));
            }
            ui.separator();
            if monitor.available_modes.is_empty() {
                ui.label(muteda("No available modes were reported."));
                return;
            }
            ui.label(muteda(format!(
                "Showing {} of {} reported modes.",
                monitor_visible_mode_count(monitor.available_modes.len()),
                monitor.available_modes.len()
            )));
            ui.label(muteda("Applies runtime monitor mode through daemon IPC while preserving current position/scale; it does not persist Hyprland config."));
            for mode in visible_monitor_modes(monitor) {
                let selected = monitor_mode_is_current(monitor, mode);
                ui.horizontal(|ui| {
                    ui.label(&mode.label);
                    if selected {
                        ui.label(RichText::new("current").color(Palette::SUCCESS));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                compositor_config_apply_enabled(
                                    panel.state.connected,
                                    panel.pending,
                                    selected,
                                ),
                                egui::Button::new("Apply"),
                            )
                            .clicked()
                        {
                            *control_action = Some(ControlAction::SetMonitorMode {
                                monitor: monitor.name.clone(),
                                mode: mode.label.clone(),
                                position: monitor.position.clone(),
                                scale: monitor.scale,
                            });
                        }
                    });
                });
            }
        });
    }
}

fn native_autostart_pane(
    ui: &mut egui::Ui,
    panel: &ControlPanel,
    autostart_name: &mut String,
    autostart_command: &mut String,
    autostart_save: &mut Option<Vec<hyprbole_core::settings::AutostartApp>>,
    control_action: &mut Option<ControlAction>,
) {
    native_pane_header(ui, "Startup Apps", "Hyprbole-managed session startup list.");
    native_group(ui, |ui| {
        let mut apps = panel.state.settings.autostart.apps.clone();
        let mut remove_index = None;
        if apps.is_empty() {
            ui.label(muteda("No startup apps are managed by Hyprbole yet."));
            ui.separator();
        }
        for index in 0..apps.len() {
            let app_name = apps[index].name.clone();
            let app_command = apps[index].command.clone();
            let app_enabled = apps[index].enabled;
            let app_status = panel
                .state
                .autostart_status
                .iter()
                .find(|status| status.name == app_name)
                .map(|status| {
                    format!(
                        "{}{}",
                        if status.running { "running" } else { "stopped" },
                        status
                            .last_status
                            .as_ref()
                            .map(|last| format!("; {last}"))
                            .unwrap_or_default()
                    )
                });
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&app_name).strong().color(Palette::TEXT));
                    ui.label(muteda(&app_command));
                    ui.label(
                        RichText::new(if app_enabled { "enabled" } else { "disabled" }).color(
                            if app_enabled {
                                Palette::SUCCESS
                            } else {
                                Palette::MUTED
                            },
                        ),
                    );
                    if let Some(status) = &app_status {
                        ui.label(muteda(status));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            panel.state.connected && !panel.pending,
                            egui::Button::new("Remove"),
                        )
                        .clicked()
                    {
                        remove_index = Some(index);
                    }
                    if ui
                        .add_enabled(
                            panel.state.connected && !panel.pending,
                            egui::Button::new("Run now"),
                        )
                        .clicked()
                    {
                        *control_action = Some(ControlAction::RunAutostart(app_name.clone()));
                    }
                    if ui
                        .add_enabled(
                            panel.state.connected && !panel.pending,
                            egui::Button::new(if app_enabled { "Disable" } else { "Enable" }),
                        )
                        .clicked()
                    {
                        apps[index].enabled = !apps[index].enabled;
                        *autostart_save = Some(apps.clone());
                    }
                });
            });
            ui.separator();
        }
        if let Some(index) = remove_index {
            apps.remove(index);
            *autostart_save = Some(apps.clone());
        }
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(autostart_name);
        });
        ui.horizontal(|ui| {
            ui.label("Command");
            ui.text_edit_singleline(autostart_command);
        });
        let validation = autostart_validation_message(&apps, autostart_name, autostart_command);
        if let Some(message) = validation {
            ui.label(RichText::new(message).color(Palette::WARNING));
        }
        if ui
            .add_enabled(
                panel.state.connected
                    && !panel.pending
                    && validation.is_none()
                    && !autostart_name.trim().is_empty()
                    && !autostart_command.trim().is_empty(),
                egui::Button::new("Add Startup App"),
            )
            .clicked()
        {
            apps.push(hyprbole_core::settings::AutostartApp {
                name: autostart_name.trim().to_string(),
                command: autostart_command.trim().to_string(),
                enabled: true,
            });
            *autostart_name = String::new();
            *autostart_command = String::new();
            *autostart_save = Some(apps);
        }
    });
}

fn native_bar_pane(
    ui: &mut egui::Ui,
    panel: &ControlPanel,
    bar_setting_change: &mut Option<(String, String)>,
) {
    native_pane_header(
        ui,
        "Bar",
        "Persisted Hyprbole-owned layout for the native layer-shell bar.",
    );
    let bar = &panel.state.settings.ui.bar;
    native_group(ui, |ui| {
        setting_row(ui, "Enabled", if bar.enabled { "yes" } else { "no" });
        setting_row(ui, "Monitor", &bar.monitor.to_string());
        setting_row(ui, "Edge", &bar.edge.to_string());
        setting_row(ui, "Height", &format!("{} px", bar.height));
        setting_row(ui, "Margin", &format!("{} px", bar.margin));
        setting_row(ui, "Padding", &format!("{} px", bar.padding));
        setting_row(
            ui,
            "Widgets",
            &bar.widgets
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        );
    });
    native_group(ui, |ui| {
        ui.label(RichText::new("Surface").strong().color(Palette::TEXT));
        ui.horizontal_wrapped(|ui| {
            for (label, value) in bar_monitor_choices(panel) {
                let selected = bar.monitor.to_string() == value;
                if ui
                    .add_enabled(
                        panel.state.connected && !panel.pending && !selected,
                        egui::Button::new(label),
                    )
                    .clicked()
                {
                    *bar_setting_change = Some(("bar.monitor".to_string(), value));
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [("Enable", "on"), ("Disable", "off")] {
                let selected = bar.enabled == (value == "on");
                if ui
                    .add_enabled(
                        panel.state.connected && !panel.pending && !selected,
                        egui::Button::new(label),
                    )
                    .clicked()
                {
                    *bar_setting_change = Some(("bar.enabled".to_string(), value.to_string()));
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for edge in ["top", "bottom"] {
                let selected = bar.edge.to_string() == edge;
                if ui
                    .add_enabled(
                        panel.state.connected && !panel.pending && !selected,
                        egui::Button::new(edge),
                    )
                    .clicked()
                {
                    *bar_setting_change = Some(("bar.edge".to_string(), edge.to_string()));
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for height in [28_u32, 36, 44, 56] {
                let selected = bar.height == height;
                if ui
                    .add_enabled(
                        panel.state.connected && !panel.pending && !selected,
                        egui::Button::new(format!("{height}px")),
                    )
                    .clicked()
                {
                    *bar_setting_change = Some(("bar.height".to_string(), height.to_string()));
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for margin in [0_u32, 2, 4, 8, 12] {
                let selected = bar.margin == margin;
                if ui
                    .add_enabled(
                        panel.state.connected && !panel.pending && !selected,
                        egui::Button::new(format!("margin {margin}px")),
                    )
                    .clicked()
                {
                    *bar_setting_change = Some(("bar.margin".to_string(), margin.to_string()));
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for padding in [8_u32, 12, 16, 24, 32] {
                let selected = bar.padding == padding;
                if ui
                    .add_enabled(
                        panel.state.connected && !panel.pending && !selected,
                        egui::Button::new(format!("padding {padding}px")),
                    )
                    .clicked()
                {
                    *bar_setting_change = Some(("bar.padding".to_string(), padding.to_string()));
                }
            }
        });
    });
    native_group(ui, |ui| {
        ui.label(RichText::new("Widgets").strong().color(Palette::TEXT));
        if ui
            .add_enabled(
                panel.state.connected
                    && !panel.pending
                    && bar.widgets != BarSettings::DEFAULT_WIDGETS,
                egui::Button::new("Reset widget order"),
            )
            .clicked()
        {
            let mut updated = bar.clone();
            updated.reset_widgets();
            *bar_setting_change = Some(("bar.widgets".to_string(), updated.widgets_csv()));
        }
        for widget in [
            BarWidget::Workspaces,
            BarWidget::FocusedWindow,
            BarWidget::DaemonStatus,
            BarWidget::Audio,
            BarWidget::Brightness,
            BarWidget::Layout,
            BarWidget::Notifications,
            BarWidget::Clock,
            BarWidget::QuickToggle,
        ] {
            let enabled = bar.widget_enabled(widget);
            let label = widget.to_string();
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{}{}",
                    if enabled {
                        format!(
                            "{}.",
                            bar.widgets
                                .iter()
                                .position(|existing| *existing == widget)
                                .unwrap_or(0)
                                + 1
                        )
                    } else {
                        "-".to_string()
                    },
                    label
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            panel.state.connected && !panel.pending,
                            egui::Button::new(if enabled { "Hide" } else { "Show" }),
                        )
                        .clicked()
                    {
                        let mut widgets = bar.widgets.clone();
                        if enabled {
                            widgets.retain(|existing| *existing != widget);
                        } else {
                            widgets.push(widget);
                        }
                        if !widgets.is_empty() {
                            let updated = BarSettings {
                                widgets,
                                ..bar.clone()
                            };
                            *bar_setting_change =
                                Some(("bar.widgets".to_string(), updated.widgets_csv()));
                        }
                    }
                    if ui
                        .add_enabled(
                            panel.state.connected && !panel.pending && enabled,
                            egui::Button::new("Down"),
                        )
                        .clicked()
                    {
                        let mut updated = bar.clone();
                        if updated.move_widget(widget, WidgetMove::Down) {
                            *bar_setting_change =
                                Some(("bar.widgets".to_string(), updated.widgets_csv()));
                        }
                    }
                    if ui
                        .add_enabled(
                            panel.state.connected && !panel.pending && enabled,
                            egui::Button::new("Up"),
                        )
                        .clicked()
                    {
                        let mut updated = bar.clone();
                        if updated.move_widget(widget, WidgetMove::Up) {
                            *bar_setting_change =
                                Some(("bar.widgets".to_string(), updated.widgets_csv()));
                        }
                    }
                });
            });
        }
    });
}

fn bar_monitor_choices(panel: &ControlPanel) -> Vec<(String, String)> {
    let mut choices = vec![
        ("focused".to_string(), BarMonitor::Focused.to_string()),
        ("primary".to_string(), BarMonitor::Primary.to_string()),
    ];
    for monitor in &panel.state.monitors {
        choices.push((monitor.name.clone(), monitor.name.clone()));
    }
    choices.sort_by(|left, right| left.0.cmp(&right.0));
    choices.dedup_by(|left, right| left.1 == right.1);
    choices
}

fn native_appearance_pane(
    ui: &mut egui::Ui,
    panel: &ControlPanel,
    theme_mode_change: &mut Option<ThemeMode>,
) {
    native_pane_header(
        ui,
        "Appearance",
        "Hyprbole-owned theme mode and built-in shell tokens.",
    );
    let theme = hyprbole_core::theme::snapshot(&panel.state.settings.theme);
    native_group(ui, |ui| {
        setting_row(ui, "Mode", &theme.mode.to_string());
        setting_row(ui, "Effective", &theme.effective_mode.to_string());
        setting_row(ui, "Background", &theme.tokens.colors.background);
        setting_row(ui, "Surface", &theme.tokens.colors.surface);
        setting_row(ui, "Accent", &theme.tokens.colors.accent);
        setting_row(
            ui,
            "Bar density",
            &format!("{} px", theme.tokens.density.bar_height),
        );
    });
    native_group(ui, |ui| {
        ui.label(RichText::new("Theme Mode").strong().color(Palette::TEXT));
        ui.horizontal_wrapped(|ui| {
            for (label, mode) in [
                ("System", ThemeMode::System),
                ("Light", ThemeMode::Light),
                ("Dark", ThemeMode::Dark),
            ] {
                let selected = panel.state.settings.theme.mode == mode;
                if ui
                    .add_enabled(
                        panel.state.connected && !panel.pending && !selected,
                        egui::Button::new(label),
                    )
                    .clicked()
                {
                    *theme_mode_change = Some(mode);
                }
            }
        });
    });
}

fn native_ownership_pane(
    ui: &mut egui::Ui,
    panel: &ControlPanel,
    ownership_change: &mut Option<(ShellSubsystem, OwnershipMode)>,
    reconcile_request: &mut Option<Option<ShellSubsystem>>,
) {
    native_pane_header(ui, "Ownership", "Per-subsystem shell control mode.");
    native_group(ui, |ui| {
        for subsystem in CONTROL_SUBSYSTEMS {
            ownership_row(
                ui,
                &panel.state,
                subsystem,
                panel.state.settings.ownership(*subsystem),
                panel.pending,
                ownership_change,
            );
            reconcile_button(
                ui,
                *subsystem,
                panel.pending,
                panel.state.connected,
                reconcile_request,
            );
            ui.separator();
        }
    });
}

fn native_diagnostics_pane(ui: &mut egui::Ui, panel: &ControlPanel) {
    native_pane_header(ui, "Diagnostics", "Daemon details and recent events.");
    native_group(ui, |ui| {
        control_daemon_rows(ui, panel);
        ui.separator();
        for row in panel.state.reconcile_rows() {
            setting_row(ui, &row.0, &row.1);
        }
        for row in panel.state.recent_event_rows() {
            setting_row(ui, &row.0, &row.1);
        }
        for row in panel.state.capability_rows() {
            setting_row(ui, &row.0, &row.1);
        }
    });
}

fn control_daemon_rows(ui: &mut egui::Ui, panel: &ControlPanel) {
    setting_row(
        ui,
        "Status",
        if panel.state.connected {
            "connected"
        } else {
            "offline"
        },
    );
    setting_row(ui, "PID", &panel.state.pid());
    setting_row(ui, "Uptime", &panel.state.uptime());
    setting_row(ui, "Protocol", &panel.state.protocol_version());
    setting_row(ui, "Build", &panel.state.build_version());
    setting_row(ui, "Events", &panel.event_status);
    setting_row(ui, "Event buffer", &panel.state.event_buffer());
    setting_row(ui, "Socket", &panel.state.socket_path(&panel.socket_path));
    setting_row(ui, "Settings", &panel.state.settings_path());
    if let Some(status) = &panel.state.last_reconcile_status {
        setting_row(ui, "Reconcile", status);
    }
    if let Some(status) = panel.state.last_settings_save_status() {
        setting_row(ui, "Settings save", &status);
    }
    if let Some(status) = panel.state.last_action_status() {
        setting_row(ui, "Last action", &status);
    }
}

fn shell_card(ui: &mut egui::Ui, title: &str, subtitle: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Palette::SURFACE)
        .stroke(Stroke::new(1.0, Palette::BORDER))
        .corner_radius(CornerRadius::same(22))
        .inner_margin(Margin::same(20))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(360.0, 170.0));
            ui.label(
                RichText::new(title)
                    .font(FontId::proportional(24.0))
                    .strong()
                    .color(Palette::TEXT),
            );
            ui.label(muteda(subtitle));
            ui.add_space(16.0);
            body(ui);
        });
}

fn workspace_strip(
    ui: &mut egui::Ui,
    workspaces: &[Workspace],
    pending_action: &mut Option<UiAction>,
) {
    ui.horizontal_wrapped(|ui| {
        if workspaces.is_empty() {
            ui.label(muteda("No Hyprland workspace data yet."));
            return;
        }

        for workspace in workspaces.iter().take(12) {
            let fill = if workspace.focused {
                Palette::ACCENT
            } else if workspace.active {
                Palette::ACCENT_DIM
            } else {
                Palette::SURFACE_STRONG
            };

            let label = workspace.name.as_deref().unwrap_or("workspace");
            if ui
                .add(
                    egui::Button::new(RichText::new(label).color(Palette::TEXT).strong())
                        .fill(fill)
                        .corner_radius(CornerRadius::same(14))
                        .min_size(Vec2::new(48.0, 34.0)),
                )
                .clicked()
            {
                *pending_action = Some(UiAction::FocusWorkspace(label.to_string()));
            }
        }
    });
}

fn quick_badge(ui: &mut egui::Ui, label: &str, enabled: bool) {
    let fill = if enabled {
        Palette::ACCENT_DIM
    } else {
        Palette::SURFACE_STRONG
    };

    egui::Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(255))
        .inner_margin(Margin::symmetric(14, 8))
        .show(ui, |ui| {
            ui.label(RichText::new(label).strong().color(Palette::TEXT));
        });
}

fn quick_action(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    action: UiAction,
    pending_action: &mut Option<UiAction>,
) {
    let fill = if enabled {
        Palette::ACCENT_DIM
    } else {
        Palette::SURFACE_STRONG
    };

    let response = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).strong().color(Palette::TEXT))
            .fill(fill)
            .corner_radius(CornerRadius::same(255))
            .min_size(Vec2::new(74.0, 34.0)),
    );

    if response.clicked() {
        *pending_action = Some(action);
    }
}

fn setting_row(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(key).strong().color(Palette::TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(muteda(value));
        });
    });
}

fn ownership_row(
    ui: &mut egui::Ui,
    state: &ControlState,
    subsystem: &ShellSubsystem,
    current: OwnershipMode,
    pending: bool,
    ownership_change: &mut Option<(ShellSubsystem, OwnershipMode)>,
) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(subsystem.to_string())
                .strong()
                .color(Palette::TEXT),
        );
        ui.horizontal_wrapped(|ui| {
            for mode in OWNERSHIP_MODES {
                let selected = *mode == current;
                let supported = ownership_choice_supported(state, *subsystem, *mode);
                let fill = if selected {
                    Palette::ACCENT
                } else {
                    Palette::SURFACE_STRONG
                };
                let response = ui.add_enabled(
                    state.connected && !pending && !selected && supported,
                    egui::Button::new(
                        RichText::new(control_mode_label(*mode))
                            .strong()
                            .color(Palette::TEXT),
                    )
                    .fill(fill)
                    .corner_radius(CornerRadius::same(255)),
                );
                if response.clicked() {
                    *ownership_change = Some((*subsystem, *mode));
                }
            }
        });
        ui.label(muteda(control_mode_hint(current)));
        if !state.ownership_mode_supported(*subsystem, OwnershipMode::RuntimeOwned) {
            ui.label(muteda(
                "Runtime/persisted ownership is reserved here; reconciliation is observe-only.",
            ));
        }
    });
}

fn reconcile_button(
    ui: &mut egui::Ui,
    subsystem: ShellSubsystem,
    pending: bool,
    connected: bool,
    reconcile_request: &mut Option<Option<ShellSubsystem>>,
) {
    let enabled = connected
        && !pending
        && matches!(
            subsystem,
            ShellSubsystem::Keybindings | ShellSubsystem::Monitors | ShellSubsystem::Workspaces
        );
    if ui
        .add_enabled(
            enabled,
            egui::Button::new(RichText::new("Reconcile").color(Palette::TEXT))
                .fill(Palette::SURFACE_STRONG)
                .corner_radius(CornerRadius::same(255)),
        )
        .clicked()
    {
        *reconcile_request = Some(Some(subsystem));
    }
}

fn ownership_choice_supported(
    state: &ControlState,
    subsystem: ShellSubsystem,
    mode: OwnershipMode,
) -> bool {
    state.ownership_mode_supported(subsystem, mode)
}

fn control_mode_label(mode: OwnershipMode) -> &'static str {
    match mode {
        OwnershipMode::RespectUserConfig => "Respect",
        OwnershipMode::RuntimeOwned => "Runtime",
        OwnershipMode::PersistedOwned => "Persisted",
    }
}

fn control_mode_hint(mode: OwnershipMode) -> &'static str {
    match mode {
        OwnershipMode::RespectUserConfig => "Observe only; user config remains authoritative.",
        OwnershipMode::RuntimeOwned => "Hyprbole may apply runtime state through IPC.",
        OwnershipMode::PersistedOwned => "Reserved for Hyprbole-owned persisted settings.",
    }
}

fn bottom_bar(ui: &mut egui::Ui, snapshot: &Snapshot, status: &str) {
    egui::Frame::new()
        .fill(Palette::SURFACE_STRONG)
        .corner_radius(CornerRadius::same(18))
        .inner_margin(Margin::symmetric(18, 12))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(muteda("Prototype command:"));
                ui.monospace("hyprbole-ui");
                ui.separator();
                ui.label(muteda(format!("Active window: {}", snapshot.active_window)));
                if let Some(event) = &snapshot.last_hyprland_event {
                    ui.separator();
                    ui.label(muteda(format!("Last event: {event}")));
                }
                ui.separator();
                if let Some(status) = &snapshot.last_reconcile_status {
                    ui.label(muteda(format!("Reconcile: {status}")));
                    ui.separator();
                }
                ui.label(RichText::new(status).color(status_color(status)));
            });
        });
}

fn control_bottom_bar(ui: &mut egui::Ui, panel: &ControlPanel) {
    egui::Frame::new()
        .fill(Palette::SURFACE_STRONG)
        .corner_radius(CornerRadius::same(18))
        .inner_margin(Margin::symmetric(18, 12))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(muteda("Command:"));
                ui.monospace("hbctl control");
                ui.separator();
                ui.label(RichText::new(&panel.status).color(status_color(&panel.status)));
                if panel.pending {
                    ui.separator();
                    ui.label(muteda("request pending"));
                }
            });
        });
}

fn status_color(status: &str) -> Color32 {
    let status = status.to_ascii_lowercase();
    if status.contains("error")
        || status.contains("failed")
        || status.contains("failure")
        || status.contains("exited with")
        || status.contains("unavailable")
        || status.contains("timed out")
    {
        Palette::WARNING
    } else {
        Palette::SUCCESS
    }
}

fn set_control_ownership(
    subsystem: ShellSubsystem,
    mode: OwnershipMode,
) -> Result<(ControlState, String), String> {
    let client = hyprbole_core::daemon::DaemonClient::from_env()
        .map_err(|err| format!("Daemon unavailable: {err}"))?;
    let response = client
        .send_shell(&hyprbole_core::daemon::ShellRequest::SettingsSetOwnership { subsystem, mode })
        .map_err(|err| format!("Daemon request failed: {err}"))?;
    let message = match response {
        hyprbole_core::daemon::ShellResponse::Ok { message } => message,
        hyprbole_core::daemon::ShellResponse::Error { message } => return Err(message),
        hyprbole_core::daemon::ShellResponse::Status { .. } => "Updated status.".to_string(),
        _ => "Updated ownership.".to_string(),
    };
    let state = ControlState::load()?;
    Ok((state, message))
}

fn run_control_reconcile(
    subsystem: Option<ShellSubsystem>,
) -> Result<(ControlState, String), String> {
    let client = hyprbole_core::daemon::DaemonClient::from_env()
        .map_err(|err| format!("Daemon unavailable: {err}"))?;
    let response = client
        .send_shell(&hyprbole_core::daemon::ShellRequest::Reconcile { subsystem })
        .map_err(|err| format!("Daemon request failed: {err}"))?;
    let message = match response {
        hyprbole_core::daemon::ShellResponse::Ok { message } => message,
        hyprbole_core::daemon::ShellResponse::Error { message } => return Err(message),
        _ => "Reconciled state.".to_string(),
    };
    let state = ControlState::load()?;
    Ok((state, message))
}

fn run_control_action(action: ControlAction) -> Result<(ControlState, String), String> {
    let shell_action = match action {
        ControlAction::FocusWorkspace(workspace) => {
            hyprbole_core::daemon::ShellAction::Compositor {
                action: hyprbole_core::compositor::CompositorAction::FocusWorkspace {
                    name: workspace,
                },
            }
        }
        ControlAction::SetLayout(name) => hyprbole_core::daemon::ShellAction::Compositor {
            action: hyprbole_core::compositor::CompositorAction::SetLayout { name },
        },
        ControlAction::SetMonitorMode {
            monitor,
            mode,
            position,
            scale,
        } => hyprbole_core::daemon::ShellAction::Compositor {
            action: hyprbole_core::compositor::CompositorAction::SetMonitorMode {
                monitor,
                mode,
                position,
                scale,
            },
        },
        ControlAction::RunAutostart(name) => {
            return run_control_autostart(&name);
        }
        ControlAction::VolumeUp => hyprbole_core::daemon::ShellAction::Audio {
            action: hyprbole_core::audio::AudioAction::VolumeUp,
        },
        ControlAction::VolumeDown => hyprbole_core::daemon::ShellAction::Audio {
            action: hyprbole_core::audio::AudioAction::VolumeDown,
        },
        ControlAction::MuteOutput => hyprbole_core::daemon::ShellAction::Audio {
            action: hyprbole_core::audio::AudioAction::ToggleMute,
        },
        ControlAction::BrightnessUp => hyprbole_core::daemon::ShellAction::Brightness {
            action: hyprbole_core::brightness::BrightnessAction::Increase,
        },
        ControlAction::BrightnessDown => hyprbole_core::daemon::ShellAction::Brightness {
            action: hyprbole_core::brightness::BrightnessAction::Decrease,
        },
        ControlAction::ToggleDnd => hyprbole_core::daemon::ShellAction::Notifications {
            action: hyprbole_core::notifications::NotificationAction::ToggleDnd,
        },
        ControlAction::ClearNotifications => hyprbole_core::daemon::ShellAction::Notifications {
            action: hyprbole_core::notifications::NotificationAction::ClearHistory,
        },
        ControlAction::Lock => hyprbole_core::daemon::ShellAction::SessionLock,
    };
    let client = hyprbole_core::daemon::DaemonClient::from_env()
        .map_err(|err| format!("Daemon unavailable: {err}"))?;
    let response = client
        .send_shell(&hyprbole_core::daemon::ShellRequest::ActionCall {
            action: shell_action,
        })
        .map_err(|err| format!("Daemon request failed: {err}"))?;
    let message = match response {
        hyprbole_core::daemon::ShellResponse::Ok { message } => message,
        hyprbole_core::daemon::ShellResponse::Error { message } => return Err(message),
        _ => "Action completed.".to_string(),
    };
    let state = ControlState::load()?;
    Ok((state, message))
}

fn run_control_autostart(name: &str) -> Result<(ControlState, String), String> {
    let client = hyprbole_core::daemon::DaemonClient::from_env()
        .map_err(|err| format!("Daemon unavailable: {err}"))?;
    let response = client
        .send_shell(&hyprbole_core::daemon::ShellRequest::AutostartRun {
            name: name.to_string(),
        })
        .map_err(|err| format!("Daemon request failed: {err}"))?;
    let message = match response {
        hyprbole_core::daemon::ShellResponse::Ok { message } => message,
        hyprbole_core::daemon::ShellResponse::Error { message } => return Err(message),
        _ => "Launched autostart app.".to_string(),
    };
    let state = ControlState::load()?;
    Ok((state, message))
}

fn set_control_autostart(
    apps: Vec<hyprbole_core::settings::AutostartApp>,
) -> Result<(ControlState, String), String> {
    let client = hyprbole_core::daemon::DaemonClient::from_env()
        .map_err(|err| format!("Daemon unavailable: {err}"))?;
    let response = client
        .send_shell(&hyprbole_core::daemon::ShellRequest::SettingsSetAutostart { apps })
        .map_err(|err| format!("Daemon request failed: {err}"))?;
    let message = match response {
        hyprbole_core::daemon::ShellResponse::Ok { message } => message,
        hyprbole_core::daemon::ShellResponse::Error { message } => return Err(message),
        _ => "Saved autostart settings.".to_string(),
    };
    let state = ControlState::load()?;
    Ok((state, message))
}

fn set_control_bar_setting(field: &str, value: &str) -> Result<(ControlState, String), String> {
    let client = hyprbole_core::daemon::DaemonClient::from_env()
        .map_err(|err| format!("Daemon unavailable: {err}"))?;
    let response = client
        .send_shell(&hyprbole_core::daemon::ShellRequest::BarSettingsSetField {
            field: field.to_string(),
            value: value.to_string(),
        })
        .map_err(|err| format!("Daemon request failed: {err}"))?;
    let message = match response {
        hyprbole_core::daemon::ShellResponse::Ok { message } => message,
        hyprbole_core::daemon::ShellResponse::Error { message } => return Err(message),
        _ => "Saved bar settings.".to_string(),
    };
    let state = ControlState::load()?;
    Ok((state, message))
}

fn set_control_theme_mode(mode: ThemeMode) -> Result<(ControlState, String), String> {
    let client = hyprbole_core::daemon::DaemonClient::from_env()
        .map_err(|err| format!("Daemon unavailable: {err}"))?;
    let response = client
        .send_shell(&hyprbole_core::daemon::ShellRequest::ThemeSetMode { mode })
        .map_err(|err| format!("Daemon request failed: {err}"))?;
    let message = match response {
        hyprbole_core::daemon::ShellResponse::Ok { message } => message,
        hyprbole_core::daemon::ShellResponse::Error { message } => return Err(message),
        _ => "Saved theme settings.".to_string(),
    };
    let state = ControlState::load()?;
    Ok((state, message))
}

fn muteda(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).color(Palette::MUTED)
}

#[derive(Default)]
struct Snapshot {
    hyprland_available: bool,
    workspaces: Vec<Workspace>,
    windows: Vec<Window>,
    monitor_count: usize,
    active_window: String,
    audio_summary: String,
    brightness_summary: String,
    last_hyprland_event: Option<String>,
    last_reconcile_status: Option<String>,
    keybindings_ownership: String,
}

impl Snapshot {
    fn load() -> Self {
        if let Some(snapshot) = load_daemon_snapshot() {
            return snapshot;
        }

        if !debug_direct_fallback_enabled() {
            return Self {
                audio_summary: "Daemon unavailable".to_string(),
                brightness_summary: "Daemon unavailable".to_string(),
                keybindings_ownership: hyprbole_core::settings::load_or_default()
                    .map(|settings| settings.ownership.keybindings.to_string())
                    .unwrap_or_else(|_| "unknown".to_string()),
                ..Self::default()
            };
        }

        let workspaces = load_workspaces().unwrap_or_default();
        let windows = load_windows().unwrap_or_default();
        let monitor_count = load_monitor_count().unwrap_or(0);
        let active_window = load_active_window().unwrap_or_else(|| "unavailable".to_string());
        let audio_summary =
            load_audio_summary().unwrap_or_else(|| "Audio status unavailable".to_string());
        let brightness_summary = "Brightness status unavailable".to_string();
        let keybindings_ownership = hyprbole_core::settings::load_or_default()
            .map(|settings| settings.ownership.keybindings.to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            hyprland_available: !workspaces.is_empty() || monitor_count > 0,
            workspaces,
            windows,
            monitor_count,
            active_window,
            audio_summary,
            brightness_summary,
            last_hyprland_event: None,
            last_reconcile_status: None,
            keybindings_ownership,
        }
    }
}

fn load_daemon_snapshot() -> Option<Snapshot> {
    let response = hyprbole_core::daemon::DaemonClient::from_env()
        .ok()?
        .send_shell(&hyprbole_core::daemon::ShellRequest::StateGet)
        .ok()?;
    let hyprbole_core::daemon::ShellResponse::State { snapshot } = response else {
        return None;
    };
    Some(Snapshot {
        hyprland_available: snapshot.hyprland_available,
        workspaces: snapshot
            .workspaces
            .into_iter()
            .map(|workspace| Workspace {
                id: workspace.id,
                name: Some(workspace.name),
                focused: workspace.focused,
                active: workspace.active,
                occupied: workspace.occupied,
            })
            .collect(),
        windows: snapshot
            .windows
            .into_iter()
            .map(|window| Window {
                title: window.title,
            })
            .collect(),
        monitor_count: snapshot.monitor_count,
        active_window: snapshot.active_window,
        audio_summary: snapshot.audio_summary,
        brightness_summary: snapshot.brightness_summary,
        last_hyprland_event: snapshot.last_hyprland_event,
        last_reconcile_status: snapshot.last_reconcile_status,
        keybindings_ownership: snapshot.settings.ownership.keybindings.to_string(),
    })
}

#[derive(Clone, Deserialize)]
struct Workspace {
    id: i64,
    name: Option<String>,
    #[serde(default)]
    focused: bool,
    #[serde(default)]
    active: bool,
    #[serde(default)]
    occupied: bool,
}

#[derive(Clone, Deserialize)]
struct Window {
    #[serde(default)]
    title: String,
}

#[derive(Deserialize)]
struct Monitor {
    #[serde(default, rename = "activeWorkspace")]
    active_workspace: Option<MonitorWorkspace>,
}

#[derive(Deserialize)]
struct MonitorWorkspace {
    id: i64,
}

#[derive(Deserialize)]
struct ActiveWindow {
    #[serde(default)]
    title: String,
    #[serde(default, rename = "class")]
    app_class: String,
}

fn load_workspaces() -> Option<Vec<Workspace>> {
    let mut workspaces: Vec<Workspace> =
        serde_json::from_str(&command_output("hyprctl", &["workspaces", "-j"])?).ok()?;
    let active_ids = load_active_workspace_ids();
    let focused_id = load_focused_workspace_id();
    for workspace in &mut workspaces {
        workspace.active = active_ids.contains(&workspace.id);
        workspace.focused = focused_id == Some(workspace.id);
    }
    workspaces.sort_by_key(|workspace| workspace.id);
    Some(workspaces)
}

fn load_focused_workspace_id() -> Option<i64> {
    let workspace: MonitorWorkspace =
        serde_json::from_str(&command_output("hyprctl", &["activeworkspace", "-j"])?).ok()?;
    Some(workspace.id)
}

fn load_active_workspace_ids() -> Vec<i64> {
    let Some(output) = command_output("hyprctl", &["monitors", "-j"]) else {
        return Vec::new();
    };
    let Ok(monitors) = serde_json::from_str::<Vec<Monitor>>(&output) else {
        return Vec::new();
    };
    monitors
        .into_iter()
        .filter_map(|monitor| monitor.active_workspace.map(|workspace| workspace.id))
        .collect()
}

fn load_windows() -> Option<Vec<Window>> {
    serde_json::from_str(&command_output("hyprctl", &["clients", "-j"])?).ok()
}

fn load_monitor_count() -> Option<usize> {
    let monitors: Vec<Monitor> =
        serde_json::from_str(&command_output("hyprctl", &["monitors", "-j"])?).ok()?;
    Some(monitors.len())
}

fn load_active_window() -> Option<String> {
    let active: ActiveWindow =
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

fn load_audio_summary() -> Option<String> {
    let output = command_output("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?
        .trim()
        .to_string();
    Some(output)
}

fn command_output(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

#[derive(Clone)]
enum UiAction {
    Refresh,
    FocusWorkspace(String),
    VolumeUp,
    VolumeDown,
    MuteOutput,
    Lock,
}

impl UiAction {
    fn pending_status(&self) -> String {
        match self {
            UiAction::Refresh => "Refreshing shell state...".to_string(),
            UiAction::FocusWorkspace(workspace) => format!("Focusing workspace {workspace}..."),
            UiAction::VolumeUp => "Raising output volume...".to_string(),
            UiAction::VolumeDown => "Lowering output volume...".to_string(),
            UiAction::MuteOutput => "Toggling output mute...".to_string(),
            UiAction::Lock => "Requesting session lock...".to_string(),
        }
    }

    fn run(self) -> String {
        if let Some(status) = self.run_daemon() {
            return status;
        }

        if !debug_direct_fallback_enabled() {
            return "Daemon unavailable; action fallback disabled. Start hyprbole-daemon or run UI with --debug-direct-fallback.".to_string();
        }

        match self {
            UiAction::Refresh => "Refreshed shell state.".to_string(),
            UiAction::FocusWorkspace(workspace) => run_status(
                "hyprctl",
                &["dispatch", "workspace", &workspace],
                format!("Focused workspace {workspace}."),
            ),
            UiAction::VolumeUp => run_status(
                "wpctl",
                &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"],
                "Raised output volume.".to_string(),
            ),
            UiAction::VolumeDown => run_status(
                "wpctl",
                &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"],
                "Lowered output volume.".to_string(),
            ),
            UiAction::MuteOutput => run_status(
                "wpctl",
                &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"],
                "Toggled output mute.".to_string(),
            ),
            UiAction::Lock => run_status(
                "loginctl",
                &["lock-session"],
                "Requested session lock.".to_string(),
            ),
        }
    }

    fn run_daemon(&self) -> Option<String> {
        let action = match self {
            UiAction::Refresh => return Some("Refreshed shell state.".to_string()),
            UiAction::FocusWorkspace(workspace) => hyprbole_core::daemon::ShellAction::Compositor {
                action: hyprbole_core::compositor::CompositorAction::FocusWorkspace {
                    name: workspace.clone(),
                },
            },
            UiAction::VolumeUp => hyprbole_core::daemon::ShellAction::Audio {
                action: hyprbole_core::audio::AudioAction::VolumeUp,
            },
            UiAction::VolumeDown => hyprbole_core::daemon::ShellAction::Audio {
                action: hyprbole_core::audio::AudioAction::VolumeDown,
            },
            UiAction::MuteOutput => hyprbole_core::daemon::ShellAction::Audio {
                action: hyprbole_core::audio::AudioAction::ToggleMute,
            },
            UiAction::Lock => hyprbole_core::daemon::ShellAction::SessionLock,
        };
        let response = hyprbole_core::daemon::DaemonClient::from_env()
            .ok()?
            .send_shell(&hyprbole_core::daemon::ShellRequest::ActionCall { action })
            .ok()?;
        match response {
            hyprbole_core::daemon::ShellResponse::Ok { message } => Some(message),
            hyprbole_core::daemon::ShellResponse::Error { message } => Some(message),
            hyprbole_core::daemon::ShellResponse::State { .. } => {
                Some("Updated state.".to_string())
            }
            hyprbole_core::daemon::ShellResponse::Settings { .. } => {
                Some("Updated settings.".to_string())
            }
            hyprbole_core::daemon::ShellResponse::UiSettings { .. } => {
                Some("Updated UI settings.".to_string())
            }
            hyprbole_core::daemon::ShellResponse::Theme { .. } => {
                Some("Updated theme.".to_string())
            }
            hyprbole_core::daemon::ShellResponse::Binds { .. } => {
                Some("Updated keybindings.".to_string())
            }
            hyprbole_core::daemon::ShellResponse::Notifications { .. } => {
                Some("Updated notifications.".to_string())
            }
            hyprbole_core::daemon::ShellResponse::Events { .. } => {
                Some("Updated events.".to_string())
            }
            hyprbole_core::daemon::ShellResponse::Status { .. } => {
                Some("Updated status.".to_string())
            }
        }
    }
}

fn debug_direct_fallback_enabled() -> bool {
    std::env::args().any(|arg| arg == "--debug-direct-fallback")
}

enum AppMessage {
    Snapshot {
        request_id: u64,
        snapshot: Snapshot,
    },
    ActionDone {
        request_id: u64,
        status: String,
        snapshot: Snapshot,
    },
    DaemonEvent,
}

fn run_status(command: &str, args: &[&str], success: String) -> String {
    match Command::new(command).args(args).output() {
        Ok(output) if output.status.success() => success,
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                format!("{command} exited with {}", output.status)
            } else {
                format!("{command} failed: {stderr}")
            }
        }
        Err(err) => format!("Failed to run {command}: {err}"),
    }
}

struct Palette;

impl Palette {
    const BG: Color32 = Color32::from_rgb(9, 11, 16);
    const SURFACE: Color32 = Color32::from_rgb(22, 25, 34);
    const SURFACE_STRONG: Color32 = Color32::from_rgb(31, 36, 48);
    const BORDER: Color32 = Color32::from_rgb(58, 68, 88);
    const TEXT: Color32 = Color32::from_rgb(236, 241, 249);
    const MUTED: Color32 = Color32::from_rgb(148, 159, 180);
    const ACCENT: Color32 = Color32::from_rgb(139, 170, 255);
    const ACCENT_DIM: Color32 = Color32::from_rgb(52, 66, 112);
    const SUCCESS: Color32 = Color32::from_rgb(126, 231, 135);
    const WARNING: Color32 = Color32::from_rgb(246, 202, 108);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_panel_minimum_width_fits_sidebar_and_pane() {
        assert!(control_minimum_layout_fits());
    }

    #[test]
    fn control_refresh_stays_available_when_not_pending() {
        assert!(control_refresh_enabled(false));
        assert!(!control_refresh_enabled(true));
    }

    #[test]
    fn process_alive_detects_current_process() {
        assert!(process_alive(std::process::id()));
    }

    #[test]
    fn quick_cmdline_requires_ui_binary_and_quick_flag() {
        assert!(cmdline_is_quick_settings(b"/tmp/hyprbole-ui\0--quick\0"));
        assert!(cmdline_is_quick_settings(
            b"hyprbole-ui\0--quick\0--quick-toggle\0"
        ));
        assert!(!cmdline_is_quick_settings(b"/tmp/hyprbole-ui\0--control\0"));
        assert!(!cmdline_is_quick_settings(
            b"/tmp/other\0hyprbole-ui\0--quick\0"
        ));
        assert!(!cmdline_is_quick_settings(b"/tmp/other\0--quick\0"));
    }

    #[test]
    fn dismiss_request_must_target_current_process() {
        let identity = quick_process_identity(std::process::id()).expect("identity");
        assert!(dismiss_request_targets_current_process(&identity));
        assert!(!dismiss_request_targets_current_process(&format!(
            "{}:1",
            std::process::id()
        )));
        assert!(!dismiss_request_targets_current_process("not-a-pid"));
    }

    #[test]
    fn monitor_mode_limit_caps_visible_rows() {
        assert_eq!(monitor_visible_mode_count(0), 0);
        assert_eq!(monitor_visible_mode_count(3), 3);
        assert_eq!(monitor_visible_mode_count(99), CONTROL_MONITOR_MODE_LIMIT);
    }

    #[test]
    fn compositor_config_apply_requires_connection_and_non_current_target() {
        assert!(compositor_config_apply_enabled(true, false, false));
        assert!(!compositor_config_apply_enabled(true, false, true));
        assert!(!compositor_config_apply_enabled(true, true, false));
        assert!(!compositor_config_apply_enabled(false, false, false));
    }

    #[test]
    fn visible_monitor_modes_keeps_current_mode_visible() {
        let available_modes = (0..12)
            .map(|index| hyprbole_core::daemon::MonitorModeSnapshot {
                width: 1920 + index,
                height: 1080,
                refresh_hz: 60.0,
                label: format!("{}x1080@60.00Hz", 1920 + index),
            })
            .collect::<Vec<_>>();
        let current_mode = available_modes[10].clone();
        let monitor = hyprbole_core::daemon::MonitorSnapshot {
            id: 1,
            name: "Virtual-1".to_string(),
            focused: true,
            position: "0x0".to_string(),
            scale: 1.0,
            current_mode: Some(current_mode.clone()),
            active_workspace: Some("1".to_string()),
            available_modes,
        };

        let visible = visible_monitor_modes(&monitor);
        assert_eq!(visible.len(), CONTROL_MONITOR_MODE_LIMIT + 1);
        assert!(visible.iter().any(|mode| mode.label == current_mode.label));
        assert!(monitor_mode_is_current(&monitor, &current_mode));
    }

    #[test]
    fn bar_monitor_choices_include_active_monitors() {
        let mut panel = ControlPanel::default();
        panel.state.monitors = vec![
            hyprbole_core::daemon::MonitorSnapshot {
                id: 1,
                name: "eDP-1".to_string(),
                focused: true,
                position: "0x0".to_string(),
                scale: 1.0,
                current_mode: None,
                active_workspace: None,
                available_modes: Vec::new(),
            },
            hyprbole_core::daemon::MonitorSnapshot {
                id: 2,
                name: "HDMI-A-1".to_string(),
                focused: false,
                position: "1920x0".to_string(),
                scale: 1.0,
                current_mode: None,
                active_workspace: None,
                available_modes: Vec::new(),
            },
        ];

        let choices = bar_monitor_choices(&panel);
        assert!(choices.iter().any(|(_, value)| value == "focused"));
        assert!(choices.iter().any(|(_, value)| value == "primary"));
        assert!(choices.iter().any(|(_, value)| value == "eDP-1"));
        assert!(choices.iter().any(|(_, value)| value == "HDMI-A-1"));
    }

    #[test]
    fn autostart_validation_reports_duplicates() {
        let apps = vec![hyprbole_core::settings::AutostartApp {
            name: "Portal".to_string(),
            command: "xdg-desktop-portal-hyprland".to_string(),
            enabled: true,
        }];

        assert_eq!(
            autostart_validation_message(&apps, "Portal", "other"),
            Some("A startup app with this name already exists.")
        );
        assert_eq!(
            autostart_validation_message(&apps, "Other", "xdg-desktop-portal-hyprland"),
            Some("A startup app with this command already exists.")
        );
        assert_eq!(autostart_validation_message(&apps, "Other", "other"), None);
    }

    #[test]
    fn workspace_state_labels_prioritize_focus() {
        let workspace = Workspace {
            id: 1,
            name: Some("1".to_string()),
            focused: true,
            active: true,
            occupied: true,
        };
        assert_eq!(workspace_state_label(&workspace), "focused");
    }
}
