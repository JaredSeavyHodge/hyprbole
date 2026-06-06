use std::cell::RefCell;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

use eframe::egui::{self, Color32, CornerRadius, FontId, Margin, RichText, Stroke, Vec2};
use gtk4::gdk;
use gtk4::prelude::*;
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
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
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
};

use crate::bar::{draw_text, truncate_text};

const LAYER_WIDTH: u32 = 720;
const LAYER_HEIGHT: u32 = 392;
const LAYER_MAX_LINES: usize = 12;

pub struct LayerLauncherOptions {
    pub stdin: bool,
    pub prompt: String,
    pub placeholder: String,
    pub lines: usize,
}

impl Default for LayerLauncherOptions {
    fn default() -> Self {
        Self {
            stdin: false,
            prompt: "Launch".to_string(),
            placeholder: "Type to filter".to_string(),
            lines: 7,
        }
    }
}

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Hyprbole Launcher")
            .with_inner_size([640.0, 520.0])
            .with_min_inner_size([420.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Hyprbole Launcher",
        options,
        Box::new(|cc| {
            install_launcher_style(&cc.egui_ctx);
            Ok(Box::new(LauncherApp::default()))
        }),
    )
}

pub fn run_gtk() -> Result<(), String> {
    let app = gtk4::Application::builder()
        .application_id("dev.hyprbole.launcher")
        .build();
    app.connect_activate(build_gtk_launcher);
    app.run_with_args(&[] as &[&str]);
    Ok(())
}

fn build_gtk_launcher(app: &gtk4::Application) {
    install_gtk_launcher_css();

    let apps = Rc::new(load_desktop_apps());
    let selected = Rc::new(RefCell::new(0usize));
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("Hyprbole Launcher")
        .default_width(680)
        .default_height(64)
        .resizable(false)
        .decorated(false)
        .build();

    gtk4_layer_shell::LayerShell::init_layer_shell(&window);
    gtk4_layer_shell::LayerShell::set_layer(&window, gtk4_layer_shell::Layer::Overlay);
    gtk4_layer_shell::LayerShell::set_anchor(&window, gtk4_layer_shell::Edge::Top, true);
    gtk4_layer_shell::LayerShell::set_margin(&window, gtk4_layer_shell::Edge::Top, 96);
    gtk4_layer_shell::LayerShell::set_keyboard_mode(
        &window,
        gtk4_layer_shell::KeyboardMode::Exclusive,
    );

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    root.add_css_class("launcher-root");
    let entry = gtk4::SearchEntry::builder()
        .placeholder_text("Spotlight Search")
        .hexpand(true)
        .build();
    entry.add_css_class("search-entry");
    root.append(&entry);

    let results_shell = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    results_shell.add_css_class("results-card");
    results_shell.set_visible(false);
    let status = gtk4::Label::new(None);
    status.add_css_class("status");
    status.set_xalign(0.0);
    let list = gtk4::ListBox::new();
    list.add_css_class("results-list");
    list.set_selection_mode(gtk4::SelectionMode::Single);
    results_shell.append(&status);
    results_shell.append(&list);
    root.append(&results_shell);
    window.set_child(Some(&root));

    refresh_gtk_results(
        &apps,
        entry.text().as_str(),
        &list,
        &status,
        &results_shell,
        &selected,
    );

    let refresh_apps = apps.clone();
    let refresh_list = list.clone();
    let refresh_status = status.clone();
    let refresh_shell = results_shell.clone();
    let refresh_selected = selected.clone();
    entry.connect_search_changed(move |entry| {
        *refresh_selected.borrow_mut() = 0;
        refresh_gtk_results(
            &refresh_apps,
            entry.text().as_str(),
            &refresh_list,
            &refresh_status,
            &refresh_shell,
            &refresh_selected,
        );
    });

    let activate_apps = apps.clone();
    let activate_window = window.clone();
    let activate_selected = selected.clone();
    entry.connect_activate(move |entry| {
        if let Some(app) = gtk_selected_app(
            &activate_apps,
            entry.text().as_str(),
            *activate_selected.borrow(),
        ) && launch_app(app).starts_with("Launched ")
        {
            activate_window.close();
        }
    });

    let row_apps = apps.clone();
    let row_entry = entry.clone();
    let row_window = window.clone();
    list.connect_row_activated(move |_list, row| {
        if let Some(app) = gtk_selected_app(
            &row_apps,
            row_entry.text().as_str(),
            row.index().max(0) as usize,
        ) && launch_app(app).starts_with("Launched ")
        {
            row_window.close();
        }
    });

    let key_entry = entry.clone();
    let key_list = list.clone();
    let key_window = window.clone();
    let key_selected = selected.clone();
    let keys = gtk4::EventControllerKey::new();
    keys.connect_key_pressed(move |_controller, key, _code, _state| match key {
        gdk::Key::Escape => {
            key_window.close();
            gtk4::glib::Propagation::Stop
        }
        gdk::Key::Down => {
            move_gtk_selection(&key_list, &key_selected, 1);
            gtk4::glib::Propagation::Stop
        }
        gdk::Key::Up => {
            move_gtk_selection(&key_list, &key_selected, -1);
            gtk4::glib::Propagation::Stop
        }
        gdk::Key::Return => {
            key_entry.activate();
            gtk4::glib::Propagation::Stop
        }
        _ => gtk4::glib::Propagation::Proceed,
    });
    window.add_controller(keys);

    let entry_key_entry = entry.clone();
    let entry_key_list = list.clone();
    let entry_key_window = window.clone();
    let entry_key_selected = selected.clone();
    let entry_keys = gtk4::EventControllerKey::new();
    entry_keys.connect_key_pressed(move |_controller, key, _code, _state| match key {
        gdk::Key::Escape => {
            entry_key_window.close();
            gtk4::glib::Propagation::Stop
        }
        gdk::Key::Down => {
            move_gtk_selection(&entry_key_list, &entry_key_selected, 1);
            gtk4::glib::Propagation::Stop
        }
        gdk::Key::Up => {
            move_gtk_selection(&entry_key_list, &entry_key_selected, -1);
            gtk4::glib::Propagation::Stop
        }
        gdk::Key::Return => {
            entry_key_entry.activate();
            gtk4::glib::Propagation::Stop
        }
        _ => gtk4::glib::Propagation::Proceed,
    });
    entry.add_controller(entry_keys);

    window.present();
    entry.grab_focus();
}

