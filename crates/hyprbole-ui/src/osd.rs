use std::{
    num::NonZeroU32,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat,
    delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
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
    protocol::{wl_output, wl_seat, wl_shm, wl_surface},
};

use crate::bar::{draw_text, truncate_text};
use hyprbole_core::settings::{BarEdge, OsdSettings};

const WIDTH: u32 = 420;
const HEIGHT: u32 = 96;
const REFRESH_INTERVAL: Duration = Duration::from_secs(2);
const MIN_REFRESH_INTERVAL: Duration = Duration::from_millis(300);

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
        layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("hyprbole-osd"), None);
    layer.set_anchor(Anchor::TOP);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.set_margin(64, 0, 0, 0);
    layer.set_exclusive_zone(0);
    layer.set_size(WIDTH, HEIGHT);
    layer.commit();

    let pool = SlotPool::new((WIDTH * HEIGHT * 4) as usize, &shm)
        .map_err(|err| format!("create shm pool: {err}"))?;
    let (refresh_tx, refresh_rx) = spawn_refresh_worker();
    let mut app = OsdSurface {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        pool,
        layer,
        width: WIDTH,
        height: HEIGHT,
        connected: false,
        message: "Waiting for daemon".to_string(),
        detail: String::new(),
        audio: String::new(),
        brightness: String::new(),
        colors: OsdColors::default(),
        settings: OsdSettings::default(),
        event_rx: spawn_event_listener(),
        refresh_tx,
        refresh_rx,
        refresh_pending: false,
        refresh_again: false,
        next_refresh_id: 0,
        active_refresh_id: None,
        last_refresh_request: None,
        last_update: None,
        visible_until: None,
        first_configure: true,
        exit: false,
    };
    app.request_refresh(true);

    while !app.exit {
        dispatch_wayland(&mut event_queue, &mut app)?;
        let event_changed = app.consume_events();
        let refresh_changed = app.consume_refreshes();
        let changed = event_changed || refresh_changed;
        if app.refresh_due() {
            app.request_refresh(false);
        }
        if changed || app.hide_due() {
            let _ = app.draw(&qh);
        }
    }
    Ok(())
}

