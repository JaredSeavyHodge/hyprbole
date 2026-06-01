use std::process::{Command, Stdio};
use std::{
    num::NonZeroU32,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_client::{
    Connection, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
};

use hyprbole_core::settings::{BarEdge, BarMonitor, BarSettings, BarWidget};
use hyprbole_core::theme::ColorTokens;

const WIDTH: u32 = 720;
const HEIGHT: u32 = 36;
const WORKSPACE_HIT_PADDING: u32 = 3;

pub fn run() -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|err| format!("connect Wayland: {err}"))?;
    let (globals, mut event_queue) =
        registry_queue_init(&conn).map_err(|err| format!("init registry: {err}"))?;
    let qh = event_queue.handle();

    let compositor =
        CompositorState::bind(&globals, &qh).map_err(|err| format!("bind wl_compositor: {err}"))?;
    let layer_shell =
        LayerShell::bind(&globals, &qh).map_err(|err| format!("bind layer shell: {err}"))?;
    let shm = Shm::bind(&globals, &qh).map_err(|err| format!("bind wl_shm: {err}"))?;

    let surface = compositor.create_surface(&qh);
    let layer =
        layer_shell.create_layer_surface(&qh, surface, Layer::Top, Some("hyprbole-bar"), None);
    configure_layer_surface(&layer, BarEdge::Top, HEIGHT, 0);

    let pool = SlotPool::new((WIDTH * HEIGHT * 4) as usize, &shm)
        .map_err(|err| format!("create shm pool: {err}"))?;
    let mut app = Bar {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor,
        layer_shell,
        shm,
        pool,
        layer,
        width: WIDTH,
        height: HEIGHT,
        daemon_connected: false,
        workspaces: Vec::new(),
        active_window: String::new(),
        audio_summary: String::new(),
        brightness_summary: String::new(),
        current_layout: String::new(),
        dnd_enabled: false,
        notification_unread: 0,
        colors: BarColors::default(),
        last_action: String::new(),
        reconcile_status: String::new(),
        reconcile_severity: String::new(),
        settings: BarSettings::default(),
        daemon_monitors: Vec::new(),
        selected_output_name: None,
        last_output_resolution: String::new(),
        force_layer_recreate: false,
        last_daemon_update: None,
        event_rx: spawn_event_listener(),
        action_tx: spawn_action_worker(),
        pointer: None,
        first_configure: true,
        exit: false,
    };

    app.seed_daemon_state();

    while !app.exit {
        dispatch_wayland(&mut event_queue, &mut app)?;
        let daemon_changed = app.consume_daemon_events();
        if daemon_changed || app.daemon_refresh_due() {
            let _ = app.draw(&qh);
        }
    }
    Ok(())
}

fn dispatch_wayland(
    event_queue: &mut wayland_client::EventQueue<Bar>,
    app: &mut Bar,
) -> Result<(), String> {
    if app.first_configure {
        event_queue
            .blocking_dispatch(app)
            .map_err(|err| format!("dispatch Wayland event: {err}"))?;
        return Ok(());
    }

    event_queue
        .dispatch_pending(app)
        .map_err(|err| format!("dispatch Wayland event: {err}"))?;
    event_queue
        .flush()
        .map_err(|err| format!("flush Wayland connection: {err}"))?;

    let Some(read_guard) = event_queue.prepare_read() else {
        return Ok(());
    };
    let mut fds = [PollFd::new(event_queue, PollFlags::IN)];
    let timeout = Timespec {
        tv_sec: 0,
        tv_nsec: 50_000_000,
    };
    let ready =
        poll(&mut fds, Some(&timeout)).map_err(|err| format!("poll Wayland socket: {err}"))?;
    if ready > 0 && fds[0].revents().contains(PollFlags::IN) {
        read_guard
            .read()
            .map_err(|err| format!("read Wayland events: {err}"))?;
        event_queue
            .dispatch_pending(app)
            .map_err(|err| format!("dispatch Wayland event: {err}"))?;
    } else {
        drop(read_guard);
    }
    Ok(())
}

fn spawn_event_listener() -> Receiver<()> {
    let (tx, rx) = mpsc::channel();
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
                    let _ = tx.send(());
                }
            }
            thread::sleep(Duration::from_millis(250));
        }
    });
    rx
}

fn spawn_action_worker() -> Sender<BarAction> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut throttle = ActionThrottle::default();
        while let Ok(action) = rx.recv() {
            if !throttle.should_dispatch(&action, Instant::now()) {
                continue;
            }
            let result = match action {
                BarAction::FocusWorkspace(workspace) => focus_workspace(&workspace),
                BarAction::FocusRelativeWorkspace(direction) => focus_relative_workspace(direction),
                BarAction::ToggleMute => {
                    dispatch_shell_action(hyprbole_core::daemon::ShellAction::Audio {
                        action: hyprbole_core::audio::AudioAction::ToggleMute,
                    })
                }
                BarAction::Brightness(direction) => {
                    dispatch_shell_action(hyprbole_core::daemon::ShellAction::Brightness {
                        action: match direction {
                            BrightnessDirection::Increase => {
                                hyprbole_core::brightness::BrightnessAction::Increase
                            }
                            BrightnessDirection::Decrease => {
                                hyprbole_core::brightness::BrightnessAction::Decrease
                            }
                        },
                    })
                }
                BarAction::OpenQuickSettings => toggle_quick_settings(),
            };
            if let Err(err) = result {
                eprintln!("bar action failed: {err}");
            }
        }
    });
    tx
}

