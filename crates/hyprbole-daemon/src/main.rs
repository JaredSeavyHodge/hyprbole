use std::hash::{Hash, Hasher};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use hyprbole_core::compositor::{CompositorBackend, HyprlandIpcBackend};
use hyprbole_core::daemon::{
    self, ActionStatus, CapabilitySupport, DaemonStatus, ReconcileSeverity, ReconcileStatus,
    ShellAction, ShellRequest, ShellResponse, ShellSnapshot, SubsystemCapability,
    SubsystemReconcileStatus,
};
use hyprbole_core::daemon::{
    AutostartAppStatus, KeybindingSnapshot, KeybindingSource, ShellEvent, ShellEventRecord,
};
use hyprbole_core::notifications::{
    NotificationAction, NotificationRecord, NotificationSnapshot, NotificationUrgency,
};
use hyprbole_core::settings::{self, OwnershipMode, SettingsStore, ShellSettings, ShellSubsystem};
use hyprbole_core::theme::{ThemeMode, ThemeSnapshot};
use serde::Deserialize;

struct DaemonState {
    started_at: Instant,
    socket_path: String,
    settings_path: std::path::PathBuf,
    last_hyprland_event: Option<String>,
    last_reconcile_status: Option<String>,
    reconcile: Option<ReconcileStatus>,
    last_settings_save_status: Option<String>,
    last_action_status: Option<String>,
    last_action: Option<ActionStatus>,
    settings: ShellSettings,
    snapshot: ShellSnapshot,
    notifications: Vec<NotificationRecord>,
    next_notification_id: u64,
    autostart_status: Vec<AutostartAppStatus>,
    events: Vec<ShellEventRecord>,
    next_event_id: u64,
    event_signal: Arc<(Mutex<u64>, Condvar)>,
}

