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

use crate::bar::{draw_text, truncate_text};

#[derive(Clone, Copy)]
struct QuickColors {
    background: u32,
    text: u32,
    accent_text: u32,
    success: u32,
    warning: u32,
    error: u32,
}

impl Default for QuickColors {
    fn default() -> Self {
        Self {
            background: 0xff111827,
            text: 0xffdce6ff,
            accent_text: 0xff0b1020,
            success: 0xff7ee787,
            warning: 0xfff6ca6c,
            error: 0xffff7b72,
        }
    }
}

impl QuickColors {
    fn from_tokens(tokens: &hyprbole_core::theme::ColorTokens) -> Self {
        let fallback = Self::default();
        Self {
            background: argb_from_hex(&tokens.background).unwrap_or(fallback.background),
            text: argb_from_hex(&tokens.text).unwrap_or(fallback.text),
            accent_text: argb_from_hex(&tokens.accent_text).unwrap_or(fallback.accent_text),
            success: argb_from_hex(&tokens.success).unwrap_or(fallback.success),
            warning: argb_from_hex(&tokens.warning).unwrap_or(fallback.warning),
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

const WIDTH: u32 = 380;
const HEIGHT: u32 = 268;

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
    let layer = layer_shell.create_layer_surface(
        &qh,
        surface,
        Layer::Overlay,
        Some("hyprbole-quick"),
        None,
    );
    layer.set_anchor(Anchor::TOP | Anchor::RIGHT);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.set_margin(46, 14, 0, 0);
    layer.set_exclusive_zone(0);
    layer.set_size(WIDTH, HEIGHT);
    layer.commit();

    let pool = SlotPool::new((WIDTH * HEIGHT * 4) as usize, &shm)
        .map_err(|err| format!("create shm pool: {err}"))?;
    let mut app = QuickPopup {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        pool,
        layer,
        width: WIDTH,
        height: HEIGHT,
        connected: false,
        audio: String::new(),
        brightness: String::new(),
        layout: String::new(),
        monitor: String::new(),
        action: String::new(),
        colors: QuickColors::default(),
        regions: quick_regions(WIDTH, HEIGHT),
        last_update: None,
        event_rx: spawn_event_listener(),
        action_tx: spawn_action_worker(),
        pointer: None,
        first_configure: true,
        exit: false,
    };
    app.update_daemon_state();

    while !app.exit {
        if dismiss_requested() {
            app.exit = true;
            break;
        }
        dispatch_wayland(&mut event_queue, &mut app)?;
        let changed = app.consume_events();
        if changed || app.refresh_due() {
            let _ = app.draw(&qh);
        }
    }
    Ok(())
}

fn dispatch_wayland(
    event_queue: &mut wayland_client::EventQueue<QuickPopup>,
    app: &mut QuickPopup,
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

struct QuickPopup {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    pool: SlotPool,
    layer: LayerSurface,
    width: u32,
    height: u32,
    connected: bool,
    audio: String,
    brightness: String,
    layout: String,
    monitor: String,
    action: String,
    colors: QuickColors,
    regions: Vec<QuickRegion>,
    last_update: Option<Instant>,
    event_rx: Receiver<()>,
    action_tx: Sender<QuickAction>,
    pointer: Option<wl_pointer::WlPointer>,
    first_configure: bool,
    exit: bool,
}

impl QuickPopup {
    fn consume_events(&mut self) -> bool {
        let mut changed = false;
        while self.event_rx.try_recv().is_ok() {
            changed = true;
        }
        if changed {
            self.last_update = None;
            self.update_daemon_state();
        }
        changed
    }

    fn refresh_due(&self) -> bool {
        self.last_update
            .is_none_or(|last| last.elapsed() > Duration::from_secs(2))
    }

    fn update_daemon_state(&mut self) {
        if self
            .last_update
            .is_some_and(|last| last.elapsed() < Duration::from_millis(400))
        {
            return;
        }
        self.last_update = Some(Instant::now());
        let Ok(client) = hyprbole_core::daemon::DaemonClient::from_env() else {
            self.connected = false;
            self.audio = "audio unavailable".to_string();
            self.brightness = "brightness unavailable".to_string();
            self.layout.clear();
            self.monitor.clear();
            self.action.clear();
            return;
        };
        let Ok(hyprbole_core::daemon::ShellResponse::State { snapshot }) =
            client.send_shell(&hyprbole_core::daemon::ShellRequest::StateGet)
        else {
            self.connected = false;
            self.action.clear();
            return;
        };
        self.connected = true;
        self.audio = snapshot.audio_summary;
        self.brightness = snapshot.brightness_summary;
        self.layout = snapshot
            .current_layout
            .unwrap_or_else(|| "unknown".to_string());
        self.monitor = snapshot
            .monitors
            .first()
            .and_then(|monitor| monitor.current_mode.as_ref())
            .map(|mode| mode.label.clone())
            .unwrap_or_else(|| "monitor unknown".to_string());
        self.colors = QuickColors::from_tokens(&snapshot.theme.tokens.colors);
        if let Ok(hyprbole_core::daemon::ShellResponse::Status { daemon }) =
            client.send_shell(&hyprbole_core::daemon::ShellRequest::StatusGet)
        {
            self.action = daemon
                .last_action
                .map(|action| {
                    format!(
                        "{} {}.{}",
                        if action.ok { "ok" } else { "err" },
                        action.domain,
                        action.name
                    )
                })
                .unwrap_or_default();
        } else {
            self.connected = false;
            self.action.clear();
        }
    }

    fn draw(&mut self, _qh: &QueueHandle<Self>) -> Result<(), String> {
        self.update_daemon_state();
        self.regions = quick_regions(self.width, self.height);
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
        for pixel in canvas.chunks_exact_mut(4) {
            pixel.copy_from_slice(&self.colors.background.to_le_bytes());
        }
        for region in &self.regions {
            draw_rect(canvas, self.width, region.rect, region.color(self.colors));
        }
        draw_text(
            canvas,
            self.width,
            14,
            14,
            "HYPRBOLE QUICK",
            self.colors.text,
        );
        draw_text(
            canvas,
            self.width,
            14,
            34,
            if self.connected {
                "DAEMON CONNECTED"
            } else {
                "DAEMON OFFLINE"
            },
            if self.connected {
                self.colors.success
            } else {
                self.colors.error
            },
        );
        draw_text(
            canvas,
            self.width,
            14,
            62,
            &format!("AUD {}", truncate_text(&self.audio, 34)),
            self.colors.text,
        );
        draw_text(
            canvas,
            self.width,
            14,
            82,
            &format!("BRI {}", truncate_text(&self.brightness, 34)),
            self.colors.text,
        );
        draw_text(
            canvas,
            self.width,
            14,
            102,
            &format!("LAY {}", truncate_text(&self.layout, 22)),
            self.colors.text,
        );
        draw_text(
            canvas,
            self.width,
            14,
            122,
            &format!("MON {}", truncate_text(&self.monitor, 32)),
            self.colors.text,
        );
        if !self.action.is_empty() {
            draw_text(
                canvas,
                self.width,
                14,
                142,
                &format!("ACT {}", truncate_text(&self.action, 32)),
                self.colors.warning,
            );
        }
        for region in &self.regions {
            draw_text(
                canvas,
                self.width,
                region.rect.x + 8,
                region.rect.y + 9,
                region.label(),
                self.colors.accent_text,
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

    fn click(&mut self, x: f64, y: f64) {
        let Some(action) = hit_quick_region(&self.regions, x, y) else {
            return;
        };
        if action == QuickAction::Close {
            self.exit = true;
            return;
        }
        let _ = self.action_tx.send(action);
        self.last_update = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuickAction {
    Close,
    VolumeDown,
    Mute,
    VolumeUp,
    BrightnessDown,
    BrightnessUp,
    LayoutDwindle,
    LayoutMaster,
    LayoutScrolling,
    Lock,
}

impl QuickAction {
    fn label(self) -> &'static str {
        match self {
            Self::Close => "CLOSE",
            Self::VolumeDown => "VOL -",
            Self::Mute => "MUTE",
            Self::VolumeUp => "VOL +",
            Self::BrightnessDown => "DIM",
            Self::BrightnessUp => "BRIGHT",
            Self::LayoutDwindle => "DWINDLE",
            Self::LayoutMaster => "MASTER",
            Self::LayoutScrolling => "SCROLL",
            Self::Lock => "LOCK",
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QuickRegion {
    action: QuickAction,
    rect: HitRect,
}

impl QuickRegion {
    fn label(self) -> &'static str {
        self.action.label()
    }

    fn color(self, colors: QuickColors) -> u32 {
        match self.action {
            QuickAction::Close => colors.error,
            QuickAction::Lock => colors.warning,
            _ => colors.success,
        }
    }
}

fn quick_regions(width: u32, _height: u32) -> Vec<QuickRegion> {
    let mut regions = Vec::new();
    let button = |action, x, y, w| QuickRegion {
        action,
        rect: HitRect {
            x,
            y,
            width: w,
            height: 28,
        },
    };
    regions.push(button(QuickAction::Close, width.saturating_sub(78), 10, 62));
    regions.push(button(QuickAction::VolumeDown, 14, 168, 66));
    regions.push(button(QuickAction::Mute, 88, 168, 62));
    regions.push(button(QuickAction::VolumeUp, 158, 168, 66));
    regions.push(button(QuickAction::BrightnessDown, 232, 168, 54));
    regions.push(button(QuickAction::BrightnessUp, 294, 168, 72));
    regions.push(button(QuickAction::LayoutDwindle, 14, 204, 78));
    regions.push(button(QuickAction::LayoutMaster, 100, 204, 72));
    regions.push(button(QuickAction::LayoutScrolling, 180, 204, 72));
    regions.push(button(QuickAction::Lock, 260, 204, 64));
    regions
}

fn hit_quick_region(regions: &[QuickRegion], x: f64, y: f64) -> Option<QuickAction> {
    regions
        .iter()
        .find(|region| region.rect.contains(x, y))
        .map(|region| region.action)
}

fn draw_rect(canvas: &mut [u8], width: u32, rect: HitRect, color: u32) {
    for y in rect.y..rect.y.saturating_add(rect.height) {
        for x in rect.x..rect.x.saturating_add(rect.width) {
            let index = ((y * width + x) * 4) as usize;
            if index + 4 <= canvas.len() {
                canvas[index..index + 4].copy_from_slice(&color.to_le_bytes());
            }
        }
    }
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

fn spawn_action_worker() -> Sender<QuickAction> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        while let Ok(action) = rx.recv() {
            if let Err(err) = dispatch_quick_action(action) {
                eprintln!("quick action failed: {err}");
            }
        }
    });
    tx
}

fn dispatch_quick_action(action: QuickAction) -> Result<(), String> {
    let action = match action {
        QuickAction::Close => return Ok(()),
        QuickAction::VolumeDown => hyprbole_core::daemon::ShellAction::Audio {
            action: hyprbole_core::audio::AudioAction::VolumeDown,
        },
        QuickAction::Mute => hyprbole_core::daemon::ShellAction::Audio {
            action: hyprbole_core::audio::AudioAction::ToggleMute,
        },
        QuickAction::VolumeUp => hyprbole_core::daemon::ShellAction::Audio {
            action: hyprbole_core::audio::AudioAction::VolumeUp,
        },
        QuickAction::BrightnessDown => hyprbole_core::daemon::ShellAction::Brightness {
            action: hyprbole_core::brightness::BrightnessAction::Decrease,
        },
        QuickAction::BrightnessUp => hyprbole_core::daemon::ShellAction::Brightness {
            action: hyprbole_core::brightness::BrightnessAction::Increase,
        },
        QuickAction::LayoutDwindle => layout_action("dwindle"),
        QuickAction::LayoutMaster => layout_action("master"),
        QuickAction::LayoutScrolling => layout_action("scrolling"),
        QuickAction::Lock => hyprbole_core::daemon::ShellAction::SessionLock,
    };
    let client = hyprbole_core::daemon::DaemonClient::from_env().map_err(|err| err.to_string())?;
    match client.send_shell(&hyprbole_core::daemon::ShellRequest::ActionCall { action }) {
        Ok(hyprbole_core::daemon::ShellResponse::Ok { .. }) => Ok(()),
        Ok(hyprbole_core::daemon::ShellResponse::Error { message }) => Err(message),
        Ok(response) => Err(format!("unexpected daemon response: {response:?}")),
        Err(err) => Err(err.to_string()),
    }
}

fn layout_action(name: &str) -> hyprbole_core::daemon::ShellAction {
    hyprbole_core::daemon::ShellAction::Compositor {
        action: hyprbole_core::compositor::CompositorAction::SetLayout {
            name: name.to_string(),
        },
    }
}

fn dismiss_requested() -> bool {
    let Ok(path) =
        hyprbole_core::runtime::ensure_runtime_dir().map(|dir| dir.join("quick.dismiss"))
    else {
        return false;
    };
    if !path.exists() {
        return false;
    }
    let requested = std::fs::read_to_string(&path).ok();
    let _ = std::fs::remove_file(path);
    requested.as_deref().map(str::trim) == quick_process_identity(std::process::id()).as_deref()
}

fn quick_process_identity(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_name = stat.rsplit_once(") ")?.1;
    let fields = after_name.split_whitespace().collect::<Vec<_>>();
    let start_time = fields.get(19)?;
    Some(format!("{pid}:{start_time}"))
}

impl CompositorHandler for QuickPopup {
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
            eprintln!("quick layer draw failed: {err}");
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

impl OutputHandler for QuickPopup {
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

impl LayerShellHandler for QuickPopup {
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
            eprintln!("quick layer draw failed: {err}");
            self.exit = true;
        }
    }
}

impl SeatHandler for QuickPopup {
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
                Err(err) => eprintln!("quick pointer setup failed: {err}"),
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

impl PointerHandler for QuickPopup {
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
            if let PointerEventKind::Press { button: 0x110, .. } = event.kind {
                self.click(event.position.0, event.position.1);
            }
        }
    }
}

impl ShmHandler for QuickPopup {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(QuickPopup);
delegate_output!(QuickPopup);
delegate_shm!(QuickPopup);
delegate_seat!(QuickPopup);
delegate_pointer!(QuickPopup);
delegate_layer!(QuickPopup);
delegate_registry!(QuickPopup);

impl ProvidesRegistryState for QuickPopup {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar::text_width;

    #[test]
    fn quick_regions_expose_actions() {
        let regions = quick_regions(WIDTH, HEIGHT);
        assert_eq!(
            hit_quick_region(&regions, 20.0, 174.0),
            Some(QuickAction::VolumeDown)
        );
        assert_eq!(
            hit_quick_region(&regions, 95.0, 174.0),
            Some(QuickAction::Mute)
        );
        assert_eq!(
            hit_quick_region(&regions, 306.0, 174.0),
            Some(QuickAction::BrightnessUp)
        );
        assert_eq!(
            hit_quick_region(&regions, 24.0, 212.0),
            Some(QuickAction::LayoutDwindle)
        );
        assert_eq!(
            hit_quick_region(&regions, (WIDTH - 20) as f64, 18.0),
            Some(QuickAction::Close)
        );
        assert_eq!(hit_quick_region(&regions, 1.0, 1.0), None);
    }

    #[test]
    fn quick_labels_are_short_enough_for_buttons() {
        for region in quick_regions(WIDTH, HEIGHT) {
            assert!(text_width(region.label()) + 16 <= region.rect.width);
        }
    }
}