struct Bar {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,
    pool: SlotPool,
    layer: LayerSurface,
    width: u32,
    height: u32,
    daemon_connected: bool,
    workspaces: Vec<WorkspaceButton>,
    active_window: String,
    audio_summary: String,
    brightness_summary: String,
    current_layout: String,
    dnd_enabled: bool,
    notification_unread: usize,
    colors: BarColors,
    last_action: String,
    reconcile_status: String,
    reconcile_severity: String,
    settings: BarSettings,
    daemon_monitors: Vec<DaemonMonitorTarget>,
    selected_output_name: Option<String>,
    last_output_resolution: String,
    force_layer_recreate: bool,
    last_daemon_update: Option<Instant>,
    event_rx: Receiver<()>,
    action_tx: Sender<BarAction>,
    pointer: Option<wl_pointer::WlPointer>,
    first_configure: bool,
    exit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DaemonMonitorTarget {
    name: String,
    focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WaylandOutputTarget {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BarOutputResolution {
    requested: String,
    daemon_monitor: Option<String>,
    output_name: Option<String>,
    reason: String,
}

#[derive(Clone, Copy)]
struct BarColors {
    surface: u32,
    surface_muted: u32,
    text: u32,
    accent: u32,
    accent_text: u32,
    border: u32,
    warning: u32,
    success: u32,
    error: u32,
}

impl Default for BarColors {
    fn default() -> Self {
        Self {
            surface: 0xff161922,
            surface_muted: 0xff252a33,
            text: 0xffdce6ff,
            accent: 0xff8baaff,
            accent_text: 0xff0b1020,
            border: 0xff2f3a4d,
            warning: 0xfff6ca6c,
            success: 0xff7ee787,
            error: 0xff2f2630,
        }
    }
}

impl BarColors {
    fn from_tokens(tokens: &ColorTokens) -> Self {
        let fallback = Self::default();
        Self {
            surface: argb_from_hex(&tokens.surface).unwrap_or(fallback.surface),
            surface_muted: argb_from_hex(&tokens.surface_muted).unwrap_or(fallback.surface_muted),
            text: argb_from_hex(&tokens.text).unwrap_or(fallback.text),
            accent: argb_from_hex(&tokens.accent).unwrap_or(fallback.accent),
            accent_text: argb_from_hex(&tokens.accent_text).unwrap_or(fallback.accent_text),
            border: argb_from_hex(&tokens.border).unwrap_or(fallback.border),
            warning: argb_from_hex(&tokens.warning).unwrap_or(fallback.warning),
            success: argb_from_hex(&tokens.success).unwrap_or(fallback.success),
            error: argb_from_hex(&tokens.error).unwrap_or(fallback.error),
        }
    }

    fn reconcile(self, severity: &str) -> u32 {
        match severity {
            "ERR" => self.error,
            "WARN" => self.warning,
            "OK" => self.success,
            _ => self.text,
        }
    }
}

fn argb_from_hex(value: &str) -> Option<u32> {
    let value = value.strip_prefix('#')?;
    if value.len() != 6 {
        return None;
    }
    u32::from_str_radix(value, 16)
        .ok()
        .map(|rgb| 0xff00_0000 | rgb)
}

fn configure_layer_surface(layer: &LayerSurface, edge: BarEdge, height: u32, margin: u32) {
    layer.set_anchor(match edge {
        BarEdge::Top => Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
        BarEdge::Bottom => Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
    });
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    match edge {
        BarEdge::Top => layer.set_margin(margin as i32, 0, 0, 0),
        BarEdge::Bottom => layer.set_margin(0, 0, margin as i32, 0),
    }
    layer.set_exclusive_zone((height + margin) as i32);
    layer.set_size(0, height);
    layer.commit();
}

fn resolve_bar_output(
    monitor: &BarMonitor,
    daemon_monitors: &[DaemonMonitorTarget],
    wayland_outputs: &[WaylandOutputTarget],
) -> BarOutputResolution {
    let requested = monitor.to_string();
    let daemon_monitor = match monitor {
        BarMonitor::Focused => daemon_monitors
            .iter()
            .find(|monitor| monitor.focused)
            .or_else(|| daemon_monitors.first())
            .map(|monitor| monitor.name.clone()),
        BarMonitor::Primary => daemon_monitors.first().map(|monitor| monitor.name.clone()),
        BarMonitor::Named(name) => Some(name.clone()),
    };
    if let Some(name) = daemon_monitor.as_deref()
        && let Some(output) = find_output_by_name(name, wayland_outputs)
    {
        return BarOutputResolution {
            requested,
            daemon_monitor,
            output_name: output.name.clone(),
            reason: "matched requested monitor to Wayland output".to_string(),
        };
    }
    let fallback = wayland_outputs
        .iter()
        .find(|output| output.name.is_some())
        .and_then(|output| output.name.clone());
    let reason = if wayland_outputs.is_empty() {
        "no Wayland outputs reported yet; using compositor default output"
    } else if daemon_monitor.is_some() {
        "requested monitor not matched to Wayland output; using first output fallback"
    } else {
        "requested monitor unavailable from daemon; using first output fallback"
    };
    BarOutputResolution {
        requested,
        daemon_monitor,
        output_name: fallback,
        reason: reason.to_string(),
    }
}

fn find_output_by_name<'a>(
    name: &str,
    wayland_outputs: &'a [WaylandOutputTarget],
) -> Option<&'a WaylandOutputTarget> {
    wayland_outputs.iter().find(|output| {
        output.name.as_deref() == Some(name) || output.description.as_deref() == Some(name)
    })
}

#[cfg(test)]
fn output_recreation_needed(
    previous: Option<&str>,
    next: Option<&str>,
    settings_changed: bool,
    force: bool,
) -> bool {
    force || previous != next || settings_changed
}

impl Bar {
    fn consume_daemon_events(&mut self) -> bool {
        let mut changed = false;
        while self.event_rx.try_recv().is_ok() {
            changed = true;
        }
        if changed {
            self.last_daemon_update = None;
            self.update_daemon_state();
        }
        changed
    }

    fn daemon_refresh_due(&self) -> bool {
        self.last_daemon_update
            .is_none_or(|last| last.elapsed() >= Duration::from_secs(1))
    }

    fn seed_daemon_state(&mut self) {
        self.last_daemon_update = None;
        self.update_daemon_state();
    }