trait DaemonRuntime {
    fn dispatch_action(&self, action: ShellAction) -> ShellResponse;
    fn refresh_snapshot(&self, state: &Arc<Mutex<DaemonState>>);
    fn load_keybindings(&self, settings: ShellSettings) -> Vec<KeybindingSnapshot>;
    fn save_settings(
        &self,
        settings_path: &Path,
        settings: &ShellSettings,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

struct RealRuntime;

impl DaemonRuntime for RealRuntime {
    fn dispatch_action(&self, action: ShellAction) -> ShellResponse {
        dispatch_action(action)
    }

    fn refresh_snapshot(&self, state: &Arc<Mutex<DaemonState>>) {
        refresh_snapshot(state);
    }

    fn load_keybindings(&self, settings: ShellSettings) -> Vec<KeybindingSnapshot> {
        load_keybinding_state(settings)
    }

    fn save_settings(
        &self,
        settings_path: &Path,
        settings: &ShellSettings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        settings::FileSettingsStore::new(settings_path.to_path_buf()).save(settings)?;
        Ok(())
    }
}

impl DaemonState {
    fn new(
        settings: ShellSettings,
        socket_path: String,
        settings_path: std::path::PathBuf,
    ) -> Self {
        Self {
            started_at: Instant::now(),
            socket_path,
            settings_path,
            last_hyprland_event: None,
            last_reconcile_status: None,
            reconcile: None,
            last_settings_save_status: None,
            last_action_status: None,
            last_action: None,
            settings,
            snapshot: ShellSnapshot::default(),
            notifications: Vec::new(),
            next_notification_id: 1,
            autostart_status: Vec::new(),
            events: Vec::new(),
            next_event_id: 1,
            event_signal: Arc::new((Mutex::new(0), Condvar::new())),
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("hyprbole daemon failed: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    log_daemon("startup", "starting hyprbole daemon");
    let socket_path = daemon::ensure_socket_dir().map_err(|err| err.to_string())?;
    if socket_path.exists() {
        match UnixStream::connect(&socket_path) {
            Ok(_) => {
                return Err(format!(
                    "daemon already running at {}",
                    socket_path.display()
                ));
            }
            Err(_) => std::fs::remove_file(&socket_path)
                .map_err(|err| format!("remove stale socket {}: {err}", socket_path.display()))?,
        }
    }

    let listener = UnixListener::bind(&socket_path)
        .map_err(|err| format!("bind {}: {err}", socket_path.display()))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("set listener nonblocking: {err}"))?;
    let settings_store = settings::FileSettingsStore::from_env().map_err(|err| err.to_string())?;
    let settings_path = settings_store.path().to_path_buf();
    let settings = settings_store
        .load_or_default()
        .map_err(|err| err.to_string())?;
    let state = Arc::new(Mutex::new(DaemonState::new(
        settings,
        socket_path.display().to_string(),
        settings_path,
    )));
    let shutdown = Arc::new(AtomicBool::new(false));
    refresh_snapshot(&state);
    reconcile_runtime_state(&state, None);
    refresh_autostart_status(&state);
    launch_enabled_autostart(&state);
    spawn_hyprland_event_listener(state.clone());
    println!("hyprbole daemon listening on {}", socket_path.display());
    log_daemon("socket", format!("listening on {}", socket_path.display()));

    let mut handlers = Vec::new();
    while !shutdown.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((mut stream, _addr)) => {
                let state = state.clone();
                let shutdown = shutdown.clone();
                handlers.push(thread::spawn(move || {
                    match handle_connection(&mut stream, &state, &shutdown) {
                        Ok(true) => {
                            shutdown.store(true, Ordering::SeqCst);
                            notify_event_streams(&state);
                        }
                        Ok(false) => {}
                        Err(err) => eprintln!("daemon connection failed: {err}"),
                    }
                }));
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(err) => return Err(format!("accept connection: {err}")),
        }
    }

    for handler in handlers {
        let _ = handler.join();
    }

    let _ = std::fs::remove_file(&socket_path);
    Ok(())
}

fn spawn_hyprland_event_listener(state: Arc<Mutex<DaemonState>>) {
    thread::spawn(move || {
        loop {
            let Some(path) = hyprland_socket2_path() else {
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            let Ok(mut stream) = UnixStream::connect(path) else {
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            log_daemon("hyprland_socket2", "connected");

            reconcile_runtime_state(&state, None);
            let mut buffer = [0_u8; 4096];
            let mut pending = String::new();
            loop {
                let Ok(read) = stream.read(&mut buffer) else {
                    break;
                };
                if read == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buffer[..read]));
                while let Some(index) = pending.find('\n') {
                    let line = pending[..index].to_owned();
                    pending = pending[index + 1..].to_owned();
                    if !line.trim().is_empty() {
                        if let Ok(mut state) = state.lock() {
                            state.last_hyprland_event = Some(line.clone());
                            push_event(&mut state, ShellEvent::Hyprland { raw: line.clone() });
                        }
                        push_state_changed_domain(&state, "hyprland_event", "compositor");
                        if is_reload_event(&line) {
                            reconcile_runtime_state(&state, None);
                        }
                    }
                }
            }
            log_daemon("hyprland_socket2", "disconnected; reconnecting");
            thread::sleep(Duration::from_millis(250));
        }
    });
}

fn log_daemon(event: &str, message: impl AsRef<str>) {
    eprintln!("hyprbole event={event} message={:?}", message.as_ref());
}

fn is_reload_event(line: &str) -> bool {
    line.contains("configreloaded") || line.contains("config.reloaded")
}

fn hyprland_socket2_path() -> Option<std::path::PathBuf> {
    Some(
        std::path::PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?)
            .join("hypr")
            .join(std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?)
            .join(".socket2.sock"),
    )
}

fn handle_connection(
    stream: &mut UnixStream,
    state: &Arc<Mutex<DaemonState>>,
    shutdown: &Arc<AtomicBool>,
) -> Result<bool, String> {
    let mut request = String::new();
    stream
        .read_to_string(&mut request)
        .map_err(|err| format!("read request: {err}"))?;

    let parsed = serde_json::from_str::<ShellRequest>(&request);
    if let Ok(ShellRequest::EventsFollow { after }) = parsed {
        return handle_event_stream(stream, state, shutdown, after);
    }

    let (response, should_shutdown) = match parsed {
        Ok(request) => handle_shell_request(request, state),
        Err(_) => handle_legacy_request(&request),
    };

    let wire = serde_json::to_string(&response).map_err(|err| format!("encode response: {err}"))?;
    stream
        .write_all(wire.as_bytes())
        .and_then(|()| stream.write_all(b"\n"))
        .map_err(|err| format!("write response: {err}"))?;
    Ok(should_shutdown)
}

fn handle_shell_request(
    request: ShellRequest,
    state: &Arc<Mutex<DaemonState>>,
) -> (ShellResponse, bool) {
    handle_shell_request_with_runtime(request, state, &RealRuntime)
}

fn handle_shell_request_with_runtime(
    request: ShellRequest,
    state: &Arc<Mutex<DaemonState>>,
    runtime: &impl DaemonRuntime,
) -> (ShellResponse, bool) {
    match request {
        ShellRequest::Ping => (ShellResponse::ok("pong"), false),
        ShellRequest::StatusGet => (
            ShellResponse::Status {
                daemon: daemon_status(state),
            },
            false,
        ),
        ShellRequest::StateGet => (
            ShellResponse::State {
                snapshot: snapshot_with_runtime(state, runtime),
            },
            false,
        ),
        ShellRequest::SettingsGet => (
            ShellResponse::Settings {
                settings: settings_snapshot(state),
            },
            false,
        ),
        ShellRequest::SettingsSetOwnership { subsystem, mode } => {
            let response = set_ownership(state, subsystem, mode)
                .unwrap_or_else(|err| ShellResponse::error(err.to_string()));
            if response.is_ok() {
                push_state_changed_domain(state, "settings_changed", "settings");
            }
            (response, false)
        }
        ShellRequest::SettingsSetAutostart { apps } => {
            let response = set_autostart_apps(state, apps)
                .unwrap_or_else(|err| ShellResponse::error(err.to_string()));
            if response.is_ok() {
                push_state_changed_domain(state, "settings_changed", "settings");
            }
            (response, false)
        }
        ShellRequest::UiSettingsGet => (
            ShellResponse::UiSettings {
                settings: settings_snapshot(state).ui,
            },
            false,
        ),
        ShellRequest::ThemeGet => (
            ShellResponse::Theme {
                snapshot: theme_snapshot(state),
            },
            false,
        ),
        ShellRequest::ThemeSetMode { mode } => {
            let response = set_theme_mode(state, runtime, mode);
            if response.is_ok() {
                push_state_changed_domain(state, "theme_changed", "theme");
            }
            (response, false)
        }
        ShellRequest::ThemeReset => {
            let response = set_theme_mode(state, runtime, ThemeMode::default());
            if response.is_ok() {
                push_state_changed_domain(state, "theme_changed", "theme");
            }
            (response, false)
        }
        ShellRequest::NotificationsGet => (
            ShellResponse::Notifications {
                snapshot: notifications_snapshot(state),
            },
            false,
        ),
        ShellRequest::BarSettingsSet { settings } => {
            let response = set_bar_settings_with_runtime(state, settings, runtime)
                .unwrap_or_else(|err| ShellResponse::error(err.to_string()));
            if response.is_ok() {
                push_state_changed_domain(state, "bar_settings_changed", "settings");
            }
            (response, false)
        }
        ShellRequest::BarSettingsSetField { field, value } => {
            let response = set_bar_setting_field_with_runtime(state, &field, &value, runtime)
                .unwrap_or_else(|err| ShellResponse::error(err.to_string()));
            if response.is_ok() {
                push_state_changed_domain(state, "bar_settings_changed", "settings");
            }
            (response, false)
        }
        ShellRequest::BarSettingsReset => {
            let response = set_bar_settings_with_runtime(
                state,
                hyprbole_core::settings::BarSettings::default(),
                runtime,
            )
            .unwrap_or_else(|err| ShellResponse::error(err.to_string()));
            if response.is_ok() {
                push_state_changed_domain(state, "bar_settings_changed", "settings");
            }
            (response, false)
        }
        ShellRequest::OsdSettingsSet { settings } => {
            let response = set_osd_settings_with_runtime(state, settings, runtime)
                .unwrap_or_else(|err| ShellResponse::error(err.to_string()));
            if response.is_ok() {
                push_state_changed_domain(state, "osd_settings_changed", "settings");
            }
            (response, false)
        }
        ShellRequest::OsdSettingsSetField { field, value } => {
            let response = set_osd_setting_field_with_runtime(state, &field, &value, runtime)
                .unwrap_or_else(|err| ShellResponse::error(err.to_string()));
            if response.is_ok() {
                push_state_changed_domain(state, "osd_settings_changed", "settings");
            }
            (response, false)
        }
        ShellRequest::OsdSettingsReset => {
            let response = set_osd_settings_with_runtime(
                state,
                hyprbole_core::settings::OsdSettings::default(),
                runtime,
            )
            .unwrap_or_else(|err| ShellResponse::error(err.to_string()));
            if response.is_ok() {
                push_state_changed_domain(state, "osd_settings_changed", "settings");
            }
            (response, false)
        }
        ShellRequest::AutostartRun { name } => {
            let response = run_autostart_app(state, &name);
            record_action_status(state, action_status("autostart", "run", &response));
            if response.is_ok() {
                push_state_changed_domain(state, "autostart_run", "autostart");
            }
            (response, false)
        }
        ShellRequest::BindsGet => (
            ShellResponse::Binds {
                keybindings: runtime.load_keybindings(settings_snapshot(state)),
            },
            false,
        ),
        ShellRequest::EventsSubscribe { after } => (
            ShellResponse::Events {
                events: events_snapshot(state, after),
            },
            false,
        ),
        ShellRequest::EventsFollow { .. } => {
            unreachable!("event stream handled before response path")
        }
        ShellRequest::Reconcile { subsystem } => (reconcile_runtime_state(state, subsystem), false),
        ShellRequest::ActionCall { action } => {
            let (domain, name) = action_identity(&action);
            let response = match action {
                ShellAction::Notifications { action } => {
                    dispatch_notification_action(state, runtime, action)
                }
                action => runtime.dispatch_action(action),
            };
            record_action_status(state, action_status(domain, name, &response));
            if response.is_ok() {
                runtime.refresh_snapshot(state);
                push_state_changed_domain(state, "action_completed", domain);
            } else {
                push_state_changed_domain(state, "action_failed", domain);
            }
            (response, false)
        }
        ShellRequest::Shutdown => (ShellResponse::ok("shutting down"), true),
    }
}

fn handle_legacy_request(request: &str) -> (ShellResponse, bool) {
    match hyprbole_core::daemon::DaemonRequest::parse(request) {
        Ok(hyprbole_core::daemon::DaemonRequest::Ping) => (ShellResponse::ok("pong"), false),
        Ok(hyprbole_core::daemon::DaemonRequest::BindMouse) => (
            dispatch_action(ShellAction::Compositor {
                action: hyprbole_core::compositor::CompositorAction::RegisterMouseWindowControls,
            }),
            false,
        ),
        Ok(hyprbole_core::daemon::DaemonRequest::Shutdown) => {
            (ShellResponse::ok("shutting down"), true)
        }
        Err(err) => (ShellResponse::error(err.to_string()), false),
    }
}

fn dispatch_action(action: ShellAction) -> ShellResponse {
    match action {
        ShellAction::Compositor { action } => {
            match HyprlandIpcBackend::from_env().and_then(|backend| backend.dispatch(action)) {
                Ok(()) => ShellResponse::ok("compositor action completed"),
                Err(err) => ShellResponse::error(err.to_string()),
            }
        }
        ShellAction::Audio { action } => match hyprbole_core::audio::dispatch(action) {
            Ok(()) => ShellResponse::ok("audio action completed"),
            Err(err) => ShellResponse::error(err.to_string()),
        },
        ShellAction::Brightness { action } => match hyprbole_core::brightness::dispatch(action) {
            Ok(()) => ShellResponse::ok("brightness action completed"),
            Err(err) => ShellResponse::error(err.to_string()),
        },
        ShellAction::Notifications { .. } => {
            ShellResponse::error("notification actions require daemon state")
        }
        ShellAction::SessionLock => match Command::new("loginctl").arg("lock-session").output() {
            Ok(output) if output.status.success() => ShellResponse::ok("session lock requested"),
            Ok(output) => ShellResponse::error(String::from_utf8_lossy(&output.stderr)),
            Err(err) => ShellResponse::error(err.to_string()),
        },
    }
}

fn action_status(domain: &str, name: &str, response: &ShellResponse) -> ActionStatus {
    let (ok, message) = match response {
        ShellResponse::Ok { message } => (true, message.clone()),
        ShellResponse::Error { message } => (false, message.clone()),
        _ => (false, "unexpected action response".to_string()),
    };
    ActionStatus {
        domain: domain.to_string(),
        name: name.to_string(),
        ok,
        message,
    }
}

fn record_action_status(state: &Arc<Mutex<DaemonState>>, status: ActionStatus) {
    let legacy_status = if status.ok {
        format!("ok: {}", status.message)
    } else {
        format!("error: {}", status.message)
    };
    if let Ok(mut state) = state.lock() {
        state.last_action_status = Some(legacy_status);
        state.last_action = Some(status.clone());
        push_event(&mut state, ShellEvent::Action { status });
    }
}

fn action_identity(action: &ShellAction) -> (&'static str, &'static str) {
    match action {
        ShellAction::Compositor { action } => ("compositor", compositor_action_name(action)),
        ShellAction::Audio { action } => ("audio", audio_action_name(action)),
        ShellAction::Brightness { action } => ("brightness", brightness_action_name(action)),
        ShellAction::Notifications { action } => {
            ("notifications", notification_action_name(action))
        }
        ShellAction::SessionLock => ("session", "lock"),
    }
}

fn notification_action_name(action: &NotificationAction) -> &'static str {
    match action {
        NotificationAction::SetDnd { .. } => "set_dnd",
        NotificationAction::ToggleDnd => "toggle_dnd",
        NotificationAction::ClearHistory => "clear_history",
        NotificationAction::Dismiss { .. } => "dismiss",
        NotificationAction::Push { .. } => "push",
    }
}

fn compositor_action_name(action: &hyprbole_core::compositor::CompositorAction) -> &'static str {
    match action {
        hyprbole_core::compositor::CompositorAction::FocusWorkspace { .. } => "focus_workspace",
        hyprbole_core::compositor::CompositorAction::FocusNextWorkspace => "focus_next_workspace",
        hyprbole_core::compositor::CompositorAction::FocusPreviousWorkspace => {
            "focus_previous_workspace"
        }
        hyprbole_core::compositor::CompositorAction::SetLayout { .. } => "set_layout",
        hyprbole_core::compositor::CompositorAction::SetMonitorMode { .. } => "set_monitor_mode",
        hyprbole_core::compositor::CompositorAction::RegisterMouseWindowControls => {
            "register_mouse_window_controls"
        }
        hyprbole_core::compositor::CompositorAction::UnregisterMouseWindowControls => {
            "unregister_mouse_window_controls"
        }
    }
}