fn dispatch_wayland(
    event_queue: &mut wayland_client::EventQueue<OsdSurface>,
    app: &mut OsdSurface,
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

fn spawn_event_listener() -> Receiver<OsdSignal> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut after = None;
        loop {
            let Ok(client) = hyprbole_core::daemon::DaemonClient::from_env() else {
                let _ = tx.send(OsdSignal::refresh());
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            if after.is_none() {
                after = initial_event_cursor(&client);
                if after.is_none() {
                    let _ = tx.send(OsdSignal::refresh());
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            }
            let Ok(events) = client.follow_events(after) else {
                let _ = tx.send(OsdSignal::refresh());
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            for event in events {
                let Ok(event) = event else {
                    break;
                };
                after = Some(event.id);
                if event.event.refreshes_snapshot() || event.event.refreshes_status() {
                    let _ = tx.send(OsdSignal {
                        show: event_shows_osd(&event.event),
                    });
                }
            }
            let _ = tx.send(OsdSignal::refresh());
            thread::sleep(Duration::from_millis(250));
        }
    });
    rx
}

fn initial_event_cursor(client: &hyprbole_core::daemon::DaemonClient) -> Option<u64> {
    match client.send_shell(&hyprbole_core::daemon::ShellRequest::StatusGet) {
        Ok(hyprbole_core::daemon::ShellResponse::Status { daemon }) => {
            Some(cursor_before_next_event(daemon.next_event_id))
        }
        _ => None,
    }
}

fn cursor_before_next_event(next_event_id: u64) -> u64 {
    next_event_id.saturating_sub(1)
}

fn spawn_refresh_worker() -> (Sender<u64>, Receiver<OsdRefresh>) {
    let (request_tx, request_rx) = mpsc::channel();
    let (update_tx, update_rx) = mpsc::channel();
    thread::spawn(move || {
        while let Ok(mut request_id) = request_rx.recv() {
            while let Ok(newer) = request_rx.try_recv() {
                request_id = newer;
            }
            let _ = update_tx.send(load_osd_refresh(request_id));
        }
    });
    (request_tx, update_rx)
}

#[derive(Clone, Copy)]
struct OsdSignal {
    show: bool,
}

impl OsdSignal {
    fn refresh() -> Self {
        Self { show: false }
    }
}

struct OsdSurface {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    pool: SlotPool,
    layer: LayerSurface,
    width: u32,
    height: u32,
    connected: bool,
    message: String,
    detail: String,
    audio: String,
    brightness: String,
    colors: OsdColors,
    settings: OsdSettings,
    event_rx: Receiver<OsdSignal>,
    refresh_tx: Sender<u64>,
    refresh_rx: Receiver<OsdRefresh>,
    refresh_pending: bool,
    refresh_again: bool,
    next_refresh_id: u64,
    active_refresh_id: Option<u64>,
    last_refresh_request: Option<Instant>,
    last_update: Option<Instant>,
    visible_until: Option<Instant>,
    first_configure: bool,
    exit: bool,
}

impl OsdSurface {
    fn consume_events(&mut self) -> bool {
        let mut changed = false;
        while let Ok(signal) = self.event_rx.try_recv() {
            changed = true;
            if signal.show {
                self.show_for_action();
            }
        }
        if changed {
            self.request_refresh(true);
        }
        changed
    }

    fn consume_refreshes(&mut self) -> bool {
        let mut latest = None;
        while let Ok(update) = self.refresh_rx.try_recv() {
            latest = Some(update);
        }
        let Some(update) = latest else {
            return false;
        };
        self.refresh_pending = false;
        if Some(update.request_id) != self.active_refresh_id || self.refresh_again {
            self.refresh_again = false;
            self.active_refresh_id = None;
            self.request_refresh(true);
            return false;
        }
        self.active_refresh_id = None;
        self.last_update = Some(Instant::now());
        self.connected = update.connected;
        self.message = update.message;
        self.detail = update.detail;
        if let Some(audio) = update.audio {
            self.audio = audio;
        }
        if let Some(brightness) = update.brightness {
            self.brightness = brightness;
        }
        if let Some(colors) = update.colors {
            self.colors = colors;
        }
        if let Some(settings) = update.settings {
            self.settings = settings;
            self.apply_settings();
        }
        true
    }

    fn request_refresh(&mut self, force: bool) {
        if self.refresh_pending {
            if force {
                self.refresh_again = true;
            }
            return;
        }
        if !force
            && self
                .last_refresh_request
                .is_some_and(|last| last.elapsed() < MIN_REFRESH_INTERVAL)
        {
            return;
        }
        self.last_refresh_request = Some(Instant::now());
        self.next_refresh_id = self.next_refresh_id.saturating_add(1);
        let request_id = self.next_refresh_id;
        if self.refresh_tx.send(request_id).is_ok() {
            self.refresh_pending = true;
            self.active_refresh_id = Some(request_id);
        }
    }

    fn refresh_due(&self) -> bool {
        !self.refresh_pending
            && self
                .last_update
                .is_none_or(|last| last.elapsed() > REFRESH_INTERVAL)
    }

    fn show_for_action(&mut self) {
        if self.settings.enabled {
            self.visible_until = Some(Instant::now() + self.visible_duration());
        }
    }

    fn visible(&self) -> bool {
        self.visible_until
            .is_some_and(|until| until > Instant::now())
    }

    fn hide_due(&mut self) -> bool {
        let Some(until) = self.visible_until else {
            return false;
        };
        if until > Instant::now() {
            return false;
        }
        self.visible_until = None;
        true
    }

    fn draw(&mut self, _qh: &QueueHandle<Self>) -> Result<(), String> {
        let needed = (self.width * self.height * 4) as usize;
        if self.pool.len() < needed {
            self.pool
                .resize(needed)
                .map_err(|err| format!("resize shm pool: {err}"))?;
        }
        let visible = self.settings.enabled && self.visible();
        let status_color = self.status_color();
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
        let background = if visible {
            self.colors.surface
        } else {
            0x0000_0000
        };
        for pixel in canvas.chunks_exact_mut(4) {
            pixel.copy_from_slice(&background.to_le_bytes());
        }
        if visible {
            fill_rect(
                canvas,
                self.width,
                Rect {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: self.height,
                },
                status_color,
            );
            draw_text(
                canvas,
                self.width,
                18,
                14,
                &truncate_text(&self.message, 46),
                self.colors.text,
            );
            draw_text(
                canvas,
                self.width,
                18,
                34,
                &truncate_text(&self.detail, 56),
                self.colors.muted,
            );
            draw_text(
                canvas,
                self.width,
                18,
                62,
                &truncate_text(&format!("AUD {}  BRI {}", self.audio, self.brightness), 62),
                self.colors.muted,
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

    fn status_color(&self) -> u32 {
        if self.connected {
            self.colors.success
        } else {
            self.colors.error
        }
    }

    fn visible_duration(&self) -> Duration {
        Duration::from_millis(self.settings.timeout_ms as u64)
    }

    fn apply_settings(&mut self) {
        let anchor = match self.settings.edge {
            BarEdge::Top => Anchor::TOP,
            BarEdge::Bottom => Anchor::BOTTOM,
        };
        self.layer.set_anchor(anchor);
        match self.settings.edge {
            BarEdge::Top => self.layer.set_margin(self.settings.margin as i32, 0, 0, 0),
            BarEdge::Bottom => self.layer.set_margin(0, 0, self.settings.margin as i32, 0),
        }
        self.layer
            .set_size(self.settings.width, self.settings.height);
        self.layer.commit();
    }
}

struct OsdRefresh {
    request_id: u64,
    connected: bool,
    message: String,
    detail: String,
    audio: Option<String>,
    brightness: Option<String>,
    colors: Option<OsdColors>,
    settings: Option<OsdSettings>,
}

fn load_osd_refresh(request_id: u64) -> OsdRefresh {
    let Ok(client) = hyprbole_core::daemon::DaemonClient::from_env() else {
        return OsdRefresh::offline(request_id, "Daemon offline");
    };

    let mut refresh = OsdRefresh {
        request_id,
        connected: false,
        message: "Daemon status unavailable".to_string(),
        detail: String::new(),
        audio: None,
        brightness: None,
        colors: None,
        settings: None,
    };

    if let Ok(hyprbole_core::daemon::ShellResponse::State { snapshot }) =
        client.send_shell(&hyprbole_core::daemon::ShellRequest::StateGet)
    {
        refresh.connected = true;
        refresh.audio = Some(snapshot.audio_summary);
        refresh.brightness = Some(snapshot.brightness_summary);
        refresh.colors = Some(OsdColors::from_tokens(&snapshot.theme.tokens.colors));
        refresh.settings = Some(snapshot.settings.ui.osd);
    }

    if let Ok(hyprbole_core::daemon::ShellResponse::Status { daemon }) =
        client.send_shell(&hyprbole_core::daemon::ShellRequest::StatusGet)
    {
        refresh.connected = true;
        let (message, detail) = osd_message(daemon.last_action.as_ref());
        refresh.message = message;
        refresh.detail = detail;
    }

    refresh
}

impl OsdRefresh {
    fn offline(request_id: u64, message: impl Into<String>) -> Self {
        Self {
            request_id,
            connected: false,
            message: message.into(),
            detail: String::new(),
            audio: None,
            brightness: None,
            colors: None,
            settings: None,
        }
    }
}

#[derive(Clone, Copy)]
struct OsdColors {
    surface: u32,
    text: u32,
    muted: u32,
    success: u32,
    error: u32,
}

impl Default for OsdColors {
    fn default() -> Self {
        Self {
            surface: 0xff1a1d24,
            text: 0xfff1f3f6,
            muted: 0xffa7afbd,
            success: 0xff7ee787,
            error: 0xffff7b72,
        }
    }
}

impl OsdColors {
    fn from_tokens(tokens: &hyprbole_core::theme::ColorTokens) -> Self {
        let fallback = Self::default();
        Self {
            surface: argb_from_hex(&tokens.surface).unwrap_or(fallback.surface),
            text: argb_from_hex(&tokens.text).unwrap_or(fallback.text),
            muted: argb_from_hex(&tokens.text_muted).unwrap_or(fallback.muted),
            success: argb_from_hex(&tokens.success).unwrap_or(fallback.success),
            error: argb_from_hex(&tokens.error).unwrap_or(fallback.error),
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

fn osd_message(action: Option<&hyprbole_core::daemon::ActionStatus>) -> (String, String) {
    let Some(action) = action else {
        return ("Hyprbole ready".to_string(), "No recent action".to_string());
    };
    let message = format!(
        "{} {}.{}",
        if action.ok { "OK" } else { "ERR" },
        action.domain,
        action.name
    );
    (message, action.message.clone())
}

fn event_shows_osd(event: &hyprbole_core::daemon::ShellEvent) -> bool {
    matches!(event, hyprbole_core::daemon::ShellEvent::Action { .. })
}

#[derive(Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn fill_rect(canvas: &mut [u8], width: u32, rect: Rect, color: u32) {
    for y in rect.y..rect.y.saturating_add(rect.height) {
        for x in rect.x..rect.x.saturating_add(rect.width) {
            let index = ((y * width + x) * 4) as usize;
            if index + 4 <= canvas.len() {
                canvas[index..index + 4].copy_from_slice(&color.to_le_bytes());
            }
        }
    }
}

impl CompositorHandler for OsdSurface {
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
            eprintln!("osd draw failed: {err}");
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

impl OutputHandler for OsdSurface {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for OsdSurface {
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
            eprintln!("osd draw failed: {err}");
            self.exit = true;
        }
    }
}

impl SeatHandler for OsdSurface {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}
    fn new_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }
    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }
    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {
    }
}

impl ShmHandler for OsdSurface {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for OsdSurface {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(OsdSurface);
delegate_output!(OsdSurface);
delegate_shm!(OsdSurface);
delegate_seat!(OsdSurface);
delegate_layer!(OsdSurface);
delegate_registry!(OsdSurface);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osd_message_reports_ready_without_action() {
        let (message, detail) = osd_message(None);
        assert_eq!(message, "Hyprbole ready");
        assert_eq!(detail, "No recent action");
    }

    #[test]
    fn osd_message_formats_action_status() {
        let action = hyprbole_core::daemon::ActionStatus {
            domain: "audio".to_string(),
            name: "volume_up".to_string(),
            ok: true,
            message: "audio action completed".to_string(),
        };
        let (message, detail) = osd_message(Some(&action));
        assert_eq!(message, "OK audio.volume_up");
        assert_eq!(detail, "audio action completed");
    }

    #[test]
    fn action_events_show_osd() {
        let event = hyprbole_core::daemon::ShellEvent::Action {
            status: hyprbole_core::daemon::ActionStatus {
                domain: "brightness".to_string(),
                name: "up".to_string(),
                ok: true,
                message: "brightness adjusted".to_string(),
            },
        };
        assert!(event_shows_osd(&event));
    }

    #[test]
    fn passive_refresh_events_do_not_show_osd() {
        let event = hyprbole_core::daemon::ShellEvent::StateChanged {
            reason: "theme_changed".to_string(),
            domain: Some("theme".to_string()),
        };
        assert!(!event_shows_osd(&event));
    }

    #[test]
    fn offline_refresh_preserves_optional_snapshot_fields() {
        let refresh = OsdRefresh::offline(7, "Daemon offline");
        assert_eq!(refresh.request_id, 7);
        assert!(!refresh.connected);
        assert_eq!(refresh.message, "Daemon offline");
        assert!(refresh.audio.is_none());
        assert!(refresh.brightness.is_none());
        assert!(refresh.colors.is_none());
    }

    #[test]
    fn initial_event_cursor_starts_after_retained_history() {
        assert_eq!(cursor_before_next_event(42), 41);
        assert_eq!(cursor_before_next_event(0), 0);
    }
}