fn gtk_selected_app<'a>(
    apps: &'a [DesktopApp],
    query: &str,
    selected: usize,
) -> Option<&'a DesktopApp> {
    if query.trim().is_empty() {
        return None;
    }
    filtered_apps(apps, query).get(selected).copied()
}

fn refresh_gtk_results(
    apps: &[DesktopApp],
    query: &str,
    list: &gtk4::ListBox,
    status: &gtk4::Label,
    results_shell: &gtk4::Box,
    selected: &Rc<RefCell<usize>>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    if query.trim().is_empty() {
        status.set_text("Start typing to search applications");
        results_shell.set_visible(false);
        return;
    }
    results_shell.set_visible(true);
    let visible = filtered_apps(apps, query);
    if visible.is_empty() {
        status.set_text("No matching applications");
        status.add_css_class("warning");
        return;
    }
    status.remove_css_class("warning");
    status.set_text("Use arrows to choose, Enter to launch, Esc to close");
    for app in visible.into_iter().take(7) {
        list.append(&gtk_launcher_row(app));
    }
    let len = list.observe_children().n_items() as usize;
    if len == 0 {
        *selected.borrow_mut() = 0;
        return;
    }
    let current = *selected.borrow();
    *selected.borrow_mut() = current.min(len - 1);
    if let Some(row) = list.row_at_index(*selected.borrow() as i32) {
        list.select_row(Some(&row));
    }
}

fn gtk_launcher_row(app: &DesktopApp) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    row.add_css_class("result-row");
    let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let badge = gtk4::Label::new(Some("A"));
    badge.add_css_class("app-badge");
    let labels = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    labels.set_hexpand(true);
    let name = gtk4::Label::new(Some(&app.name));
    name.add_css_class("app-name");
    name.set_xalign(0.0);
    let command = gtk4::Label::new(Some(&app.command));
    command.add_css_class("app-command");
    command.set_xalign(0.0);
    labels.append(&name);
    labels.append(&command);
    content.append(&badge);
    content.append(&labels);
    row.set_child(Some(&content));
    row
}

fn move_gtk_selection(list: &gtk4::ListBox, selected: &Rc<RefCell<usize>>, delta: isize) {
    let len = list.observe_children().n_items() as usize;
    if len == 0 {
        *selected.borrow_mut() = 0;
        return;
    }
    let next = (*selected.borrow() as isize + delta).clamp(0, len.saturating_sub(1) as isize);
    *selected.borrow_mut() = next as usize;
    if let Some(row) = list.row_at_index(next as i32) {
        list.select_row(Some(&row));
    }
}