fn audio_action_name(action: &hyprbole_core::audio::AudioAction) -> &'static str {
    match action {
        hyprbole_core::audio::AudioAction::VolumeUp => "volume_up",
        hyprbole_core::audio::AudioAction::VolumeDown => "volume_down",
        hyprbole_core::audio::AudioAction::ToggleMute => "toggle_mute",
    }
}

fn brightness_action_name(action: &hyprbole_core::brightness::BrightnessAction) -> &'static str {
    match action {
        hyprbole_core::brightness::BrightnessAction::Increase => "increase",
        hyprbole_core::brightness::BrightnessAction::Decrease => "decrease",
    }
}

#[allow(dead_code)]
fn legacy_action_status(response: &ShellResponse) -> String {
    match response {
        ShellResponse::Ok { message } => format!("ok: {message}"),
        ShellResponse::Error { message } => format!("error: {message}"),
        _ => "unexpected action response".to_string(),
    }
}

fn handle_event_stream(
    stream: &mut UnixStream,
    state: &Arc<Mutex<DaemonState>>,
    shutdown: &Arc<AtomicBool>,
    after: Option<u64>,
) -> Result<bool, String> {
    let mut cursor = after.unwrap_or(0);
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|err| format!("set event stream write timeout: {err}"))?;
    let signal = state
        .lock()
        .map(|state| state.event_signal.clone())
        .map_err(|_| "state lock poisoned".to_string())?;
    let (signal_lock, signal_cvar) = &*signal;
    let mut seen_signal = *signal_lock
        .lock()
        .map_err(|_| "event signal lock poisoned".to_string())?;
    while !shutdown.load(Ordering::SeqCst) {
        let events = events_snapshot(state, Some(cursor));
        for event in events {
            cursor = event.id;
            let wire =
                serde_json::to_string(&event).map_err(|err| format!("encode event: {err}"))?;
            stream
                .write_all(wire.as_bytes())
                .and_then(|()| stream.write_all(b"\n"))
                .map_err(|err| format!("write event: {err}"))?;
        }
        stream
            .flush()
            .map_err(|err| format!("flush event stream: {err}"))?;
        let guard = signal_lock
            .lock()
            .map_err(|_| "event signal lock poisoned".to_string())?;
        if *guard == seen_signal {
            let (guard, _) = signal_cvar
                .wait_timeout(guard, Duration::from_secs(30))
                .map_err(|_| "event signal lock poisoned".to_string())?;
            seen_signal = *guard;
        } else {
            seen_signal = *guard;
        }
    }
    Ok(false)
}

fn push_event(state: &mut DaemonState, event: ShellEvent) {
    let id = state.next_event_id;
    state.next_event_id += 1;
    state.events.push(ShellEventRecord { id, event });
    let (signal_lock, signal_cvar) = &*state.event_signal;
    if let Ok(mut signal) = signal_lock.lock() {
        *signal += 1;
        signal_cvar.notify_all();
    }
    if state.events.len() > 100 {
        let extra = state.events.len() - 100;
        state.events.drain(0..extra);
    }
}

fn push_state_changed(state: &Arc<Mutex<DaemonState>>, reason: impl Into<String>) {
    push_state_changed_optional_domain(state, reason.into(), None);
}

fn push_state_changed_domain(
    state: &Arc<Mutex<DaemonState>>,
    reason: impl Into<String>,
    domain: impl Into<String>,
) {
    push_state_changed_optional_domain(state, reason.into(), Some(domain.into()));
}

fn push_state_changed_optional_domain(
    state: &Arc<Mutex<DaemonState>>,
    reason: String,
    domain: Option<String>,
) {
    if let Ok(mut state) = state.lock() {
        push_event(&mut state, ShellEvent::StateChanged { reason, domain });
    }
}

fn notify_event_streams(state: &Arc<Mutex<DaemonState>>) {
    let signal = state.lock().ok().map(|state| state.event_signal.clone());
    if let Some(signal) = signal {
        let (signal_lock, signal_cvar) = &*signal;
        if let Ok(mut signal) = signal_lock.lock() {
            *signal += 1;
            signal_cvar.notify_all();
        }
    }
}

fn events_snapshot(state: &Arc<Mutex<DaemonState>>, after: Option<u64>) -> Vec<ShellEventRecord> {
    let after = after.unwrap_or(0);
    state
        .lock()
        .map(|state| {
            let mut events = Vec::new();
            if let Some(oldest) = state.events.first() {
                if oldest.id > after + 1 {
                    events.push(ShellEventRecord {
                        id: oldest.id - 1,
                        event: ShellEvent::Gap {
                            after,
                            oldest: oldest.id,
                        },
                    });
                }
            }
            events.extend(
                state
                    .events
                    .iter()
                    .filter(|event| event.id > after)
                    .cloned(),
            );
            events
        })
        .unwrap_or_default()
}

fn settings_snapshot(state: &Arc<Mutex<DaemonState>>) -> ShellSettings {
    state
        .lock()
        .map(|state| state.settings.clone())
        .unwrap_or_default()
}

fn daemon_status(state: &Arc<Mutex<DaemonState>>) -> DaemonStatus {
    refresh_autostart_status(state);
    state
        .lock()
        .map(|state| DaemonStatus {
            protocol_version: hyprbole_core::runtime::PROTOCOL_VERSION,
            build_version: env!("CARGO_PKG_VERSION").to_string(),
            pid: std::process::id(),
            uptime_seconds: state.started_at.elapsed().as_secs(),
            socket_path: state.socket_path.clone(),
            settings_path: state.settings_path.display().to_string(),
            event_count: state.events.len(),
            next_event_id: state.next_event_id,
            recent_events: state
                .events
                .iter()
                .rev()
                .take(8)
                .cloned()
                .collect::<Vec<_>>(),
            ownership_capabilities: ownership_capabilities(),
            last_hyprland_event: state.last_hyprland_event.clone(),
            last_reconcile_status: state.last_reconcile_status.clone(),
            reconcile: state.reconcile.clone(),
            last_settings_save_status: state.last_settings_save_status.clone(),
            last_action_status: state.last_action_status.clone(),
            last_action: state.last_action.clone(),
            autostart: state.autostart_status.clone(),
        })
        .unwrap_or_else(|_| DaemonStatus {
            protocol_version: hyprbole_core::runtime::PROTOCOL_VERSION,
            build_version: env!("CARGO_PKG_VERSION").to_string(),
            pid: std::process::id(),
            uptime_seconds: 0,
            socket_path: "unknown".to_string(),
            settings_path: "unknown".to_string(),
            event_count: 0,
            next_event_id: 0,
            recent_events: Vec::new(),
            ownership_capabilities: ownership_capabilities(),
            last_hyprland_event: None,
            last_reconcile_status: None,
            reconcile: None,
            last_settings_save_status: Some("state lock poisoned".to_string()),
            last_action_status: None,
            last_action: None,
            autostart: Vec::new(),
        })
}

fn ownership_capabilities() -> Vec<SubsystemCapability> {
    [
        (
            ShellSubsystem::Keybindings,
            CapabilitySupport::Implemented,
            CapabilitySupport::Reserved,
            "runtime keybinding reconciliation is implemented",
        ),
        (
            ShellSubsystem::Monitors,
            CapabilitySupport::ObserveOnly,
            CapabilitySupport::Reserved,
            "monitor reconciliation currently reports ownership only",
        ),
        (
            ShellSubsystem::Workspaces,
            CapabilitySupport::ObserveOnly,
            CapabilitySupport::Reserved,
            "workspace reconciliation currently reports ownership only",
        ),
        (
            ShellSubsystem::Audio,
            CapabilitySupport::Partial,
            CapabilitySupport::Reserved,
            "audio actions and structured status are available",
        ),
        (
            ShellSubsystem::Brightness,
            CapabilitySupport::Partial,
            CapabilitySupport::Reserved,
            "brightness actions and structured status are available",
        ),
        (
            ShellSubsystem::Appearance,
            CapabilitySupport::Reserved,
            CapabilitySupport::Reserved,
            "appearance ownership is reserved",
        ),
        (
            ShellSubsystem::Notifications,
            CapabilitySupport::Reserved,
            CapabilitySupport::Reserved,
            "notification ownership is reserved",
        ),
        (
            ShellSubsystem::Power,
            CapabilitySupport::Reserved,
            CapabilitySupport::Reserved,
            "power ownership is reserved",
        ),
    ]
    .into_iter()
    .map(
        |(subsystem, runtime_reconcile, persisted_ownership, notes)| SubsystemCapability {
            subsystem,
            runtime_reconcile,
            persisted_ownership,
            notes: notes.to_string(),
        },
    )
    .collect()
}