    fn draw(&mut self, qh: &QueueHandle<Self>) -> Result<(), String> {
        self.update_daemon_state();
        if self.recreate_layer_if_output_changed(qh) {
            return Ok(());
        }
        if !self.settings.enabled {
            self.exit = true;
            return Ok(());
        }
        if self.height != self.settings.height {
            self.height = self.settings.height;
            resize_workspace_hit_regions(&mut self.workspaces, self.height);
        }
        configure_layer_surface(
            &self.layer,
            self.settings.edge,
            self.height,
            self.settings.margin,
        );
        let needed = (self.width * self.height * 4) as usize;
        if self.pool.len() < needed {
            self.pool
                .resize(needed)
                .map_err(|err| format!("resize shm pool: {err}"))?;
        }

        let stride = self.width as i32 * 4;
        let (buffer, canvas) = self
            .pool
            .create_buffer(
                self.width as i32,
                self.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .map_err(|err| format!("create buffer: {err}"))?;

        for (index, pixel) in canvas.chunks_exact_mut(4).enumerate() {
            let x = index as u32 % self.width;
            let layout = BarLayout::new_with_settings(self.width, self.height, &self.settings);
            let workspace_band = if self.settings.widget_enabled(BarWidget::Workspaces) {
                layout.workspace.width
            } else {
                0
            };
            let color = if !self.daemon_connected {
                self.colors.error
            } else if x >= layout.workspace.x
                && x < layout.workspace.x.saturating_add(workspace_band)
            {
                self.colors.accent
            } else if x >= layout.system.x {
                self.colors.surface_muted
            } else {
                self.colors.surface
            };
            pixel.copy_from_slice(&color.to_le_bytes());
        }

        let workspace_text = if !self.settings.widget_enabled(BarWidget::Workspaces) {
            String::new()
        } else if self.workspaces.is_empty() {
            "WS --".to_string()
        } else {
            format!(
                "WS {}",
                self.workspaces
                    .iter()
                    .map(|workspace| workspace.label.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };
        let layout = BarLayout::new_with_settings(self.width, self.height, &self.settings);
        let window_text = if !self.settings.widget_enabled(BarWidget::FocusedWindow) {
            String::new()
        } else if self.active_window.is_empty() {
            "WIN --".to_string()
        } else {
            format!("WIN {}", truncate_text(&self.active_window, 22))
        };
        let status_text = if !self.settings.widget_enabled(BarWidget::DaemonStatus) {
            String::new()
        } else if !self.daemon_connected {
            "DAEMON --".to_string()
        } else if self.reconcile_status.is_empty() {
            "DAEMON OK".to_string()
        } else {
            format!(
                "{} {}",
                if self.reconcile_severity.is_empty() {
                    "SYNC"
                } else {
                    &self.reconcile_severity
                },
                truncate_text(&self.reconcile_status, 26)
            )
        };
        let right_text = system_text(
            &self.settings,
            &self.audio_summary,
            &self.brightness_summary,
            &self.current_layout,
            self.dnd_enabled,
            self.notification_unread,
        );
        let status_text = if !self.last_action.is_empty() {
            format!("ACT {}", truncate_text(&self.last_action, 24))
        } else {
            status_text
        };

        draw_vertical_rule(
            canvas,
            self.width,
            layout.window.x.saturating_sub(12),
            self.colors.border,
        );
        draw_vertical_rule(
            canvas,
            self.width,
            layout.system.x.saturating_sub(12),
            self.colors.border,
        );
        let text_y = self.height.saturating_sub(16) / 2;
        if !workspace_text.is_empty() {
            draw_text(
                canvas,
                self.width,
                layout.workspace_text_x(),
                text_y,
                &workspace_text,
                self.colors.accent_text,
            );
        }
        if !window_text.is_empty() {
            draw_text(
                canvas,
                self.width,
                layout.window.x,
                text_y,
                &window_text,
                self.colors.text,
            );
        }
        if !status_text.is_empty() {
            draw_text(
                canvas,
                self.width,
                layout.status.x,
                text_y,
                &status_text,
                self.colors.reconcile(&self.reconcile_severity),
            );
        }
        if !right_text.is_empty() {
            draw_text(
                canvas,
                self.width,
                layout.system.x,
                text_y,
                &right_text,
                self.colors.text,
            );
        }

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        buffer
            .attach_to(self.layer.wl_surface())
            .map_err(|err| format!("attach buffer: {err}"))?;
        self.layer.commit();
        Ok(())
    }

    fn update_daemon_state(&mut self) {
        if self
            .last_daemon_update
            .is_some_and(|last| last.elapsed() < Duration::from_secs(1))
        {
            return;
        }
        self.last_daemon_update = Some(Instant::now());
        let Ok(client) = hyprbole_core::daemon::DaemonClient::from_env() else {
            self.daemon_connected = false;
            self.workspaces.clear();
            self.active_window.clear();
            self.audio_summary.clear();
            self.brightness_summary.clear();
            self.current_layout.clear();
            self.last_action.clear();
            self.reconcile_status.clear();
            self.reconcile_severity = "ERR".to_string();
            return;
        };
        let Ok(hyprbole_core::daemon::ShellResponse::State { snapshot }) =
            client.send_shell(&hyprbole_core::daemon::ShellRequest::StateGet)
        else {
            self.daemon_connected = false;
            self.workspaces.clear();
            self.active_window.clear();
            self.audio_summary.clear();
            self.brightness_summary.clear();
            self.current_layout.clear();
            self.last_action.clear();
            self.reconcile_status.clear();
            self.reconcile_severity = "ERR".to_string();
            return;
        };
        self.daemon_connected = true;
        self.daemon_monitors = snapshot
            .monitors
            .iter()
            .map(|monitor| DaemonMonitorTarget {
                name: monitor.name.clone(),
                focused: monitor.focused,
            })
            .collect();
        self.workspaces = workspace_buttons(
            &snapshot
                .workspaces
                .iter()
                .map(|workspace| WorkspaceInput {
                    name: workspace.name.clone(),
                    focused: workspace.focused,
                    occupied: workspace.occupied,
                })
                .collect::<Vec<_>>(),
            BarLayout::new_with_settings(self.width, self.height, &snapshot.settings.ui.bar)
                .workspace_text_x()
                + text_width("WS "),
            self.height,
        );
        self.settings = snapshot.settings.ui.bar;
        self.active_window = snapshot.active_window;
        self.audio_summary = snapshot.audio_summary;
        self.brightness_summary = snapshot.brightness_summary;
        self.current_layout = snapshot.current_layout.unwrap_or_default();
        self.dnd_enabled = snapshot.notifications.dnd_enabled;
        self.notification_unread = snapshot.notifications.unread_count;
        self.colors = BarColors::from_tokens(&snapshot.theme.tokens.colors);
        self.reconcile_severity = snapshot
            .reconcile
            .as_ref()
            .map(|reconcile| match reconcile.severity {
                hyprbole_core::daemon::ReconcileSeverity::Ok => "OK".to_string(),
                hyprbole_core::daemon::ReconcileSeverity::Warning => "WARN".to_string(),
                hyprbole_core::daemon::ReconcileSeverity::Error => "ERR".to_string(),
            })
            .unwrap_or_default();
        self.reconcile_status = snapshot.last_reconcile_status.unwrap_or_default();
        if let Ok(hyprbole_core::daemon::ShellResponse::Status { daemon }) =
            client.send_shell(&hyprbole_core::daemon::ShellRequest::StatusGet)
        {
            self.last_action = daemon
                .last_action
                .map(|action| {
                    format!(
                        "{} {}.{}",
                        if action.ok { "OK" } else { "ERR" },
                        action.domain,
                        action.name
                    )
                })
                .unwrap_or_default();
        }
    }

    fn recreate_layer_if_output_changed(&mut self, qh: &QueueHandle<Self>) -> bool {
        let outputs = self.wayland_output_targets();
        let resolution =
            resolve_bar_output(&self.settings.monitor, &self.daemon_monitors, &outputs);
        let log_line = format!(
            "bar output target={} daemon={:?} output={:?} reason={}",
            resolution.requested,
            resolution.daemon_monitor,
            resolution.output_name,
            resolution.reason
        );
        if self.last_output_resolution != log_line {
            eprintln!("{log_line}");
            self.last_output_resolution = log_line;
        }
        if !self.force_layer_recreate && self.selected_output_name == resolution.output_name {
            return false;
        }
        self.force_layer_recreate = false;
        self.selected_output_name = resolution.output_name.clone();
        let output = self.selected_output_object();
        let surface = self.compositor.create_surface(qh);
        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Top,
            Some("hyprbole-bar"),
            output.as_ref(),
        );
        configure_layer_surface(
            &layer,
            self.settings.edge,
            self.height,
            self.settings.margin,
        );
        self.layer = layer;
        self.first_configure = true;
        true
    }

    fn wayland_output_targets(&self) -> Vec<WaylandOutputTarget> {
        self.output_state
            .outputs()
            .filter_map(|output| self.output_state.info(&output))
            .map(|info| WaylandOutputTarget {
                name: info.name,
                description: info.description,
            })
            .collect()
    }

    fn selected_output_object(&self) -> Option<wl_output::WlOutput> {
        let target = self.selected_output_name.as_deref()?;
        self.output_state.outputs().find(|output| {
            self.output_state
                .info(output)
                .and_then(|info| info.name)
                .as_deref()
                == Some(target)
        })
    }

    fn handle_click(&mut self, x: f64, y: f64) {
        match hit_bar_region_with_settings(
            &self.workspaces,
            self.width,
            self.height,
            &self.settings,
            x,
            y,
        ) {
            BarRegion::Workspace(workspace) => {
                let _ = self.action_tx.send(BarAction::FocusWorkspace(workspace));
            }
            BarRegion::Status => {
                let _ = self.action_tx.send(BarAction::OpenQuickSettings);
            }
            BarRegion::System => {
                let _ = self.action_tx.send(BarAction::ToggleMute);
            }
            BarRegion::Window | BarRegion::None => return,
        }
        self.last_daemon_update = None;
    }

    fn handle_scroll(
        &mut self,
        x: f64,
        y: f64,
        vertical: &smithay_client_toolkit::seat::pointer::AxisScroll,
    ) {
        let Some(direction) = scroll_direction(vertical) else {
            return;
        };
        match hit_bar_region_with_settings(
            &self.workspaces,
            self.width,
            self.height,
            &self.settings,
            x,
            y,
        ) {
            BarRegion::Workspace(_) => {
                let _ = self
                    .action_tx
                    .send(BarAction::FocusRelativeWorkspace(direction));
            }
            BarRegion::System => {
                let direction = match direction {
                    WorkspaceScrollDirection::Next => BrightnessDirection::Decrease,
                    WorkspaceScrollDirection::Previous => BrightnessDirection::Increase,
                };
                let _ = self.action_tx.send(BarAction::Brightness(direction));
            }
            BarRegion::Window | BarRegion::Status | BarRegion::None => return,
        }
        self.last_daemon_update = None;
    }
}

enum BarAction {
    FocusWorkspace(String),
    FocusRelativeWorkspace(WorkspaceScrollDirection),
    ToggleMute,
    Brightness(BrightnessDirection),
    OpenQuickSettings,
}

enum BrightnessDirection {
    Increase,
    Decrease,
}

#[derive(Default)]
struct ActionThrottle {
    workspace_scroll: Option<Instant>,
    brightness: Option<Instant>,
    mute: Option<Instant>,
    quick: Option<Instant>,
}

impl ActionThrottle {
    fn should_dispatch(&mut self, action: &BarAction, now: Instant) -> bool {
        let (slot, interval) = match action {
            BarAction::FocusRelativeWorkspace(_) => {
                (&mut self.workspace_scroll, Duration::from_millis(180))
            }
            BarAction::Brightness(_) => (&mut self.brightness, Duration::from_millis(160)),
            BarAction::ToggleMute => (&mut self.mute, Duration::from_millis(250)),
            BarAction::OpenQuickSettings => (&mut self.quick, Duration::from_millis(350)),
            BarAction::FocusWorkspace(_) => return true,
        };
        if slot.is_some_and(|last| now.duration_since(last) < interval) {
            return false;
        }
        *slot = Some(now);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceInput {
    name: String,
    focused: bool,
    occupied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceButton {
    name: String,
    label: String,
    rect: HitRect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BarRegion {
    Workspace(String),
    Window,
    Status,
    System,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HitRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl HitRect {
    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x as f64
            && x < self.x.saturating_add(self.width) as f64
            && y >= self.y as f64
            && y < self.y.saturating_add(self.height) as f64
    }
}

fn workspace_buttons(workspaces: &[WorkspaceInput], x: u32, height: u32) -> Vec<WorkspaceButton> {
    let mut cursor = x;
    let mut buttons = Vec::with_capacity(workspaces.len());
    for workspace in workspaces {
        let label = if workspace.focused {
            format!("[{}]", workspace.name)
        } else if workspace.occupied {
            format!("*{}", workspace.name)
        } else {
            workspace.name.clone()
        };
        let width = text_width(&label);
        buttons.push(WorkspaceButton {
            name: workspace.name.clone(),
            label,
            rect: HitRect {
                x: cursor.saturating_sub(WORKSPACE_HIT_PADDING),
                y: 0,
                width: width.saturating_add(WORKSPACE_HIT_PADDING * 2),
                height,
            },
        });
        cursor = cursor.saturating_add(width + text_width(" "));
    }
    buttons
}

fn resize_workspace_hit_regions(workspaces: &mut [WorkspaceButton], height: u32) {
    for workspace in workspaces {
        workspace.rect.height = height;
    }
}

fn hit_workspace(workspaces: &[WorkspaceButton], x: f64, y: f64, height: u32) -> Option<&str> {
    if y < 0.0 || y >= height as f64 {
        return None;
    }
    workspaces
        .iter()
        .find(|workspace| workspace.rect.contains(x, y))
        .map(|workspace| workspace.name.as_str())
}

#[cfg(test)]
fn hit_bar_region(
    workspaces: &[WorkspaceButton],
    width: u32,
    height: u32,
    x: f64,
    y: f64,
) -> BarRegion {
    hit_bar_region_with_settings(workspaces, width, height, &BarSettings::default(), x, y)
}

fn hit_bar_region_with_settings(
    workspaces: &[WorkspaceButton],
    width: u32,
    height: u32,
    settings: &BarSettings,
    x: f64,
    y: f64,
) -> BarRegion {
    if y < 0.0 || y >= height as f64 {
        return BarRegion::None;
    }
    if settings.widget_enabled(BarWidget::Workspaces)
        && let Some(workspace) = hit_workspace(workspaces, x, y, height)
    {
        return BarRegion::Workspace(workspace.to_string());
    }
    let layout = BarLayout::new_with_settings(width, height, settings);
    if settings.widget_enabled(BarWidget::FocusedWindow) && layout.window.contains(x, y) {
        BarRegion::Window
    } else if settings.widget_enabled(BarWidget::QuickToggle) && layout.status.contains(x, y) {
        BarRegion::Status
    } else if system_region_enabled(settings) && layout.system.contains(x, y) {
        BarRegion::System
    } else {
        BarRegion::None
    }
}

#[cfg(test)]
fn workspace_strip_region(workspaces: &[WorkspaceButton], width: u32, height: u32) -> HitRect {
    let layout = BarLayout::new(width, height);
    if workspaces.is_empty() {
        return HitRect {
            x: layout.workspace.x,
            y: 0,
            width: 0,
            height,
        };
    }
    let strip_end = workspaces
        .iter()
        .map(|workspace| workspace.rect.x.saturating_add(workspace.rect.width))
        .max()
        .unwrap_or(layout.workspace.x);
    HitRect {
        x: layout.workspace.x,
        y: 0,
        width: strip_end.min(width).saturating_sub(layout.workspace.x),
        height,
    }
}

fn focus_workspace(name: &str) -> Result<(), String> {
    dispatch_shell_action(hyprbole_core::daemon::ShellAction::Compositor {
        action: hyprbole_core::compositor::CompositorAction::FocusWorkspace {
            name: name.to_string(),
        },
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceScrollDirection {
    Next,
    Previous,
}

fn scroll_direction(
    vertical: &smithay_client_toolkit::seat::pointer::AxisScroll,
) -> Option<WorkspaceScrollDirection> {
    let amount = if vertical.value120 != 0 {
        vertical.value120
    } else if vertical.discrete != 0 {
        vertical.discrete
    } else if vertical.absolute > 0.0 {
        1
    } else if vertical.absolute < 0.0 {
        -1
    } else {
        0
    };

    match amount.cmp(&0) {
        std::cmp::Ordering::Greater => Some(WorkspaceScrollDirection::Next),
        std::cmp::Ordering::Less => Some(WorkspaceScrollDirection::Previous),
        std::cmp::Ordering::Equal => None,
    }
}

fn focus_relative_workspace(direction: WorkspaceScrollDirection) -> Result<(), String> {
    let action = match direction {
        WorkspaceScrollDirection::Next => {
            hyprbole_core::compositor::CompositorAction::FocusNextWorkspace
        }
        WorkspaceScrollDirection::Previous => {
            hyprbole_core::compositor::CompositorAction::FocusPreviousWorkspace
        }
    };
    dispatch_shell_action(hyprbole_core::daemon::ShellAction::Compositor { action })
}

fn dispatch_shell_action(action: hyprbole_core::daemon::ShellAction) -> Result<(), String> {
    let client = hyprbole_core::daemon::DaemonClient::from_env()
        .map_err(|err| format!("connect daemon: {err}"))?;
    match client.send_shell(&hyprbole_core::daemon::ShellRequest::ActionCall { action }) {
        Ok(hyprbole_core::daemon::ShellResponse::Ok { .. }) => Ok(()),
        Ok(hyprbole_core::daemon::ShellResponse::Error { message }) => Err(message),
        Ok(response) => Err(format!("unexpected daemon response: {response:?}")),
        Err(err) => Err(err.to_string()),
    }
}

fn toggle_quick_settings() -> Result<(), String> {
    if let Some((pid, identity)) = quick_settings_instance() {
        return dismiss_quick_settings(pid, &identity);
    }
    let current_exe = std::env::current_exe().map_err(|err| err.to_string())?;
    Command::new(current_exe)
        .arg("--quick")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn quick_settings_instance() -> Option<(u32, String)> {
    let Ok(path) = hyprbole_core::runtime::ensure_runtime_dir().map(|dir| dir.join("quick.pid"))
    else {
        return None;
    };
    let identity = std::fs::read_to_string(path)
        .ok()
        .and_then(|value| parse_quick_identity(&value))?;
    (quick_process_alive(identity.0)
        && quick_process_identity(identity.0).as_deref() == Some(&identity.1))
    .then_some(identity)
}

fn quick_process_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
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

fn dismiss_quick_settings(_pid: u32, identity: &str) -> Result<(), String> {
    if quick_settings_instance()
        .as_ref()
        .map(|(_pid, id)| id.as_str())
        != Some(identity)
    {
        return Ok(());
    }
    let path = hyprbole_core::runtime::ensure_runtime_dir()
        .map_err(|err| err.to_string())?
        .join("quick.dismiss");
    std::fs::write(path, identity).map_err(|err| err.to_string())
}

struct BarLayout {
    workspace: HitRect,
    window: HitRect,
    status: HitRect,
    system: HitRect,
}

impl BarLayout {
    #[cfg(test)]
    fn new(width: u32, height: u32) -> Self {
        Self::new_with_settings(width, height, &BarSettings::default())
    }

    fn new_with_settings(width: u32, height: u32, settings: &BarSettings) -> Self {
        let padding = settings.padding.min(width / 4);
        let usable_width = width.saturating_sub(padding * 2);
        let left = padding;
        let center = left.saturating_add(usable_width / 4);
        let status = left.saturating_add(usable_width / 2);
        let right_width = if system_region_enabled(settings) {
            220.min(usable_width / 2)
        } else {
            0
        };
        let right = width.saturating_sub(padding).saturating_sub(right_width);
        Self {
            workspace: HitRect {
                x: left,
                y: 0,
                width: if settings.widget_enabled(BarWidget::Workspaces) {
                    center.saturating_sub(left).saturating_sub(12)
                } else {
                    0
                },
                height,
            },
            window: HitRect {
                x: center,
                y: 0,
                width: if settings.widget_enabled(BarWidget::FocusedWindow) {
                    status.saturating_sub(center).saturating_sub(12)
                } else {
                    0
                },
                height,
            },
            status: HitRect {
                x: status,
                y: 0,
                width: if settings.widget_enabled(BarWidget::QuickToggle) {
                    right.saturating_sub(status).saturating_sub(12)
                } else {
                    0
                },
                height,
            },
            system: HitRect {
                x: right,
                y: 0,
                width: right_width,
                height,
            },
        }
    }

    fn workspace_text_x(&self) -> u32 {
        self.workspace.x.saturating_add(12)
    }
}

fn system_region_enabled(settings: &BarSettings) -> bool {
    settings.widget_enabled(BarWidget::Audio)
        || settings.widget_enabled(BarWidget::Brightness)
        || settings.widget_enabled(BarWidget::Layout)
        || settings.widget_enabled(BarWidget::Notifications)
        || settings.widget_enabled(BarWidget::Clock)
}

fn system_text(
    settings: &BarSettings,
    audio_summary: &str,
    brightness_summary: &str,
    current_layout: &str,
    dnd_enabled: bool,
    notification_unread: usize,
) -> String {
    let mut parts = Vec::new();
    for widget in &settings.widgets {
        match widget {
            BarWidget::Audio => parts.push(short_status(audio_summary, "AUDIO --")),
            BarWidget::Brightness => parts.push(short_status(brightness_summary, "BRI --")),
            BarWidget::Layout => parts.push(if current_layout.is_empty() {
                "LAY --".to_string()
            } else {
                format!("LAY {}", truncate_text(current_layout, 8))
            }),
            BarWidget::Notifications => parts.push(format!(
                "NOT {}:{}",
                if dnd_enabled { "DND" } else { "ON" },
                notification_unread
            )),
            BarWidget::Clock => parts.push(clock_text()),
            BarWidget::Workspaces
            | BarWidget::FocusedWindow
            | BarWidget::DaemonStatus
            | BarWidget::QuickToggle => {}
        }
    }
    parts.join("  ")
}

fn clock_text() -> String {
    let Ok(output) = Command::new("date").arg("+%H:%M").output() else {
        return "TIME --".to_string();
    };
    if output.status.success() {
        format!("TIME {}", String::from_utf8_lossy(&output.stdout).trim())
    } else {
        "TIME --".to_string()
    }
}

fn short_status(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() || value.contains("unavailable") {
        fallback.to_string()
    } else {
        truncate_text(value, 18)
    }
}

pub(super) fn text_width(value: &str) -> u32 {
    value.chars().count() as u32 * 6
}

pub(super) fn truncate_text(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for ch in value.chars().take(max_chars) {
        output.push(ch);
    }
    if value.chars().count() > max_chars && max_chars > 1 {
        output.pop();
        output.push('.');
    }
    output
}

fn draw_vertical_rule(canvas: &mut [u8], width: u32, x: u32, color: u32) {
    for y in 8..28 {
        let index = ((y * width + x) * 4) as usize;
        if index + 4 <= canvas.len() {
            canvas[index..index + 4].copy_from_slice(&color.to_le_bytes());
        }
    }
}

pub(super) fn draw_text(canvas: &mut [u8], width: u32, x: u32, y: u32, text: &str, color: u32) {
    let mut cursor = x;
    for ch in text.to_ascii_uppercase().chars() {
        draw_char(canvas, width, cursor, y, ch, color);
        cursor += 6;
        if cursor + 6 >= width {
            break;
        }
    }
}

fn draw_char(canvas: &mut [u8], width: u32, x: u32, y: u32, ch: char, color: u32) {
    let glyph = glyph(ch);
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) == 0 {
                continue;
            }
            let px = x + col;
            let py = y + row as u32;
            let index = ((py * width + px) * 4) as usize;
            if index + 4 <= canvas.len() {
                canvas[index..index + 4].copy_from_slice(&color.to_le_bytes());
            }
        }
    }
}

fn glyph(ch: char) -> [u8; 7] {
    match ch {
        '0' => [
            0b11111, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b11111,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b11110, 0b00001, 0b00001, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b10010, 0b10010, 0b10010, 0b11111, 0b00010, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01111, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b11110,
        ],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        ':' => [0, 0b00100, 0b00100, 0, 0b00100, 0b00100, 0],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '[' => [
            0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110,
        ],
        ']' => [
            0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110,
        ],
        '%' => [0b10001, 0b00010, 0b00100, 0b01000, 0b10001, 0, 0],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        ' ' => [0; 7],
        _ => [0; 7],
    }
}

impl CompositorHandler for Bar {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        if let Err(err) = self.draw(qh) {
            eprintln!("layer-shell draw failed: {err}");
            self.exit = true;
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for Bar {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.output_state.info(&output) {
            eprintln!(
                "bar output discovered name={:?} description={:?}",
                info.name, info.description
            );
        }
        self.last_output_resolution.clear();
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.output_state.info(&output) {
            eprintln!(
                "bar output updated name={:?} description={:?}",
                info.name, info.description
            );
        }
        self.last_output_resolution.clear();
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.output_state.info(&output) {
            eprintln!(
                "bar output destroyed name={:?} description={:?}",
                info.name, info.description
            );
        }
        self.selected_output_name = None;
        self.force_layer_recreate = true;
        self.last_output_resolution.clear();
    }
}

impl LayerShellHandler for Bar {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.width = NonZeroU32::new(configure.new_size.0).map_or(WIDTH, NonZeroU32::get);
        self.height = NonZeroU32::new(configure.new_size.1).map_or(HEIGHT, NonZeroU32::get);
        self.first_configure = false;
        if let Err(err) = self.draw(qh) {
            eprintln!("layer-shell draw failed: {err}");
            self.exit = true;
        }
    }
}

impl SeatHandler for Bar {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            match self.seat_state.get_pointer(qh, &seat) {
                Ok(pointer) => self.pointer = Some(pointer),
                Err(err) => eprintln!("bar pointer setup failed: {err}"),
            }
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_some() {
            self.pointer
                .take()
                .expect("pointer checked above")
                .release();
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {
    }
}

impl PointerHandler for Bar {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if &event.surface != self.layer.wl_surface() {
                continue;
            }
            match &event.kind {
                PointerEventKind::Press { button: 0x110, .. } => {
                    self.handle_click(event.position.0, event.position.1);
                }
                PointerEventKind::Axis { vertical, .. } => {
                    self.handle_scroll(event.position.0, event.position.1, vertical);
                }
                _ => {}
            }
        }
    }
}

impl ShmHandler for Bar {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(Bar);
delegate_output!(Bar);
delegate_shm!(Bar);
delegate_seat!(Bar);
delegate_pointer!(Bar);
delegate_layer!(Bar);
delegate_registry!(Bar);

impl ProvidesRegistryState for Bar {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_buttons_track_label_regions() {
        let buttons = workspace_buttons(
            &[
                WorkspaceInput {
                    name: "1".to_string(),
                    focused: true,
                    occupied: true,
                },
                WorkspaceInput {
                    name: "dev".to_string(),
                    focused: false,
                    occupied: true,
                },
            ],
            30,
            HEIGHT,
        );

        assert_eq!(buttons[0].label, "[1]");
        assert_eq!(buttons[0].rect.x, 27);
        assert_eq!(buttons[0].rect.width, text_width("[1]") + 6);
        assert_eq!(buttons[1].label, "*dev");
        assert_eq!(buttons[1].rect.x, 30 + text_width("[1] ") - 3);
        assert_eq!(buttons[1].rect.width, text_width("*dev") + 6);
    }

    #[test]
    fn hit_workspace_returns_clicked_workspace_name() {
        let buttons = workspace_buttons(
            &[
                WorkspaceInput {
                    name: "1".to_string(),
                    focused: false,
                    occupied: false,
                },
                WorkspaceInput {
                    name: "2".to_string(),
                    focused: false,
                    occupied: false,
                },
            ],
            20,
            HEIGHT,
        );

        assert_eq!(hit_workspace(&buttons, 21.0, 8.0, HEIGHT), Some("1"));
        assert_eq!(
            hit_workspace(&buttons, (20 + text_width("1 ")) as f64, 8.0, HEIGHT),
            Some("2")
        );
        assert_eq!(hit_workspace(&buttons, 16.0, 8.0, HEIGHT), None);
        assert_eq!(hit_workspace(&buttons, 21.0, HEIGHT as f64, HEIGHT), None);
    }

    #[test]
    fn workspace_buttons_use_configured_height() {
        let buttons = workspace_buttons(
            &[WorkspaceInput {
                name: "1".to_string(),
                focused: false,
                occupied: false,
            }],
            20,
            52,
        );

        assert_eq!(buttons[0].rect.height, 52);
        assert_eq!(hit_workspace(&buttons, 21.0, 51.0, 52), Some("1"));
        assert_eq!(hit_workspace(&buttons, 21.0, 52.0, 52), None);
    }

    #[test]
    fn workspace_hit_regions_resize_after_bar_height_change() {
        let mut buttons = workspace_buttons(
            &[WorkspaceInput {
                name: "1".to_string(),
                focused: false,
                occupied: false,
            }],
            20,
            36,
        );
        resize_workspace_hit_regions(&mut buttons, 56);

        assert_eq!(buttons[0].rect.height, 56);
        assert_eq!(hit_workspace(&buttons, 21.0, 55.0, 56), Some("1"));
    }

    #[test]
    fn bar_layout_exposes_widget_regions() {
        let layout = BarLayout::new(720, 44);

        assert!(layout.workspace.contains(24.0, 20.0));
        assert!(layout.window.contains(190.0, 20.0));
        assert!(layout.status.contains(370.0, 20.0));
        assert!(layout.system.contains(520.0, 20.0));
        assert!(!layout.workspace.contains(190.0, 20.0));
        assert_eq!(layout.workspace_text_x(), 24);
    }

    #[test]
    fn bar_layout_respects_configured_padding() {
        let settings = BarSettings {
            padding: 24,
            ..BarSettings::default()
        };
        let layout = BarLayout::new_with_settings(720, 44, &settings);

        assert_eq!(layout.workspace.x, 24);
        assert_eq!(layout.workspace_text_x(), 36);
        assert!(layout.system.x < 720 - 24);
    }

    #[test]
    fn hit_bar_region_prioritizes_workspace_buttons() {
        let buttons = workspace_buttons(
            &[WorkspaceInput {
                name: "1".to_string(),
                focused: true,
                occupied: true,
            }],
            24,
            HEIGHT,
        );

        assert_eq!(
            hit_bar_region(&buttons, 720, HEIGHT, 25.0, 8.0),
            BarRegion::Workspace("1".to_string())
        );
        assert_eq!(
            hit_bar_region(&buttons, 720, HEIGHT, 190.0, 8.0),
            BarRegion::Window
        );
        assert_eq!(
            hit_bar_region(&buttons, 720, HEIGHT, 370.0, 8.0),
            BarRegion::Status
        );
        assert_eq!(
            hit_bar_region(&buttons, 720, HEIGHT, 520.0, 8.0),
            BarRegion::System
        );
        assert_eq!(
            hit_bar_region(&buttons, 720, HEIGHT, 720.0, 8.0),
            BarRegion::None
        );
    }

    #[test]
    fn hidden_bar_widgets_remove_hit_regions() {
        let buttons = workspace_buttons(
            &[WorkspaceInput {
                name: "1".to_string(),
                focused: true,
                occupied: true,
            }],
            24,
            HEIGHT,
        );
        let settings = BarSettings {
            widgets: vec![BarWidget::Audio],
            ..BarSettings::default()
        };

        assert_eq!(
            hit_bar_region_with_settings(&buttons, 720, HEIGHT, &settings, 25.0, 8.0),
            BarRegion::None
        );
        assert_eq!(
            hit_bar_region_with_settings(&buttons, 720, HEIGHT, &settings, 520.0, 8.0),
            BarRegion::System
        );
        assert_eq!(
            hit_bar_region_with_settings(&buttons, 720, HEIGHT, &settings, 370.0, 8.0),
            BarRegion::None
        );
    }

    #[test]
    fn workspace_strip_region_extends_to_rendered_buttons() {
        let buttons = workspace_buttons(
            &[
                WorkspaceInput {
                    name: "1".to_string(),
                    focused: false,
                    occupied: false,
                },
                WorkspaceInput {
                    name: "long-workspace".to_string(),
                    focused: false,
                    occupied: true,
                },
            ],
            24,
            HEIGHT,
        );
        let region = workspace_strip_region(&buttons, 720, HEIGHT);
        let last_end = buttons
            .last()
            .map(|button| button.rect.x + button.rect.width)
            .unwrap();

        assert!(region.contains((last_end - 1) as f64, 8.0));
        assert!(!region.contains(last_end as f64, 8.0));
    }

    #[test]
    fn workspace_strip_region_is_empty_without_buttons() {
        let region = workspace_strip_region(&[], 720, HEIGHT);

        assert_eq!(region.width, 0);
        assert!(!region.contains(10.0, 8.0));
    }

    #[test]
    fn system_text_follows_configured_widget_order() {
        let settings = BarSettings {
            widgets: vec![BarWidget::Clock, BarWidget::Layout, BarWidget::Audio],
            ..BarSettings::default()
        };
        let text = system_text(
            &settings,
            "volume: 50%",
            "brightness: 80%",
            "dwindle",
            false,
            2,
        );

        assert!(text.starts_with("TIME "));
        assert!(text.contains("  LAY dwindle  volume: 50%"));
    }

    #[test]
    fn bar_colors_fall_back_for_invalid_theme_values() {
        let mut tokens =
            hyprbole_core::theme::tokens_for_mode(hyprbole_core::theme::ThemeMode::Dark).colors;
        tokens.surface = "not-a-color".to_string();
        let colors = BarColors::from_tokens(&tokens);

        assert_eq!(colors.surface, BarColors::default().surface);
        assert_ne!(colors.text, BarColors::default().text);
    }

    #[test]
    fn resolves_focused_monitor_to_matching_wayland_output() {
        let daemon = vec![
            DaemonMonitorTarget {
                name: "HDMI-A-1".to_string(),
                focused: false,
            },
            DaemonMonitorTarget {
                name: "eDP-1".to_string(),
                focused: true,
            },
        ];
        let outputs = vec![WaylandOutputTarget {
            name: Some("eDP-1".to_string()),
            description: None,
        }];

        let resolution = resolve_bar_output(&BarMonitor::Focused, &daemon, &outputs);
        assert_eq!(resolution.daemon_monitor.as_deref(), Some("eDP-1"));
        assert_eq!(resolution.output_name.as_deref(), Some("eDP-1"));
    }

    #[test]
    fn resolves_primary_to_first_daemon_monitor() {
        let daemon = vec![DaemonMonitorTarget {
            name: "DP-1".to_string(),
            focused: false,
        }];
        let outputs = vec![WaylandOutputTarget {
            name: Some("DP-1".to_string()),
            description: None,
        }];

        let resolution = resolve_bar_output(&BarMonitor::Primary, &daemon, &outputs);
        assert_eq!(resolution.daemon_monitor.as_deref(), Some("DP-1"));
        assert_eq!(resolution.output_name.as_deref(), Some("DP-1"));
    }

    #[test]
    fn resolves_named_monitor_and_falls_back_when_missing() {
        let daemon = vec![DaemonMonitorTarget {
            name: "eDP-1".to_string(),
            focused: true,
        }];
        let outputs = vec![WaylandOutputTarget {
            name: Some("eDP-1".to_string()),
            description: None,
        }];

        let matched =
            resolve_bar_output(&BarMonitor::Named("eDP-1".to_string()), &daemon, &outputs);
        assert_eq!(matched.output_name.as_deref(), Some("eDP-1"));

        let missing = resolve_bar_output(&BarMonitor::Named("DP-9".to_string()), &daemon, &outputs);
        assert_eq!(missing.daemon_monitor.as_deref(), Some("DP-9"));
        assert_eq!(missing.output_name.as_deref(), Some("eDP-1"));
        assert!(missing.reason.contains("fallback"));
    }

    #[test]
    fn output_resolution_uses_compositor_default_until_outputs_exist() {
        let resolution = resolve_bar_output(&BarMonitor::Focused, &[], &[]);
        assert_eq!(resolution.output_name, None);
        assert!(resolution.reason.contains("compositor default"));
    }

    #[test]
    fn output_recreation_tracks_output_target_changes() {
        assert!(output_recreation_needed(
            Some("eDP-1"),
            Some("DP-1"),
            false,
            false
        ));
        assert!(output_recreation_needed(
            Some("eDP-1"),
            Some("eDP-1"),
            true,
            false
        ));
        assert!(output_recreation_needed(None, None, false, true));
        assert!(!output_recreation_needed(
            Some("eDP-1"),
            Some("eDP-1"),
            false,
            false
        ));
    }

    #[test]
    fn scroll_direction_maps_vertical_motion() {
        let mut scroll = smithay_client_toolkit::seat::pointer::AxisScroll {
            value120: 120,
            ..Default::default()
        };
        assert_eq!(
            scroll_direction(&scroll),
            Some(WorkspaceScrollDirection::Next)
        );

        scroll.value120 = -120;
        assert_eq!(
            scroll_direction(&scroll),
            Some(WorkspaceScrollDirection::Previous)
        );

        scroll.value120 = 0;
        assert_eq!(scroll_direction(&scroll), None);
    }

    #[test]
    fn action_throttle_drops_rapid_scroll_duplicates() {
        let mut throttle = ActionThrottle::default();
        let now = Instant::now();
        let scroll = BarAction::FocusRelativeWorkspace(WorkspaceScrollDirection::Next);

        assert!(throttle.should_dispatch(&scroll, now));
        assert!(!throttle.should_dispatch(&scroll, now + Duration::from_millis(40)));
        assert!(throttle.should_dispatch(&scroll, now + Duration::from_millis(220)));

        let brightness = BarAction::Brightness(BrightnessDirection::Increase);
        assert!(throttle.should_dispatch(&brightness, now));
        assert!(!throttle.should_dispatch(&brightness, now + Duration::from_millis(40)));
        assert!(throttle.should_dispatch(&brightness, now + Duration::from_millis(200)));
    }

    #[test]
    fn action_throttle_allows_direct_workspace_clicks() {
        let mut throttle = ActionThrottle::default();
        let now = Instant::now();
        let action = BarAction::FocusWorkspace("1".to_string());

        assert!(throttle.should_dispatch(&action, now));
        assert!(throttle.should_dispatch(&action, now));
    }

    #[test]
    fn quick_cmdline_requires_ui_binary_and_quick_flag() {
        assert!(cmdline_is_quick_settings(b"/tmp/hyprbole-ui\0--quick\0"));
        assert!(!cmdline_is_quick_settings(b"/tmp/hyprbole-ui\0--bar\0"));
        assert!(!cmdline_is_quick_settings(
            b"/tmp/other\0hyprbole-ui\0--quick\0"
        ));
        assert!(!cmdline_is_quick_settings(b"/tmp/other\0--quick\0"));
    }
}