fn install_gtk_launcher_css() {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(
        r#"
        window { background: #171a21; }
        .launcher-root {
            padding: 0;
            background: #20242d;
            border: 0;
        }
        .search-entry {
            min-height: 64px;
            background: #20242d;
            border: 1px solid #343a46;
            box-shadow: none;
            color: #f1f3f6;
            font-size: 22px;
            padding: 0 14px;
        }
        .search-entry text { color: #f1f3f6; }
        .search-entry placeholder { color: #677489; }
        .results-card {
            padding: 8px;
            background: rgba(26, 29, 36, 0.96);
            border: 1px solid #343a46;
        }
        .status { color: #a7afbd; font-size: 13px; padding: 4px 8px; }
        .status.warning { color: #f2cc60; }
        .results-list { background: transparent; }
        .result-row {
            min-height: 48px;
            padding: 8px 12px;
            margin: 2px 0;
            background: #252a33;
            color: #f1f3f6;
        }
        .result-row:selected { background: #8fb4ff; color: #0d1117; }
        .app-badge {
            min-width: 28px;
            min-height: 28px;
            background: #343a46;
            border-radius: 8px;
            color: #a7afbd;
            font-weight: 700;
        }
        .app-name { color: inherit; font-size: 15px; font-weight: 700; }
        .app-command { color: #a7afbd; font-size: 12px; }
        .result-row:selected .app-command { color: #263142; }
        .result-row:selected .app-badge { background: #0d1117; color: #8fb4ff; }
        "#,
    );
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn run_layer(options: LayerLauncherOptions) -> Result<(), String> {
    let lines = options.lines.clamp(1, LAYER_MAX_LINES);
    let height = layer_height(lines);
    let _guard = LauncherInstanceGuard::acquire()?;
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
        Some("hyprbole-launcher"),
        None,
    );
    layer.set_anchor(Anchor::TOP);
    layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    layer.set_margin(96, 0, 0, 0);
    layer.set_exclusive_zone(0);
    layer.set_size(LAYER_WIDTH, height);
    layer.commit();

    let pool = SlotPool::new((LAYER_WIDTH * height * 4) as usize, &shm)
        .map_err(|err| format!("create shm pool: {err}"))?;
    let entries = if options.stdin {
        load_stdin_entries().map_err(|err| format!("read launcher stdin: {err}"))?
    } else {
        load_desktop_apps()
            .into_iter()
            .map(LauncherEntry::from_app)
            .collect()
    };
    let mut app = LayerLauncher {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        pool,
        layer,
        width: LAYER_WIDTH,
        height,
        entries,
        stdin_mode: options.stdin,
        prompt: options.prompt,
        placeholder: options.placeholder,
        lines,
        query: String::new(),
        selected: 0,
        status: String::new(),
        regions: Vec::new(),
        keyboard: None,
        pointer: None,
        first_configure: true,
        exit: false,
    };

    while !app.exit {
        dispatch_layer_wayland(&mut event_queue, &mut app)?;
    }
    Ok(())
}

fn layer_height(lines: usize) -> u32 {
    126 + lines.clamp(1, LAYER_MAX_LINES) as u32 * 38
}

fn dispatch_layer_wayland(
    event_queue: &mut wayland_client::EventQueue<LayerLauncher>,
    app: &mut LayerLauncher,
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

struct LauncherInstanceGuard {
    path: PathBuf,
    identity: String,
}

impl LauncherInstanceGuard {
    fn acquire() -> Result<Self, String> {
        let dir = hyprbole_core::runtime::ensure_runtime_dir()
            .map_err(|err| format!("ensure runtime dir: {err}"))?;
        let path = dir.join("launcher.pid");
        let identity = launcher_process_identity(std::process::id())
            .ok_or_else(|| "read launcher process identity".to_string())?;
        loop {
            if let Some((pid, _identity)) = running_launcher_layer_excluding(std::process::id()) {
                return Err(format!("launcher already running pid {pid}"));
            }
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    file.write_all(identity.as_bytes()).map_err(|err| {
                        format!("write launcher pidfile {}: {err}", path.display())
                    })?;
                    return Ok(Self { path, identity });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    if let Ok(value) = fs::read_to_string(&path)
                        && let Some(existing) = parse_launcher_identity(&value)
                        && launcher_identity_alive(&existing)
                    {
                        return Err(format!("launcher already running pid {}", existing.0));
                    }
                    let _ = fs::remove_file(&path);
                }
                Err(err) => {
                    return Err(format!("open launcher pidfile {}: {err}", path.display()));
                }
            }
        }
    }
}

impl Drop for LauncherInstanceGuard {
    fn drop(&mut self) {
        if fs::read_to_string(&self.path).ok().as_deref() == Some(self.identity.as_str()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn launcher_process_identity(pid: u32) -> Option<String> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_name = stat.rsplit_once(") ")?.1;
    let fields = after_name.split_whitespace().collect::<Vec<_>>();
    let start_time = fields.get(19)?;
    Some(format!("{pid}:{start_time}"))
}

fn parse_launcher_identity(value: &str) -> Option<(u32, String)> {
    let value = value.trim();
    let (pid, start_time) = value.split_once(':')?;
    let pid = pid.parse().ok()?;
    start_time.parse::<u64>().ok()?;
    Some((pid, value.to_string()))
}

fn launcher_identity_alive(identity: &(u32, String)) -> bool {
    launcher_process_alive(identity.0)
        && launcher_process_identity(identity.0).as_deref() == Some(&identity.1)
}

fn launcher_process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
        && fs::read(format!("/proc/{pid}/cmdline"))
            .map(|cmdline| launcher_cmdline_is_layer(&cmdline))
            .unwrap_or(false)
}

fn running_launcher_layer_excluding(excluded_pid: u32) -> Option<(u32, String)> {
    let entries = fs::read_dir("/proc").ok()?;
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == excluded_pid || !launcher_process_alive(pid) {
            continue;
        }
        if let Some(identity) = launcher_process_identity(pid) {
            return Some((pid, identity));
        }
    }
    None
}

fn launcher_cmdline_is_layer(cmdline: &[u8]) -> bool {
    let args = cmdline
        .split(|byte| *byte == 0)
        .filter_map(|arg| std::str::from_utf8(arg).ok())
        .collect::<Vec<_>>();
    args.first().is_some_and(|arg| arg.ends_with("hyprbole-ui"))
        && args.contains(&"--launcher-layer")
}

struct LauncherApp {
    query: String,
    last_query: String,
    apps: Vec<DesktopApp>,
    selected: usize,
    status: String,
}

impl Default for LauncherApp {
    fn default() -> Self {
        Self {
            query: String::new(),
            last_query: String::new(),
            apps: load_desktop_apps(),
            selected: 0,
            status: String::new(),
        }
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.query != self.last_query {
            self.last_query = self.query.clone();
            self.selected = 0;
        }
        let filtered = filtered_apps(&self.apps, &self.query);
        let visible = if self.query.trim().is_empty() {
            Vec::new()
        } else {
            filtered.iter().take(9).copied().collect::<Vec<_>>()
        };
        self.selected = self.selected.min(visible.len().saturating_sub(1));
        if ctx.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
            self.selected = (self.selected + 1).min(visible.len().saturating_sub(1));
        }
        if ctx.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
            self.selected = self.selected.saturating_sub(1);
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Enter))
            && let Some(app) = visible.get(self.selected)
        {
            self.status = launch_app(app);
            if self.status.starts_with("Launched ") {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(17, 19, 24)))
            .show(ctx, |ui| {
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Launch")
                                .font(FontId::proportional(34.0))
                                .strong()
                                .color(Color32::from_rgb(241, 243, 246)),
                        );
                        ui.label(
                            RichText::new("Applications from your desktop entries")
                                .color(Color32::from_rgb(167, 175, 189)),
                        );
                    });
                });
                ui.add_space(16.0);
                egui::Frame::new()
                    .fill(Color32::from_rgb(26, 29, 36))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(52, 58, 70)))
                    .corner_radius(CornerRadius::same(18))
                    .inner_margin(Margin::same(18))
                    .show(ui, |ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.query)
                                .hint_text("Type an app name")
                                .desired_width(f32::INFINITY),
                        );
                        response.request_focus();
                        ui.add_space(12.0);
                        if self.query.trim().is_empty() {
                            ui.label(
                                RichText::new("Start typing to search applications")
                                    .color(Color32::from_rgb(167, 175, 189)),
                            );
                        } else if filtered.is_empty() {
                            ui.label(
                                RichText::new("No matching applications")
                                    .color(Color32::from_rgb(242, 204, 96)),
                            );
                        }
                        for (index, app) in visible.iter().enumerate() {
                            let selected = index == self.selected;
                            let fill = if selected {
                                Color32::from_rgb(143, 180, 255)
                            } else {
                                Color32::from_rgb(37, 42, 51)
                            };
                            let text = if selected {
                                Color32::from_rgb(13, 17, 23)
                            } else {
                                Color32::from_rgb(241, 243, 246)
                            };
                            let response = egui::Frame::new()
                                .fill(fill)
                                .corner_radius(CornerRadius::same(12))
                                .inner_margin(Margin::symmetric(14, 10))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&app.name).strong().color(text));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(&app.command).small().color(text),
                                                );
                                            },
                                        );
                                    });
                                })
                                .response;
                            if response.clicked() {
                                self.selected = index;
                                self.status = launch_app(app);
                                if self.status.starts_with("Launched ") {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            }
                            ui.add_space(6.0);
                        }
                    });
                if !self.status.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new(&self.status).color(Color32::from_rgb(167, 175, 189)));
                }
            });
    }
}