fn set_ownership(
    state: &Arc<Mutex<DaemonState>>,
    subsystem: ShellSubsystem,
    mode: OwnershipMode,
) -> Result<ShellResponse, Box<dyn std::error::Error>> {
    if !ownership_mode_supported(subsystem, mode) {
        return Ok(ShellResponse::error(format!(
            "{subsystem} {mode} ownership is not supported yet; use respect_user_config"
        )));
    }
    let old_mode = settings_snapshot(state).ownership(subsystem);

    if subsystem == ShellSubsystem::Keybindings
        && old_mode.controls_runtime()
        && !mode.controls_runtime()
    {
        persist_ownership_settings(state, subsystem, mode, |settings_path, settings| {
            settings::FileSettingsStore::new(settings_path.to_path_buf()).save(settings)?;
            Ok(())
        })?;
        let release = unregister_runtime_keybindings(state);
        if !release.is_ok() {
            let _ = persist_ownership_settings(
                state,
                subsystem,
                old_mode,
                |settings_path, settings| {
                    settings::FileSettingsStore::new(settings_path.to_path_buf()).save(settings)?;
                    Ok(())
                },
            );
            return Ok(release);
        }
        return Ok(ShellResponse::ok(
            "set keybindings ownership to respect_user_config and released runtime binds",
        ));
    }

    persist_ownership_settings(state, subsystem, mode, |settings_path, settings| {
        settings::FileSettingsStore::new(settings_path.to_path_buf()).save(settings)?;
        Ok(())
    })?;

    if subsystem == ShellSubsystem::Keybindings {
        Ok(reconcile_runtime_state(
            state,
            Some(ShellSubsystem::Keybindings),
        ))
    } else {
        Ok(ShellResponse::ok(format!(
            "set {subsystem} ownership to {mode}"
        )))
    }
}

fn set_autostart_apps(
    state: &Arc<Mutex<DaemonState>>,
    apps: Vec<hyprbole_core::settings::AutostartApp>,
) -> Result<ShellResponse, Box<dyn std::error::Error>> {
    if let Some(duplicate) = duplicate_autostart_entry(&apps) {
        return Ok(ShellResponse::error(format!(
            "duplicate autostart entry `{duplicate}`"
        )));
    }
    let mut state = state.lock().map_err(|_| "state lock poisoned")?;
    let mut settings = state.settings.clone();
    settings.autostart.apps = apps;
    match settings::FileSettingsStore::new(state.settings_path.clone()).save(&settings) {
        Ok(()) => {
            state.settings = settings;
            state.last_settings_save_status = Some("saved autostart settings".to_string());
            refresh_autostart_status_locked(&mut state);
            Ok(ShellResponse::ok("saved autostart settings"))
        }
        Err(err) => {
            state.last_settings_save_status = Some(format!("failed to save settings: {err}"));
            Err(Box::new(err))
        }
    }
}

fn dispatch_notification_action(
    state: &Arc<Mutex<DaemonState>>,
    runtime: &impl DaemonRuntime,
    action: NotificationAction,
) -> ShellResponse {
    match action {
        NotificationAction::SetDnd { enabled } => set_dnd(state, runtime, enabled),
        NotificationAction::ToggleDnd => toggle_dnd(state, runtime),
        NotificationAction::ClearHistory => {
            if let Ok(mut state) = state.lock() {
                state.notifications.clear();
            }
            ShellResponse::ok("cleared notification history")
        }
        NotificationAction::Dismiss { id } => {
            if let Ok(mut state) = state.lock() {
                if let Some(record) = state
                    .notifications
                    .iter_mut()
                    .find(|record| record.id == id)
                {
                    record.read = true;
                    return ShellResponse::ok(format!("dismissed notification {id}"));
                }
            }
            ShellResponse::error(format!("notification {id} not found"))
        }
        NotificationAction::Push {
            summary,
            body,
            urgency,
        } => push_notification_record(state, summary, body, urgency),
    }
}

fn set_dnd(
    state: &Arc<Mutex<DaemonState>>,
    runtime: &impl DaemonRuntime,
    enabled: bool,
) -> ShellResponse {
    let mut state = match state.lock() {
        Ok(state) => state,
        Err(_) => return ShellResponse::error("state lock poisoned"),
    };
    let mut settings = state.settings.clone();
    settings.notifications.dnd_enabled = enabled;
    match runtime.save_settings(&state.settings_path, &settings) {
        Ok(()) => {
            state.settings = settings.clone();
            state.snapshot.settings = settings;
            state.last_settings_save_status = Some("saved notification settings".to_string());
            ShellResponse::ok(if enabled {
                "enabled DND"
            } else {
                "disabled DND"
            })
        }
        Err(err) => {
            state.last_settings_save_status = Some(format!("failed to save settings: {err}"));
            ShellResponse::error(err.to_string())
        }
    }
}

fn toggle_dnd(state: &Arc<Mutex<DaemonState>>, runtime: &impl DaemonRuntime) -> ShellResponse {
    let mut state = match state.lock() {
        Ok(state) => state,
        Err(_) => return ShellResponse::error("state lock poisoned"),
    };
    let mut settings = state.settings.clone();
    settings.notifications.dnd_enabled = !settings.notifications.dnd_enabled;
    match runtime.save_settings(&state.settings_path, &settings) {
        Ok(()) => {
            let enabled = settings.notifications.dnd_enabled;
            state.settings = settings.clone();
            state.snapshot.settings = settings;
            state.last_settings_save_status = Some("saved notification settings".to_string());
            ShellResponse::ok(if enabled {
                "enabled DND"
            } else {
                "disabled DND"
            })
        }
        Err(err) => {
            state.last_settings_save_status = Some(format!("failed to save settings: {err}"));
            ShellResponse::error(err.to_string())
        }
    }
}

fn push_notification_record(
    state: &Arc<Mutex<DaemonState>>,
    summary: String,
    body: String,
    urgency: NotificationUrgency,
) -> ShellResponse {
    if summary.trim().is_empty() {
        return ShellResponse::error("notification summary cannot be empty");
    }
    let mut state = match state.lock() {
        Ok(state) => state,
        Err(_) => return ShellResponse::error("state lock poisoned"),
    };
    let id = state.next_notification_id;
    state.next_notification_id += 1;
    let record = NotificationRecord {
        id,
        summary,
        body,
        urgency,
        read: state.settings.notifications.dnd_enabled,
    };
    state.notifications.insert(0, record);
    let limit = state.settings.notifications.effective_history_limit();
    if state.notifications.len() > limit {
        state.notifications.truncate(limit);
    }
    ShellResponse::ok(format!("recorded notification {id}"))
}

fn notifications_snapshot(state: &Arc<Mutex<DaemonState>>) -> NotificationSnapshot {
    state
        .lock()
        .map(|state| {
            hyprbole_core::notifications::snapshot(
                &state.settings.notifications,
                &state.notifications,
            )
        })
        .unwrap_or_default()
}

fn theme_snapshot(state: &Arc<Mutex<DaemonState>>) -> ThemeSnapshot {
    state
        .lock()
        .map(|state| hyprbole_core::theme::snapshot(&state.settings.theme))
        .unwrap_or_default()
}

fn set_theme_mode(
    state: &Arc<Mutex<DaemonState>>,
    runtime: &impl DaemonRuntime,
    mode: ThemeMode,
) -> ShellResponse {
    let mut state = match state.lock() {
        Ok(state) => state,
        Err(_) => return ShellResponse::error("state lock poisoned"),
    };
    let mut settings = state.settings.clone();
    settings.theme.mode = mode;
    match runtime.save_settings(&state.settings_path, &settings) {
        Ok(()) => {
            state.settings = settings.clone();
            state.snapshot.settings = settings.clone();
            state.snapshot.theme = hyprbole_core::theme::snapshot(&settings.theme);
            state.last_settings_save_status = Some("saved theme settings".to_string());
            ShellResponse::ok(format!("set theme mode to {mode}"))
        }
        Err(err) => {
            state.last_settings_save_status = Some(format!("failed to save settings: {err}"));
            ShellResponse::error(err.to_string())
        }
    }
}

fn set_bar_setting_field_with_runtime(
    state: &Arc<Mutex<DaemonState>>,
    field: &str,
    value: &str,
    runtime: &impl DaemonRuntime,
) -> Result<ShellResponse, Box<dyn std::error::Error>> {
    let mut settings = settings_snapshot(state).ui.bar;
    settings.set_field(field, value)?;
    set_bar_settings_with_runtime(state, settings, runtime)
}

fn set_bar_settings_with_runtime(
    state: &Arc<Mutex<DaemonState>>,
    bar: hyprbole_core::settings::BarSettings,
    runtime: &impl DaemonRuntime,
) -> Result<ShellResponse, Box<dyn std::error::Error>> {
    bar.validate()?;
    persist_bar_settings(state, bar, |settings_path, settings| {
        runtime.save_settings(settings_path, settings)
    })
}