struct LayerLauncher {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    pool: SlotPool,
    layer: LayerSurface,
    width: u32,
    height: u32,
    entries: Vec<LauncherEntry>,
    stdin_mode: bool,
    prompt: String,
    placeholder: String,
    lines: usize,
    query: String,
    selected: usize,
    status: String,
    regions: Vec<LauncherRegion>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    first_configure: bool,
    exit: bool,
}

impl LayerLauncher {
    fn visible_entries(&self) -> Vec<&LauncherEntry> {
        if !self.stdin_mode && self.query.trim().is_empty() {
            return Vec::new();
        }
        filtered_entries(&self.entries, &self.query)
            .into_iter()
            .take(self.lines)
            .collect()
    }

    fn draw(&mut self, _qh: &QueueHandle<Self>) -> Result<(), String> {
        let visible = self
            .visible_entries()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
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
        fill(canvas, 0x0000_0000);

        let search_rect = HitRect {
            x: 12,
            y: 8,
            width: self.width.saturating_sub(24),
            height: 64,
        };
        draw_rounded_rect(canvas, self.width, search_rect, 32, 0xf21a_1d24);
        draw_rounded_rect(
            canvas,
            self.width,
            HitRect {
                x: search_rect.x,
                y: search_rect.y,
                width: search_rect.width,
                height: 1,
            },
            1,
            0xff34_3a46,
        );
        let query = if self.query.is_empty() {
            if self.placeholder.trim().is_empty() {
                self.prompt.clone()
            } else {
                self.placeholder.clone()
            }
        } else {
            format!("{}|", self.query)
        };
        draw_text(canvas, self.width, 36, 30, ">", 0xff8f_b4ff);
        draw_text(
            canvas,
            self.width,
            70,
            30,
            &truncate_text(&query, 56),
            if self.query.is_empty() {
                0xff67_7489
            } else {
                0xfff1_f3f6
            },
        );

        self.regions.clear();
        if !self.query.trim().is_empty() || self.stdin_mode {
            let result_count = visible.len().max(1) as u32;
            let results_rect = HitRect {
                x: 12,
                y: 86,
                width: self.width.saturating_sub(24),
                height: 20 + result_count * 38,
            };
            draw_rounded_rect(canvas, self.width, results_rect, 22, 0xf21a_1d24);
        }

        if visible.is_empty() {
            if !self.query.trim().is_empty() || self.stdin_mode {
                draw_text(
                    canvas,
                    self.width,
                    36,
                    112,
                    "NO MATCHING ENTRIES",
                    0xffff_d166,
                );
            }
        } else {
            for (index, app) in visible.iter().enumerate() {
                let y = 96 + index as u32 * 38;
                let rect = HitRect {
                    x: 22,
                    y,
                    width: self.width.saturating_sub(44),
                    height: 32,
                };
                let selected = index == self.selected;
                draw_rounded_rect(
                    canvas,
                    self.width,
                    rect,
                    12,
                    if selected { 0xff8f_b4ff } else { 0xff25_2a33 },
                );
                draw_text(
                    canvas,
                    self.width,
                    rect.x + 12,
                    rect.y + 10,
                    &truncate_text(&app.title, 34),
                    if selected { 0xff0d_1117 } else { 0xfff1_f3f6 },
                );
                let command = truncate_text(&app.detail, 32);
                let command_x = rect
                    .x
                    .saturating_add(rect.width)
                    .saturating_sub(crate::bar::text_width(&command))
                    .saturating_sub(12);
                draw_text(
                    canvas,
                    self.width,
                    command_x,
                    rect.y + 10,
                    &command,
                    if selected { 0xff26_3142 } else { 0xffa7_afbd },
                );
                self.regions.push(LauncherRegion { index, rect });
            }
        }
        if !self.status.is_empty() {
            draw_text(
                canvas,
                self.width,
                38,
                106 + self.lines as u32 * 38,
                &truncate_text(&self.status, 70),
                0xffa7_afbd,
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

    fn move_selection(&mut self, delta: isize) {
        let len = self.visible_entries().len();
        if len == 0 {
            self.selected = 0;
            return;
        }
        self.selected =
            (self.selected as isize + delta).clamp(0, len.saturating_sub(1) as isize) as usize;
    }

    fn launch_selected(&mut self) {
        let visible = self.visible_entries();
        let Some(entry) = visible.get(self.selected) else {
            self.status = "No entry selected".to_string();
            return;
        };
        self.status = activate_entry(entry);
        if self.status.starts_with("Launched ") || self.stdin_mode {
            self.exit = true;
        }
    }

    fn click(&mut self, x: f64, y: f64) {
        let Some(index) = self
            .regions
            .iter()
            .find(|region| region.rect.contains(x, y))
            .map(|region| region.index)
        else {
            return;
        };
        self.selected = index;
        self.launch_selected();
    }

    fn handle_key(&mut self, event: KeyEvent) {
        match event.keysym {
            Keysym::Escape => self.exit = true,
            Keysym::Return => self.launch_selected(),
            Keysym::Up => self.move_selection(-1),
            Keysym::Down => self.move_selection(1),
            Keysym::BackSpace => {
                self.query.pop();
                self.selected = 0;
            }
            _ => {
                if let Some(text) = event.utf8
                    && text.chars().all(|ch| !ch.is_control())
                {
                    self.query.push_str(&text);
                    self.selected = 0;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct LauncherRegion {
    index: usize,
    rect: HitRect,
}

#[derive(Clone, Copy)]
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

fn fill(canvas: &mut [u8], color: u32) {
    for pixel in canvas.chunks_exact_mut(4) {
        pixel.copy_from_slice(&color.to_le_bytes());
    }
}

fn draw_rounded_rect(canvas: &mut [u8], width: u32, rect: HitRect, radius: u32, color: u32) {
    let radius = radius.min(rect.width / 2).min(rect.height / 2);
    let radius_squared = (radius * radius) as i64;
    let right = rect.x.saturating_add(rect.width);
    let bottom = rect.y.saturating_add(rect.height);
    for y in rect.y..bottom {
        for x in rect.x..right {
            let in_left = x < rect.x + radius;
            let in_right = x >= right.saturating_sub(radius);
            let in_top = y < rect.y + radius;
            let in_bottom = y >= bottom.saturating_sub(radius);
            if (in_left || in_right) && (in_top || in_bottom) {
                let center_x = if in_left {
                    rect.x + radius
                } else {
                    right.saturating_sub(radius + 1)
                };
                let center_y = if in_top {
                    rect.y + radius
                } else {
                    bottom.saturating_sub(radius + 1)
                };
                let dx = x as i64 - center_x as i64;
                let dy = y as i64 - center_y as i64;
                if dx * dx + dy * dy > radius_squared {
                    continue;
                }
            }
            let index = ((y * width + x) * 4) as usize;
            if index + 4 <= canvas.len() {
                canvas[index..index + 4].copy_from_slice(&color.to_le_bytes());
            }
        }
    }
}

impl CompositorHandler for LayerLauncher {
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
            eprintln!("launcher layer draw failed: {err}");
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

impl OutputHandler for LayerLauncher {
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

impl LayerShellHandler for LayerLauncher {
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
        self.width = NonZeroU32::new(configure.new_size.0).map_or(LAYER_WIDTH, NonZeroU32::get);
        self.height = NonZeroU32::new(configure.new_size.1).map_or(LAYER_HEIGHT, NonZeroU32::get);
        self.first_configure = false;
        if let Err(err) = self.draw(qh) {
            eprintln!("launcher layer draw failed: {err}");
            self.exit = true;
        }
    }
}

impl SeatHandler for LayerLauncher {
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
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            match self.seat_state.get_keyboard(qh, &seat, None) {
                Ok(keyboard) => self.keyboard = Some(keyboard),
                Err(err) => eprintln!("launcher keyboard setup failed: {err}"),
            }
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            match self.seat_state.get_pointer(qh, &seat) {
                Ok(pointer) => self.pointer = Some(pointer),
                Err(err) => eprintln!("launcher pointer setup failed: {err}"),
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
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            self.keyboard
                .take()
                .expect("keyboard checked above")
                .release();
        }
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

impl KeyboardHandler for LayerLauncher {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
    }
    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
    }
    fn press_key(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.handle_key(event);
        if !self.exit
            && let Err(err) = self.draw(qh)
        {
            eprintln!("launcher layer draw failed: {err}");
            self.exit = true;
        }
    }
    fn repeat_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        serial: u32,
        event: KeyEvent,
    ) {
        self.press_key(conn, qh, keyboard, serial, event);
    }
    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }
    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
    }
}

impl PointerHandler for LayerLauncher {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if &event.surface != self.layer.wl_surface() {
                continue;
            }
            if let PointerEventKind::Press { button: 0x110, .. } = event.kind {
                self.click(event.position.0, event.position.1);
                if !self.exit
                    && let Err(err) = self.draw(qh)
                {
                    eprintln!("launcher layer draw failed: {err}");
                    self.exit = true;
                }
            }
        }
    }
}

impl ShmHandler for LayerLauncher {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(LayerLauncher);
delegate_output!(LayerLauncher);
delegate_shm!(LayerLauncher);
delegate_seat!(LayerLauncher);
delegate_keyboard!(LayerLauncher);
delegate_pointer!(LayerLauncher);
delegate_layer!(LayerLauncher);
delegate_registry!(LayerLauncher);

impl ProvidesRegistryState for LayerLauncher {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DesktopApp {
    name: String,
    command: String,
    argv: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LauncherEntry {
    title: String,
    detail: String,
    argv: Option<Vec<String>>,
}

impl LauncherEntry {
    fn from_app(app: DesktopApp) -> Self {
        Self {
            title: app.name,
            detail: app.command,
            argv: Some(app.argv),
        }
    }
}

fn install_launcher_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.window_corner_radius = CornerRadius::same(18);
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    ctx.set_style(style);
}

fn filtered_apps<'a>(apps: &'a [DesktopApp], query: &str) -> Vec<&'a DesktopApp> {
    let query = query.trim().to_ascii_lowercase();
    let mut matches = apps
        .iter()
        .filter_map(|app| app_match_score(app, &query).map(|score| (score, app)))
        .collect::<Vec<_>>();
    matches.sort_by(|(left_score, left), (right_score, right)| {
        left_score
            .cmp(right_score)
            .then_with(|| left.name.cmp(&right.name))
    });
    matches.into_iter().map(|(_, app)| app).collect()
}

fn filtered_entries<'a>(entries: &'a [LauncherEntry], query: &str) -> Vec<&'a LauncherEntry> {
    let query = query.trim().to_ascii_lowercase();
    let mut matches = entries
        .iter()
        .filter_map(|entry| entry_match_score(entry, &query).map(|score| (score, entry)))
        .collect::<Vec<_>>();
    matches.sort_by(|(left_score, left), (right_score, right)| {
        left_score
            .cmp(right_score)
            .then_with(|| left.title.cmp(&right.title))
    });
    matches.into_iter().map(|(_, entry)| entry).collect()
}

fn entry_match_score(entry: &LauncherEntry, query: &str) -> Option<u32> {
    if query.is_empty() {
        return Some(100);
    }
    let title = entry.title.to_ascii_lowercase();
    let detail = entry.detail.to_ascii_lowercase();
    if title.starts_with(query) {
        return Some(0);
    }
    if detail.starts_with(query) {
        return Some(10);
    }
    if title.contains(query) {
        return Some(20 + title.find(query).unwrap_or_default() as u32);
    }
    if detail.contains(query) {
        return Some(40 + detail.find(query).unwrap_or_default() as u32);
    }
    fuzzy_score(&title, query).or_else(|| fuzzy_score(&detail, query).map(|score| score + 40))
}

fn app_match_score(app: &DesktopApp, query: &str) -> Option<u32> {
    if query.is_empty() {
        return Some(100);
    }
    let name = app.name.to_ascii_lowercase();
    let command = app.command.to_ascii_lowercase();
    if name.starts_with(query) {
        return Some(0);
    }
    if command.starts_with(query) {
        return Some(10);
    }
    if name.contains(query) {
        return Some(20 + name.find(query).unwrap_or_default() as u32);
    }
    if command.contains(query) {
        return Some(40 + command.find(query).unwrap_or_default() as u32);
    }
    fuzzy_score(&name, query).or_else(|| fuzzy_score(&command, query).map(|score| score + 40))
}

fn fuzzy_score(value: &str, query: &str) -> Option<u32> {
    let mut last_index = 0;
    let mut score = 80;
    for needle in query.chars() {
        let Some(offset) = value[last_index..].find(needle) else {
            return None;
        };
        score += offset as u32;
        last_index += offset + needle.len_utf8();
    }
    Some(score)
}

fn launch_app(app: &DesktopApp) -> String {
    let Some((program, args)) = app.argv.split_first() else {
        return format!("Failed to launch {}: empty command", app.name);
    };
    match Command::new(program).args(args).spawn() {
        Ok(_) => format!("Launched {}", app.name),
        Err(err) => format!("Failed to launch {}: {err}", app.name),
    }
}

fn activate_entry(entry: &LauncherEntry) -> String {
    let Some(argv) = &entry.argv else {
        println!("{}", entry.title);
        return format!("Selected {}", entry.title);
    };
    let app = DesktopApp {
        name: entry.title.clone(),
        command: entry.detail.clone(),
        argv: argv.clone(),
    };
    launch_app(&app)
}

fn load_stdin_entries() -> io::Result<Vec<LauncherEntry>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    Ok(input
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(|line| LauncherEntry {
            title: line.to_string(),
            detail: String::new(),
            argv: None,
        })
        .collect())
}