fn persist_bar_settings(
    state: &Arc<Mutex<DaemonState>>,
    bar: hyprbole_core::settings::BarSettings,
    save: impl FnOnce(&Path, &ShellSettings) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<ShellResponse, Box<dyn std::error::Error>> {
    let mut state = state.lock().map_err(|_| "state lock poisoned")?;
    let mut settings = state.settings.clone();
    settings.ui.bar = bar;
    match save(&state.settings_path, &settings) {
        Ok(()) => {
            state.settings = settings.clone();
            state.snapshot.settings = settings;
            state.last_settings_save_status = Some("saved bar settings".to_string());
            Ok(ShellResponse::ok("saved bar settings"))
        }
        Err(err) => {
            state.last_settings_save_status = Some(format!("failed to save settings: {err}"));
            Err(err)
        }
    }
}

fn set_osd_setting_field_with_runtime(
    state: &Arc<Mutex<DaemonState>>,
    field: &str,
    value: &str,
    runtime: &impl DaemonRuntime,
) -> Result<ShellResponse, Box<dyn std::error::Error>> {
    let mut settings = settings_snapshot(state).ui.osd;
    settings.set_field(field, value)?;
    set_osd_settings_with_runtime(state, settings, runtime)
}

fn set_osd_settings_with_runtime(
    state: &Arc<Mutex<DaemonState>>,
    osd: hyprbole_core::settings::OsdSettings,
    runtime: &impl DaemonRuntime,
) -> Result<ShellResponse, Box<dyn std::error::Error>> {
    osd.validate()?;
    persist_osd_settings(state, osd, |settings_path, settings| {
        runtime.save_settings(settings_path, settings)
    })
}

fn persist_osd_settings(
    state: &Arc<Mutex<DaemonState>>,
    osd: hyprbole_core::settings::OsdSettings,
    save: impl FnOnce(&Path, &ShellSettings) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<ShellResponse, Box<dyn std::error::Error>> {
    let mut state = state.lock().map_err(|_| "state lock poisoned")?;
    let mut settings = state.settings.clone();
    settings.ui.osd = osd;
    match save(&state.settings_path, &settings) {
        Ok(()) => {
            state.settings = settings.clone();
            state.snapshot.settings = settings;
            state.last_settings_save_status = Some("saved OSD settings".to_string());
            Ok(ShellResponse::ok("saved OSD settings"))
        }
        Err(err) => {
            state.last_settings_save_status = Some(format!("failed to save settings: {err}"));
            Err(err)
        }
    }
}

fn launch_enabled_autostart(state: &Arc<Mutex<DaemonState>>) {
    let apps = settings_snapshot(state).autostart.apps;
    for app in apps.into_iter().filter(|app| app.enabled) {
        let response = launch_autostart_app(state, &app);
        if !response.is_ok() {
            log_daemon("autostart", response_message(&response));
        }
    }
}

fn refresh_autostart_status(state: &Arc<Mutex<DaemonState>>) {
    if let Ok(mut state) = state.lock() {
        refresh_autostart_status_locked(&mut state);
    }
}

fn run_autostart_app(state: &Arc<Mutex<DaemonState>>, name: &str) -> ShellResponse {
    let Some(app) = settings_snapshot(state)
        .autostart
        .apps
        .into_iter()
        .find(|app| app.name == name)
    else {
        return ShellResponse::error(format!("unknown autostart app `{name}`"));
    };
    launch_autostart_app(state, &app)
}

fn launch_autostart_app(
    state: &Arc<Mutex<DaemonState>>,
    app: &hyprbole_core::settings::AutostartApp,
) -> ShellResponse {
    if app.command.trim().is_empty() || app.name.trim().is_empty() {
        update_autostart_status(
            state,
            app,
            false,
            None,
            Some("missing name or command".to_string()),
        );
        return ShellResponse::error("missing autostart name or command");
    }

    if let Some(pid) = running_autostart_pid(app) {
        update_autostart_status(
            state,
            app,
            true,
            Some(pid),
            Some("already running".to_string()),
        );
        return ShellResponse::ok(format!("{} already running", app.name));
    }

    match Command::new("sh")
        .arg("-c")
        .arg(&app.command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => {
            let pid = child.id();
            if let Err(err) = write_autostart_pid(app, pid) {
                update_autostart_status(
                    state,
                    app,
                    true,
                    Some(pid),
                    Some(format!("launched pid {pid}; failed to record pid: {err}")),
                );
                return ShellResponse::error(format!(
                    "launched {} but failed to record pid: {err}",
                    app.name
                ));
            }
            update_autostart_status(
                state,
                app,
                true,
                Some(pid),
                Some(format!("launched pid {pid}")),
            );
            ShellResponse::ok(format!("launched {}", app.name))
        }
        Err(err) => {
            update_autostart_status(state, app, false, None, Some(err.to_string()));
            ShellResponse::error(format!("failed to launch {}: {err}", app.name))
        }
    }
}

fn running_autostart_pid(app: &hyprbole_core::settings::AutostartApp) -> Option<u32> {
    let pid = std::fs::read_to_string(autostart_pid_path(app).ok()?)
        .ok()?
        .trim()
        .parse()
        .ok()?;
    if pid_alive(pid) { Some(pid) } else { None }
}

fn write_autostart_pid(
    app: &hyprbole_core::settings::AutostartApp,
    pid: u32,
) -> Result<(), String> {
    let path = autostart_pid_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    std::fs::write(path, pid.to_string()).map_err(|err| err.to_string())
}

fn autostart_pid_path(
    app: &hyprbole_core::settings::AutostartApp,
) -> Result<std::path::PathBuf, String> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    app.command.hash(&mut hasher);
    let key = hasher.finish();
    Ok(hyprbole_core::runtime::ensure_runtime_dir()
        .map_err(|err| err.to_string())?
        .join("autostart")
        .join(format!("{key:016x}.pid")))
}

fn pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .is_ok_and(|status| status.success())
}

fn duplicate_autostart_entry(apps: &[hyprbole_core::settings::AutostartApp]) -> Option<String> {
    let mut names = std::collections::HashSet::new();
    let mut commands = std::collections::HashSet::new();
    for app in apps {
        if !names.insert(app.name.trim().to_string()) {
            return Some(app.name.clone());
        }
        if !commands.insert(app.command.trim().to_string()) {
            return Some(app.command.clone());
        }
    }
    None
}

fn update_autostart_status(
    state: &Arc<Mutex<DaemonState>>,
    app: &hyprbole_core::settings::AutostartApp,
    running: bool,
    pid: Option<u32>,
    last_status: Option<String>,
) {
    if let Ok(mut state) = state.lock() {
        let status = AutostartAppStatus {
            name: app.name.clone(),
            command: app.command.clone(),
            enabled: app.enabled,
            running,
            pid,
            last_status,
        };
        if let Some(existing) = state
            .autostart_status
            .iter_mut()
            .find(|existing| existing.name == app.name)
        {
            *existing = status;
        } else {
            state.autostart_status.push(status);
        }
    }
}

fn refresh_autostart_status_locked(state: &mut DaemonState) {
    state.autostart_status = state
        .settings
        .autostart
        .apps
        .iter()
        .map(|app| {
            let pid = running_autostart_pid(app);
            AutostartAppStatus {
                name: app.name.clone(),
                command: app.command.clone(),
                enabled: app.enabled,
                running: pid.is_some(),
                pid,
                last_status: state
                    .autostart_status
                    .iter()
                    .find(|status| status.name == app.name)
                    .and_then(|status| status.last_status.clone()),
            }
        })
        .collect();
}

fn ownership_mode_supported(subsystem: ShellSubsystem, mode: OwnershipMode) -> bool {
    if mode == OwnershipMode::RespectUserConfig {
        return true;
    }
    let Some(capability) = ownership_capabilities()
        .into_iter()
        .find(|capability| capability.subsystem == subsystem)
    else {
        return false;
    };
    let support = match mode {
        OwnershipMode::RespectUserConfig => return true,
        OwnershipMode::RuntimeOwned => capability.runtime_reconcile,
        OwnershipMode::PersistedOwned => capability.persisted_ownership,
    };
    matches!(
        support,
        CapabilitySupport::Implemented | CapabilitySupport::Partial
    )
}

fn persist_ownership_settings(
    state: &Arc<Mutex<DaemonState>>,
    subsystem: ShellSubsystem,
    mode: OwnershipMode,
    save: impl FnOnce(&Path, &ShellSettings) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<OwnershipMode, Box<dyn std::error::Error>> {
    let mut state = state.lock().map_err(|_| "state lock poisoned")?;
    let mut settings = state.settings.clone();
    let old_mode = settings.ownership(subsystem);
    settings.set_ownership(subsystem, mode);
    match save(&state.settings_path, &settings) {
        Ok(()) => {
            state.last_settings_save_status =
                Some(format!("saved {subsystem} ownership as {mode}"));
        }
        Err(err) => {
            state.last_settings_save_status = Some(format!("failed to save settings: {err}"));
            return Err(err);
        }
    }
    state.settings = settings;
    Ok(old_mode)
}

fn unregister_runtime_keybindings(state: &Arc<Mutex<DaemonState>>) -> ShellResponse {
    let response = dispatch_action(ShellAction::Compositor {
        action: hyprbole_core::compositor::CompositorAction::UnregisterMouseWindowControls,
    });
    let message = runtime_keybinding_release_message(&response);
    record_keybinding_release_status(state, message, response.is_ok());
    push_state_changed(state, "keybindings_released");
    response
}

fn runtime_keybinding_release_message(response: &ShellResponse) -> String {
    match response {
        ShellResponse::Ok { message } => format!("runtime keybindings released: {message}"),
        ShellResponse::Error { message } => {
            format!("runtime keybindings release failed: {message}")
        }
        _ => "runtime keybindings release returned unexpected response".to_string(),
    }
}

fn record_keybinding_release_status(state: &Arc<Mutex<DaemonState>>, message: String, ok: bool) {
    if let Ok(mut state) = state.lock() {
        let severity = if ok {
            ReconcileSeverity::Ok
        } else {
            ReconcileSeverity::Error
        };
        state.last_reconcile_status = Some(message.clone());
        state.reconcile = Some(ReconcileStatus {
            severity,
            summary: message.clone(),
            results: vec![SubsystemReconcileStatus {
                subsystem: ShellSubsystem::Keybindings,
                severity,
                implemented: true,
                message,
            }],
        });
        if let Some(status) = state.last_reconcile_status.clone() {
            push_event(&mut state, ShellEvent::Reconcile { status });
        }
    }
}

fn reconcile_runtime_state(
    state: &Arc<Mutex<DaemonState>>,
    subsystem: Option<ShellSubsystem>,
) -> ShellResponse {
    log_daemon("reconcile", "starting runtime reconciliation");
    if let Some(subsystem) = subsystem {
        let response = reconcile_subsystem(state, subsystem, false);
        let result = reconcile_result(
            subsystem,
            &response,
            subsystem_reconcile_implemented(subsystem),
        );
        let severity = result.severity;
        let summary = result.message.clone();
        record_reconcile_status_structured(
            state,
            ReconcileStatus {
                severity,
                summary,
                results: vec![result],
            },
        );
        return response;
    }

    let responses = vec![
        (
            ShellSubsystem::Keybindings,
            reconcile_subsystem(state, ShellSubsystem::Keybindings, false),
        ),
        (
            ShellSubsystem::Monitors,
            reconcile_subsystem(state, ShellSubsystem::Monitors, false),
        ),
        (
            ShellSubsystem::Workspaces,
            reconcile_subsystem(state, ShellSubsystem::Workspaces, false),
        ),
    ];
    finish_full_reconcile(state, responses)
}

fn finish_full_reconcile(
    state: &Arc<Mutex<DaemonState>>,
    responses: Vec<(ShellSubsystem, ShellResponse)>,
) -> ShellResponse {
    let summary = responses
        .iter()
        .map(|(_, response)| response_message(response))
        .collect::<Vec<_>>()
        .join("; ");
    let results = responses
        .iter()
        .map(|(subsystem, response)| {
            reconcile_result(
                *subsystem,
                response,
                subsystem_reconcile_implemented(*subsystem),
            )
        })
        .collect::<Vec<_>>();
    let severity = reconcile_severity_for_results(&results);
    let prefix = match severity {
        ReconcileSeverity::Ok => "full reconcile ok",
        ReconcileSeverity::Warning => "full reconcile warning",
        ReconcileSeverity::Error => "full reconcile failed",
    };
    record_reconcile_status_structured(
        state,
        ReconcileStatus {
            severity,
            summary: format!("{prefix}: {summary}"),
            results,
        },
    );
    if severity == ReconcileSeverity::Error {
        ShellResponse::error(summary)
    } else {
        ShellResponse::ok(summary)
    }
}

fn reconcile_subsystem(
    state: &Arc<Mutex<DaemonState>>,
    subsystem: ShellSubsystem,
    record: bool,
) -> ShellResponse {
    match subsystem {
        ShellSubsystem::Keybindings => reconcile_keybindings(state, record),
        ShellSubsystem::Monitors | ShellSubsystem::Workspaces => {
            reconcile_observe_only_subsystem(state, subsystem, record)
        }
        subsystem => {
            let message =
                format!("{subsystem} reconciliation is not implemented yet; observe-only");
            if record {
                record_reconcile_status(state, message.clone());
            }
            ShellResponse::ok(message)
        }
    }
}

fn reconcile_keybindings(state: &Arc<Mutex<DaemonState>>, record: bool) -> ShellResponse {
    let settings = settings_snapshot(state);
    if !settings
        .ownership(ShellSubsystem::Keybindings)
        .controls_runtime()
    {
        let message = "keybindings respect user config; skipped runtime bind reconciliation";
        if record {
            record_reconcile_status(state, message.to_string());
        }
        return ShellResponse::ok(message);
    }

    let response = dispatch_action(ShellAction::Compositor {
        action: hyprbole_core::compositor::CompositorAction::RegisterMouseWindowControls,
    });
    let message = match &response {
        ShellResponse::Ok { message } => format!("runtime keybindings reconciled: {message}"),
        ShellResponse::Error { message } => format!("runtime keybindings failed: {message}"),
        _ => "runtime keybindings reconciliation returned unexpected response".to_string(),
    };
    if record {
        record_reconcile_status(state, message);
    }
    if record {
        refresh_snapshot(state);
        push_state_changed(state, "reconcile");
    }
    response
}

fn reconcile_observe_only_subsystem(
    state: &Arc<Mutex<DaemonState>>,
    subsystem: ShellSubsystem,
    record: bool,
) -> ShellResponse {
    let mode = settings_snapshot(state).ownership(subsystem);
    let message =
        format!("{subsystem} ownership is {mode}; runtime reconciliation is observe-only");
    if record {
        record_reconcile_status(state, message.clone());
    }
    ShellResponse::ok(message)
}

fn record_reconcile_status(state: &Arc<Mutex<DaemonState>>, message: String) {
    if let Ok(mut state) = state.lock() {
        state.last_reconcile_status = Some(message);
        state.reconcile = state
            .last_reconcile_status
            .clone()
            .map(|summary| ReconcileStatus {
                severity: ReconcileSeverity::Ok,
                summary,
                results: Vec::new(),
            });
        if let Some(status) = state.last_reconcile_status.clone() {
            push_event(&mut state, ShellEvent::Reconcile { status });
        }
    }
}

fn record_reconcile_status_structured(state: &Arc<Mutex<DaemonState>>, reconcile: ReconcileStatus) {
    if let Ok(mut state) = state.lock() {
        state.last_reconcile_status = Some(reconcile.summary.clone());
        state.reconcile = Some(reconcile);
        if let Some(status) = state.last_reconcile_status.clone() {
            push_event(&mut state, ShellEvent::Reconcile { status });
        }
    }
}

fn reconcile_result(
    subsystem: ShellSubsystem,
    response: &ShellResponse,
    implemented: bool,
) -> SubsystemReconcileStatus {
    let severity = match response {
        ShellResponse::Error { .. } => ReconcileSeverity::Error,
        _ if !implemented => ReconcileSeverity::Warning,
        _ => ReconcileSeverity::Ok,
    };
    SubsystemReconcileStatus {
        subsystem,
        severity,
        implemented,
        message: response_message(response),
    }
}

fn subsystem_reconcile_implemented(subsystem: ShellSubsystem) -> bool {
    matches!(subsystem, ShellSubsystem::Keybindings)
}

fn reconcile_severity_for_results(results: &[SubsystemReconcileStatus]) -> ReconcileSeverity {
    if results
        .iter()
        .any(|result| result.severity == ReconcileSeverity::Error)
    {
        ReconcileSeverity::Error
    } else if results
        .iter()
        .any(|result| result.severity == ReconcileSeverity::Warning)
    {
        ReconcileSeverity::Warning
    } else {
        ReconcileSeverity::Ok
    }
}

fn response_message(response: &ShellResponse) -> String {
    match response {
        ShellResponse::Ok { message } | ShellResponse::Error { message } => message.clone(),
        _ => "unexpected reconcile response".to_string(),
    }
}

fn snapshot_with_runtime(
    state: &Arc<Mutex<DaemonState>>,
    runtime: &impl DaemonRuntime,
) -> ShellSnapshot {
    runtime.refresh_snapshot(state);
    state
        .lock()
        .map(|state| state.snapshot.clone())
        .unwrap_or_default()
}

fn refresh_snapshot(state: &Arc<Mutex<DaemonState>>) {
    let mut snapshot = load_hyprland_snapshot();
    snapshot.audio = hyprbole_core::audio::snapshot();
    snapshot.audio_summary = snapshot.audio.summary.clone();
    snapshot.brightness = hyprbole_core::brightness::snapshot();
    snapshot.brightness_summary = snapshot.brightness.summary.clone();
    if let Ok(mut state) = state.lock() {
        snapshot.last_hyprland_event = state.last_hyprland_event.clone();
        snapshot.last_reconcile_status = state.last_reconcile_status.clone();
        snapshot.reconcile = state.reconcile.clone();
        snapshot.settings = state.settings.clone();
        snapshot.theme = hyprbole_core::theme::snapshot(&state.settings.theme);
        snapshot.notifications = hyprbole_core::notifications::snapshot(
            &state.settings.notifications,
            &state.notifications,
        );
        state.snapshot = snapshot;
    }
}

fn load_hyprland_snapshot() -> ShellSnapshot {
    hyprbole_core::hyprland_state::snapshot()
}

#[derive(Deserialize)]
struct HyprBind {
    key: String,
    #[serde(default)]
    modmask: i64,
    #[serde(default)]
    release: bool,
    #[serde(default)]
    dispatcher: String,
}

fn load_keybinding_state(settings: ShellSettings) -> Vec<KeybindingSnapshot> {
    let binds: Vec<HyprBind> = command_output("hyprctl", &["binds", "-j"])
        .and_then(|output| serde_json::from_str(&output).ok())
        .unwrap_or_default();
    [
        ("mouse_drag", "Super + left mouse drag", "mouse:272"),
        ("mouse_resize", "Super + right mouse resize", "mouse:273"),
    ]
    .into_iter()
    .map(|(id, description, key)| {
        let active = binds.iter().any(|bind| {
            bind.key == key
                && bind.modmask & 64 == 64
                && !bind.release
                && bind.dispatcher == "__lua"
        });
        KeybindingSnapshot {
            id: id.to_string(),
            description: description.to_string(),
            expected: format!("SUPER + {key}"),
            active,
            source: if active {
                if settings
                    .ownership(ShellSubsystem::Keybindings)
                    .controls_runtime()
                {
                    KeybindingSource::HyprboleRuntime
                } else {
                    KeybindingSource::UserConfig
                }
            } else {
                KeybindingSource::Missing
            },
        }
    })
    .collect()
}

fn command_output(command: &str, args: &[&str]) -> Option<String> {
    let started = Instant::now();
    let output = Command::new(command).args(args).output().ok()?;
    let elapsed = started.elapsed();
    if elapsed > Duration::from_millis(100) {
        log_daemon(
            "slow_command",
            format!("{command} {args:?} took {}ms", elapsed.as_millis()),
        );
    }
    if !output.status.success() {
        log_daemon("command_failed", format!("{command} {args:?}"));
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn test_state() -> Arc<Mutex<DaemonState>> {
        Arc::new(Mutex::new(DaemonState::new(
            ShellSettings::default(),
            "/tmp/hyprbole.sock".to_string(),
            std::path::PathBuf::from("/tmp/settings.json"),
        )))
    }

    struct MockRuntime {
        action_response: ShellResponse,
        save_ok: bool,
        saved_settings: RefCell<Vec<ShellSettings>>,
        saved_paths: RefCell<Vec<String>>,
        refreshed: Cell<bool>,
        keybindings: Vec<KeybindingSnapshot>,
    }

    impl Default for MockRuntime {
        fn default() -> Self {
            Self {
                action_response: ShellResponse::ok("mock action completed"),
                save_ok: true,
                saved_settings: RefCell::new(Vec::new()),
                saved_paths: RefCell::new(Vec::new()),
                refreshed: Cell::new(false),
                keybindings: Vec::new(),
            }
        }
    }

    impl DaemonRuntime for MockRuntime {
        fn dispatch_action(&self, _action: ShellAction) -> ShellResponse {
            self.action_response.clone()
        }

        fn refresh_snapshot(&self, state: &Arc<Mutex<DaemonState>>) {
            self.refreshed.set(true);
            let settings = settings_snapshot(state);
            if let Ok(mut state) = state.lock() {
                state.snapshot.settings = settings;
            }
        }

        fn load_keybindings(&self, _settings: ShellSettings) -> Vec<KeybindingSnapshot> {
            self.keybindings.clone()
        }

        fn save_settings(
            &self,
            settings_path: &Path,
            settings: &ShellSettings,
        ) -> Result<(), Box<dyn std::error::Error>> {
            if self.save_ok {
                self.saved_paths
                    .borrow_mut()
                    .push(settings_path.display().to_string());
                self.saved_settings.borrow_mut().push(settings.clone());
                Ok(())
            } else {
                Err("mock save failed".into())
            }
        }
    }

    fn handle_with_mock(
        state: &Arc<Mutex<DaemonState>>,
        runtime: &MockRuntime,
        request: ShellRequest,
    ) -> ShellResponse {
        let (response, shutdown) = handle_shell_request_with_runtime(request, state, runtime);
        assert!(!shutdown);
        response
    }

    #[test]
    fn events_snapshot_filters_after_cursor() {
        let state = test_state();
        {
            let mut state = state.lock().unwrap();
            push_event(
                &mut state,
                ShellEvent::StateChanged {
                    reason: "a".into(),
                    domain: None,
                },
            );
            push_event(
                &mut state,
                ShellEvent::StateChanged {
                    reason: "b".into(),
                    domain: None,
                },
            );
        }

        let events = events_snapshot(&state, Some(1));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, 2);
    }

    #[test]
    fn events_snapshot_reports_gap_for_rotated_cursor_zero() {
        let state = test_state();
        {
            let mut state = state.lock().unwrap();
            state.next_event_id = 42;
            push_event(
                &mut state,
                ShellEvent::StateChanged {
                    reason: "new".into(),
                    domain: None,
                },
            );
        }

        let events = events_snapshot(&state, Some(0));
        assert!(matches!(
            events.first().map(|event| &event.event),
            Some(ShellEvent::Gap {
                after: 0,
                oldest: 42
            })
        ));
    }

    #[test]
    fn handler_ui_settings_get_uses_in_memory_settings() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(&state, &runtime, ShellRequest::UiSettingsGet);

        let ShellResponse::UiSettings { settings } = response else {
            panic!("expected ui settings response");
        };
        assert_eq!(settings.bar.height, 36);
    }

    #[test]
    fn handler_theme_get_uses_in_memory_settings() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(&state, &runtime, ShellRequest::ThemeGet);

        let ShellResponse::Theme { snapshot } = response else {
            panic!("expected theme response");
        };
        assert_eq!(snapshot.mode, ThemeMode::Dark);
        assert_eq!(snapshot.tokens.density.bar_height, 36);
    }

    #[test]
    fn handler_theme_set_mode_persists_setting_and_emits_event() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::ThemeSetMode {
                mode: ThemeMode::Light,
            },
        );

        assert!(response.is_ok());
        assert_eq!(settings_snapshot(&state).theme.mode, ThemeMode::Light);
        assert_eq!(runtime.saved_settings.borrow().len(), 1);
        assert!(events_snapshot(&state, None).iter().any(|event| matches!(
            &event.event,
            ShellEvent::StateChanged { domain, .. } if domain.as_deref() == Some("theme")
        )));
    }

    #[test]
    fn handler_notifications_get_uses_in_memory_state() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let _ = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::ActionCall {
                action: ShellAction::Notifications {
                    action: NotificationAction::Push {
                        summary: "hello".to_string(),
                        body: String::new(),
                        urgency: NotificationUrgency::Normal,
                    },
                },
            },
        );
        let response = handle_with_mock(&state, &runtime, ShellRequest::NotificationsGet);

        let ShellResponse::Notifications { snapshot } = response else {
            panic!("expected notifications response");
        };
        assert_eq!(snapshot.unread_count, 1);
        assert_eq!(snapshot.recent[0].summary, "hello");
    }

    #[test]
    fn notification_dnd_toggle_persists_setting_and_emits_event() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::ActionCall {
                action: ShellAction::Notifications {
                    action: NotificationAction::ToggleDnd,
                },
            },
        );

        assert!(response.is_ok());
        assert!(settings_snapshot(&state).notifications.dnd_enabled);
        assert_eq!(runtime.saved_settings.borrow().len(), 1);
        assert!(events_snapshot(&state, None).iter().any(|event| matches!(
            &event.event,
            ShellEvent::StateChanged { domain, .. } if domain.as_deref() == Some("notifications")
        )));
    }

    #[test]
    fn notification_clear_history_removes_records() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let _ = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::ActionCall {
                action: ShellAction::Notifications {
                    action: NotificationAction::Push {
                        summary: "hello".to_string(),
                        body: String::new(),
                        urgency: NotificationUrgency::Normal,
                    },
                },
            },
        );
        let response = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::ActionCall {
                action: ShellAction::Notifications {
                    action: NotificationAction::ClearHistory,
                },
            },
        );

        assert!(response.is_ok());
        assert_eq!(notifications_snapshot(&state).recent.len(), 0);
    }

    #[test]
    fn handler_state_get_uses_mock_snapshot_refresh() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(&state, &runtime, ShellRequest::StateGet);

        let ShellResponse::State { snapshot } = response else {
            panic!("expected state response");
        };
        assert!(runtime.refreshed.get());
        assert_eq!(snapshot.settings.ui.bar.height, 36);
    }

    #[test]
    fn handler_settings_get_uses_in_memory_settings() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(&state, &runtime, ShellRequest::SettingsGet);

        let ShellResponse::Settings { settings } = response else {
            panic!("expected settings response");
        };
        assert_eq!(settings.ownership.keybindings, OwnershipMode::RuntimeOwned);
    }

    #[test]
    fn handler_status_get_reports_mock_state_metadata() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(&state, &runtime, ShellRequest::StatusGet);

        let ShellResponse::Status { daemon } = response else {
            panic!("expected status response");
        };
        assert_eq!(daemon.socket_path, "/tmp/hyprbole.sock");
        assert_eq!(daemon.next_event_id, 1);
    }

    #[test]
    fn handler_events_subscribe_returns_recent_events() {
        let state = test_state();
        let runtime = MockRuntime::default();
        push_state_changed_domain(&state, "test_changed", "settings");
        let response = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::EventsSubscribe { after: Some(0) },
        );

        let ShellResponse::Events { events } = response else {
            panic!("expected events response");
        };
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, 1);
    }

    #[test]
    fn handler_reconcile_monitor_is_observe_only() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::Reconcile {
                subsystem: Some(ShellSubsystem::Monitors),
            },
        );

        assert!(response.is_ok());
        assert!(
            daemon_status(&state)
                .last_reconcile_status
                .is_some_and(|status| status.contains("observe-only"))
        );
    }

    #[test]
    fn handler_bar_settings_field_persists_and_emits_settings_event() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::BarSettingsSetField {
                field: "bar.height".to_string(),
                value: "44".to_string(),
            },
        );

        assert!(response.is_ok());
        assert_eq!(settings_snapshot(&state).ui.bar.height, 44);
        assert_eq!(runtime.saved_settings.borrow().len(), 1);
        assert_eq!(runtime.saved_paths.borrow()[0], "/tmp/settings.json");
        assert!(events_snapshot(&state, None).iter().any(|event| matches!(
            &event.event,
            ShellEvent::StateChanged { reason, domain }
                if reason == "bar_settings_changed" && domain.as_deref() == Some("settings")
        )));
    }

    #[test]
    fn handler_bar_settings_reset_restores_defaults() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let _ = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::BarSettingsSetField {
                field: "bar.height".to_string(),
                value: "44".to_string(),
            },
        );
        let response = handle_with_mock(&state, &runtime, ShellRequest::BarSettingsReset);

        assert!(response.is_ok());
        assert_eq!(settings_snapshot(&state).ui.bar.height, 36);
        assert_eq!(runtime.saved_settings.borrow().len(), 2);
    }

    #[test]
    fn handler_bar_settings_save_failure_does_not_mutate_memory() {
        let state = test_state();
        let runtime = MockRuntime {
            save_ok: false,
            ..MockRuntime::default()
        };
        let response = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::BarSettingsSetField {
                field: "bar.height".to_string(),
                value: "44".to_string(),
            },
        );

        assert!(!response.is_ok());
        assert_eq!(settings_snapshot(&state).ui.bar.height, 36);
        assert!(events_snapshot(&state, None).is_empty());
    }

    #[test]
    fn handler_osd_settings_field_persists_and_emits_settings_event() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::OsdSettingsSetField {
                field: "osd.timeout_ms".to_string(),
                value: "2500".to_string(),
            },
        );

        assert!(response.is_ok());
        assert_eq!(settings_snapshot(&state).ui.osd.timeout_ms, 2500);
        assert_eq!(runtime.saved_settings.borrow().len(), 1);
        assert!(events_snapshot(&state, None).iter().any(|event| matches!(
            &event.event,
            ShellEvent::StateChanged { reason, domain }
                if reason == "osd_settings_changed" && domain.as_deref() == Some("settings")
        )));
    }

    #[test]
    fn handler_osd_settings_reset_restores_defaults() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let _ = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::OsdSettingsSetField {
                field: "osd.width".to_string(),
                value: "480".to_string(),
            },
        );
        let response = handle_with_mock(&state, &runtime, ShellRequest::OsdSettingsReset);

        assert!(response.is_ok());
        assert_eq!(settings_snapshot(&state).ui.osd.width, 420);
        assert_eq!(runtime.saved_settings.borrow().len(), 2);
    }

    #[test]
    fn handler_osd_settings_save_failure_does_not_mutate_memory() {
        let state = test_state();
        let runtime = MockRuntime {
            save_ok: false,
            ..MockRuntime::default()
        };
        let response = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::OsdSettingsSetField {
                field: "osd.width".to_string(),
                value: "480".to_string(),
            },
        );

        assert!(!response.is_ok());
        assert_eq!(settings_snapshot(&state).ui.osd.width, 420);
        assert!(events_snapshot(&state, None).is_empty());
    }

    #[test]
    fn handler_action_call_records_action_event_and_refreshes_snapshot() {
        let state = test_state();
        let runtime = MockRuntime::default();
        let response = handle_with_mock(
            &state,
            &runtime,
            ShellRequest::ActionCall {
                action: ShellAction::Audio {
                    action: hyprbole_core::audio::AudioAction::ToggleMute,
                },
            },
        );

        assert!(response.is_ok());
        assert!(runtime.refreshed.get());
        let status = daemon_status(&state);
        let action = status.last_action.expect("last action");
        assert_eq!(action.domain, "audio");
        assert_eq!(action.name, "toggle_mute");
        assert!(
            events_snapshot(&state, None)
                .iter()
                .any(|event| { matches!(event.event, ShellEvent::Action { .. }) })
        );
    }

    #[test]
    fn handler_binds_get_uses_mock_keybindings() {
        let state = test_state();
        let runtime = MockRuntime {
            keybindings: vec![KeybindingSnapshot {
                id: "mock".to_string(),
                description: "Mock bind".to_string(),
                expected: "SUPER + M".to_string(),
                active: true,
                source: KeybindingSource::HyprboleRuntime,
            }],
            ..MockRuntime::default()
        };
        let response = handle_with_mock(&state, &runtime, ShellRequest::BindsGet);

        let ShellResponse::Binds { keybindings } = response else {
            panic!("expected binds response");
        };
        assert_eq!(keybindings.len(), 1);
        assert_eq!(keybindings[0].id, "mock");
    }

    #[test]
    fn ownership_persist_failure_does_not_mutate_memory() {
        let state = test_state();
        let result = persist_ownership_settings(
            &state,
            ShellSubsystem::Monitors,
            OwnershipMode::RuntimeOwned,
            |_, _| Err("save failed".into()),
        );

        assert!(result.is_err());
        let state = state.lock().unwrap();
        assert_eq!(
            state.settings.ownership(ShellSubsystem::Monitors),
            OwnershipMode::RespectUserConfig
        );
    }

    #[test]
    fn ownership_persist_uses_state_settings_path() {
        let state = test_state();
        let mut captured_path = String::new();
        persist_ownership_settings(
            &state,
            ShellSubsystem::Monitors,
            OwnershipMode::RuntimeOwned,
            |path, _| {
                captured_path = path.display().to_string();
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(captured_path, "/tmp/settings.json");
    }

    #[test]
    fn ownership_persist_success_updates_memory() {
        let state = test_state();
        let old_mode = persist_ownership_settings(
            &state,
            ShellSubsystem::Monitors,
            OwnershipMode::RuntimeOwned,
            |_, _| Ok(()),
        )
        .unwrap();

        assert_eq!(old_mode, OwnershipMode::RespectUserConfig);
        let state = state.lock().unwrap();
        assert_eq!(
            state.settings.ownership(ShellSubsystem::Monitors),
            OwnershipMode::RuntimeOwned
        );
    }

    #[test]
    fn unsupported_persisted_ownership_is_rejected_by_daemon() {
        let state = test_state();
        let response = set_ownership(
            &state,
            ShellSubsystem::Brightness,
            OwnershipMode::PersistedOwned,
        )
        .unwrap();

        assert!(!response.is_ok());
        let state = state.lock().unwrap();
        assert_ne!(
            state.settings.ownership(ShellSubsystem::Brightness),
            OwnershipMode::PersistedOwned
        );
    }

    #[test]
    fn bar_settings_update_mutates_memory_after_validation() {
        let state = test_state();
        let mut settings = settings_snapshot(&state).ui.bar;
        settings.set_field("bar.height", "40").unwrap();
        let response = persist_bar_settings(&state, settings, |_, _| Ok(())).unwrap();
        assert!(response.is_ok());
        assert_eq!(settings_snapshot(&state).ui.bar.height, 40);
    }

    #[test]
    fn bar_settings_update_rejects_invalid_height() {
        let state = test_state();
        let mut settings = settings_snapshot(&state).ui.bar;
        let response = settings.set_field("bar.height", "4");
        assert!(response.is_err());
        assert_eq!(settings_snapshot(&state).ui.bar.height, 36);
    }

    #[test]
    fn status_reports_daemon_metadata() {
        let state = test_state();
        let status = daemon_status(&state);

        assert_eq!(status.pid, std::process::id());
        assert_eq!(status.socket_path, "/tmp/hyprbole.sock");
        assert_eq!(status.settings_path, "/tmp/settings.json");
        assert_eq!(status.next_event_id, 1);
        assert!(status.last_action.is_none());
    }

    #[test]
    fn action_status_records_structured_status_and_event() {
        let state = test_state();
        record_action_status(
            &state,
            action_status(
                "audio",
                "toggle_mute",
                &ShellResponse::ok("audio action completed"),
            ),
        );

        let status = daemon_status(&state);
        let action = status.last_action.expect("last action");
        assert_eq!(action.domain, "audio");
        assert_eq!(action.name, "toggle_mute");
        assert!(action.ok);
        assert_eq!(
            status.last_action_status.as_deref(),
            Some("ok: audio action completed")
        );
        assert!(matches!(
            events_snapshot(&state, None)
                .first()
                .map(|event| &event.event),
            Some(ShellEvent::Action { .. })
        ));
    }

    #[test]
    fn monitor_reconcile_is_observe_only() {
        let state = test_state();
        let response = reconcile_runtime_state(&state, Some(ShellSubsystem::Monitors));

        assert!(matches!(response, ShellResponse::Ok { .. }));
        let status = daemon_status(&state);
        assert!(
            status.last_reconcile_status.is_some_and(
                |status| status.contains("monitors") && status.contains("observe-only")
            )
        );
        let events = events_snapshot(&state, None)
            .into_iter()
            .filter(|event| matches!(event.event, ShellEvent::Reconcile { .. }))
            .count();
        assert_eq!(events, 1);
    }

    #[test]
    fn full_reconcile_reports_observe_only_workspaces() {
        let state = test_state();
        let response = reconcile_runtime_state(&state, None);

        let ShellResponse::Ok { message } = response else {
            panic!("expected reconcile ok response");
        };
        assert!(message.contains("workspaces"));
        assert!(message.contains("observe-only"));

        let status = daemon_status(&state);
        let reconcile = status.reconcile.expect("structured reconcile status");
        assert_eq!(reconcile.severity, ReconcileSeverity::Warning);
        assert_eq!(reconcile.results.len(), 3);
        assert!(reconcile.results.iter().any(|result| {
            result.subsystem == ShellSubsystem::Workspaces
                && !result.implemented
                && result.severity == ReconcileSeverity::Warning
        }));
    }

    #[test]
    fn full_reconcile_error_status_overrides_later_observe_only_results() {
        let state = test_state();
        let response = finish_full_reconcile(
            &state,
            vec![
                (
                    ShellSubsystem::Keybindings,
                    ShellResponse::error("keybinding failure"),
                ),
                (
                    ShellSubsystem::Monitors,
                    ShellResponse::ok("monitors observe-only"),
                ),
            ],
        );

        assert!(!response.is_ok());
        let status = daemon_status(&state);
        let reconcile = status.reconcile.expect("structured reconcile status");
        assert_eq!(reconcile.severity, ReconcileSeverity::Error);
        assert!(reconcile.summary.contains("full reconcile failed"));
        assert!(
            reconcile
                .results
                .iter()
                .any(|result| result.subsystem == ShellSubsystem::Keybindings
                    && result.severity == ReconcileSeverity::Error)
        );
    }

    #[test]
    fn keybinding_release_failure_records_error_severity() {
        let state = test_state();
        record_keybinding_release_status(
            &state,
            runtime_keybinding_release_message(&ShellResponse::error("unbind failed")),
            false,
        );

        let status = daemon_status(&state);
        let reconcile = status.reconcile.expect("structured release status");
        assert_eq!(reconcile.severity, ReconcileSeverity::Error);
        assert!(
            reconcile
                .summary
                .contains("runtime keybindings release failed")
        );
    }
}