fn load_desktop_apps() -> Vec<DesktopApp> {
    let mut apps = Vec::new();
    for dir in desktop_app_dirs() {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some("desktop")
                && let Some(app) = parse_desktop_file(&path)
            {
                apps.push(app);
            }
        }
    }
    apps.sort_by(|left, right| left.name.cmp(&right.name));
    apps.dedup_by(|left, right| left.name == right.name && left.command == right.command);
    apps
}

fn desktop_app_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    }
    if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
        for dir in std::env::split_paths(&data_dirs) {
            dirs.push(dir.join("applications"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/local/share/applications"));
        dirs.push(PathBuf::from("/usr/share/applications"));
    }
    dirs
}

fn parse_desktop_file(path: &Path) -> Option<DesktopApp> {
    let contents = std::fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut try_exec = None;
    let mut hidden = false;
    let mut no_display = false;
    let mut terminal = false;
    let mut only_show_in = None;
    let mut not_show_in = None;
    for line in contents.lines() {
        let line = line.trim();
        if line == "[Desktop Entry]" {
            in_entry = true;
            continue;
        }
        if line.starts_with('[') && in_entry {
            break;
        }
        if !in_entry {
            continue;
        }
        if let Some(value) = line.strip_prefix("Name=") {
            name = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("Exec=") {
            exec = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("TryExec=") {
            try_exec = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("Hidden=") {
            hidden = value.eq_ignore_ascii_case("true");
        } else if let Some(value) = line.strip_prefix("NoDisplay=") {
            no_display = value.eq_ignore_ascii_case("true");
        } else if let Some(value) = line.strip_prefix("Terminal=") {
            terminal = value.eq_ignore_ascii_case("true");
        } else if let Some(value) = line.strip_prefix("OnlyShowIn=") {
            only_show_in = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("NotShowIn=") {
            not_show_in = Some(value.to_string());
        }
    }
    if hidden
        || no_display
        || terminal
        || !desktop_visibility_allows(only_show_in.as_deref(), not_show_in.as_deref())
    {
        return None;
    }
    if let Some(try_exec) = try_exec
        && !command_exists(&try_exec)
    {
        return None;
    }
    let argv = split_exec_command(&exec?);
    if argv.is_empty() {
        return None;
    }
    Some(DesktopApp {
        name: name?,
        command: argv.join(" "),
        argv,
    })
}

#[cfg(test)]
fn clean_exec_command(command: &str) -> String {
    split_exec_command(command).join(" ")
}

fn exec_program_and_args(command: &str) -> Option<(String, Vec<String>)> {
    let mut parts = split_exec_command(command).into_iter();
    Some((parts.next()?, parts.collect()))
}

fn split_exec_command(command: &str) -> Vec<String> {
    split_quoted(command)
        .into_iter()
        .filter_map(|part| strip_field_codes(&part))
        .collect()
}

fn strip_field_codes(part: &str) -> Option<String> {
    let mut output = String::new();
    let mut chars = part.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('%') => output.push('%'),
            Some('f' | 'F' | 'u' | 'U' | 'd' | 'D' | 'n' | 'N' | 'i' | 'c' | 'k' | 'v' | 'm') => {}
            Some(other) => {
                output.push('%');
                output.push(other);
            }
            None => output.push('%'),
        }
    }
    (!output.is_empty()).then_some(output)
}

fn desktop_visibility_allows(only_show_in: Option<&str>, not_show_in: Option<&str>) -> bool {
    let current = current_desktops();
    if let Some(only_show_in) = only_show_in
        && !desktop_list_contains(only_show_in, &current)
    {
        return false;
    }
    if let Some(not_show_in) = not_show_in
        && desktop_list_contains(not_show_in, &current)
    {
        return false;
    }
    true
}

fn current_desktops() -> Vec<String> {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_else(|_| "Hyprland".to_string())
        .split(':')
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn desktop_list_contains(list: &str, current: &[String]) -> bool {
    list.split(';')
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .any(|value| current.iter().any(|current| current == &value))
}

fn command_exists(command: &str) -> bool {
    let Some((program, _args)) = exec_program_and_args(command) else {
        return false;
    };
    let path = Path::new(&program);
    if path.components().count() > 1 {
        return path.exists();
    }
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(&program).exists()))
}

fn split_quoted(value: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        match quote {
            Some(active) if ch == active => quote = None,
            Some(_) => current.push(ch),
            None if ch == '\'' || ch == '"' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_exec_command_strips_desktop_placeholders() {
        assert_eq!(clean_exec_command("firefox %u"), "firefox");
        assert_eq!(clean_exec_command("app --flag %F"), "app --flag");
        assert_eq!(clean_exec_command("app %% %U"), "app %");
    }

    #[test]
    fn exec_parser_preserves_quoted_arguments_without_shell() {
        assert_eq!(
            split_exec_command("'/opt/My App/app' --name 'Two Words' \"and more\" %U"),
            vec!["/opt/My App/app", "--name", "Two Words", "and more"]
        );
        assert_eq!(
            exec_program_and_args("app --flag value").unwrap(),
            (
                "app".to_string(),
                vec!["--flag".to_string(), "value".to_string()]
            )
        );
    }

    #[test]
    fn desktop_list_matching_is_case_insensitive() {
        let current = vec!["hyprland".to_string(), "wlroots".to_string()];
        assert!(desktop_list_contains("Hyprland;", &current));
        assert!(desktop_list_contains("GNOME;wlroots;", &current));
        assert!(!desktop_list_contains("GNOME;KDE;", &current));
    }

    #[test]
    fn filtered_apps_matches_name_or_command() {
        let apps = vec![
            DesktopApp {
                name: "Firefox".to_string(),
                command: "firefox".to_string(),
                argv: vec!["firefox".to_string()],
            },
            DesktopApp {
                name: "Files".to_string(),
                command: "nautilus".to_string(),
                argv: vec!["nautilus".to_string()],
            },
        ];
        assert_eq!(filtered_apps(&apps, "fox").len(), 1);
        assert_eq!(filtered_apps(&apps, "naut").len(), 1);
        assert_eq!(filtered_apps(&apps, "").len(), 2);
    }

    #[test]
    fn gtk_selection_requires_non_empty_query() {
        let apps = vec![DesktopApp {
            name: "Firefox".to_string(),
            command: "firefox".to_string(),
            argv: vec!["firefox".to_string()],
        }];
        assert!(gtk_selected_app(&apps, "", 0).is_none());
        assert_eq!(
            gtk_selected_app(&apps, "fire", 0).map(|app| app.name.as_str()),
            Some("Firefox")
        );
    }

    #[test]
    fn filtered_apps_ranks_spotlight_style_matches() {
        let apps = vec![
            DesktopApp {
                name: "Visual Studio Code".to_string(),
                command: "code".to_string(),
                argv: vec!["code".to_string()],
            },
            DesktopApp {
                name: "Calculator".to_string(),
                command: "gnome-calculator".to_string(),
                argv: vec!["gnome-calculator".to_string()],
            },
            DesktopApp {
                name: "Calendar".to_string(),
                command: "gnome-calendar".to_string(),
                argv: vec!["gnome-calendar".to_string()],
            },
        ];

        let matches = filtered_apps(&apps, "calc");
        assert_eq!(
            matches.first().map(|app| app.name.as_str()),
            Some("Calculator")
        );

        let matches = filtered_apps(&apps, "vsc");
        assert_eq!(
            matches.first().map(|app| app.name.as_str()),
            Some("Visual Studio Code")
        );
    }

    #[test]
    fn filtered_entries_supports_generic_menu_items() {
        let entries = vec![
            LauncherEntry {
                title: "switch workspace".to_string(),
                detail: "script action".to_string(),
                argv: None,
            },
            LauncherEntry {
                title: "open terminal".to_string(),
                detail: String::new(),
                argv: None,
            },
        ];

        let matches = filtered_entries(&entries, "sw");
        assert_eq!(
            matches.first().map(|entry| entry.title.as_str()),
            Some("switch workspace")
        );
    }

    #[test]
    fn layer_height_grows_with_visible_lines() {
        assert_eq!(layer_height(7), LAYER_HEIGHT);
        assert!(layer_height(12) > LAYER_HEIGHT);
        assert_eq!(layer_height(99), layer_height(LAYER_MAX_LINES));
    }

    #[test]
    fn desktop_parser_skips_entries_that_should_not_launch_directly() {
        let terminal = write_desktop(
            "terminal",
            "[Desktop Entry]\nName=Term\nExec=xterm\nTerminal=true\n",
        );
        let missing = write_desktop(
            "missing",
            "[Desktop Entry]\nName=Missing\nExec=missing-app\nTryExec=/definitely/missing/hyprbole-test\n",
        );
        let hidden = write_desktop(
            "hidden",
            "[Desktop Entry]\nName=Hidden\nExec=hidden-app\nOnlyShowIn=DefinitelyNotHyprbole;\n",
        );
        let visible = write_desktop(
            "visible",
            "[Desktop Entry]\nName=Visible\nExec=sh -c 'true' %U\nTryExec=sh\n",
        );

        assert!(parse_desktop_file(&terminal).is_none());
        assert!(parse_desktop_file(&missing).is_none());
        assert!(parse_desktop_file(&hidden).is_none());
        assert_eq!(
            parse_desktop_file(&visible),
            Some(DesktopApp {
                name: "Visible".to_string(),
                command: "sh -c true".to_string(),
                argv: vec!["sh".to_string(), "-c".to_string(), "true".to_string()],
            })
        );

        let _ = std::fs::remove_file(terminal);
        let _ = std::fs::remove_file(missing);
        let _ = std::fs::remove_file(hidden);
        let _ = std::fs::remove_file(visible);
    }

    fn write_desktop(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hyprbole-launcher-{name}-{}-{}.desktop",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, contents).unwrap();
        path
    }
}
