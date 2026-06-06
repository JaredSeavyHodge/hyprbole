use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{thread, time::Duration};

use crate::{
    bar_identity_alive, bar_instance, dismiss_quick_settings, exit_success, launch_launcher,
    launch_layer_spike, launch_osd, launch_quick, launcher_layer_identity_alive,
    launcher_layer_instance, osd_identity_alive, osd_instance, parse_quick_identity,
    quick_identity_alive, quick_settings_instance, running_bar_instance, running_osd_instance,
    running_quick_instance, stop_bar, stop_launcher_layer, stop_osd,
};

mod doctor;

pub(crate) fn surfaces_command(args: &[String]) -> ExitCode {
    if let Some(exit) = doctor::surface_doctor_command(args) {
        return exit;
    }
    if let Some(exit) = surface_check_command(args) {
        return exit;
    }
    if let Some(exit) = surface_wait_command(args) {
        return exit;
    }
    if let Some(exit) = surface_control_command(args) {
        return exit;
    }
    let Some(options) = surface_command_options(args) else {
        eprintln!("usage: hbctl surfaces [quick|bar|osd|launcher] [--plain]");
        eprintln!("       hbctl surfaces <start|stop|restart> <quick|bar|osd|launcher|all>");
        eprintln!("       hbctl surfaces clean <quick|bar|osd|launcher|all> [--dry-run]");
        eprintln!(
            "       hbctl surfaces doctor [quick|bar|osd|launcher|all] [--json|--commands] [--unhealthy-only] [--actionable-only] [--zero-ok] [--warnings-only|--errors-only]"
        );
        eprintln!(
            "       hbctl surfaces check <quick|bar|osd|launcher|all> <running|stopped|healthy|pidfile-ok|pidfile-missing|pidfile-stale|pidfile-invalid|pidfile-unreadable> [--quiet] [--json]"
        );
        eprintln!(
            "       hbctl surfaces wait <quick|bar|osd|launcher|all> <running|stopped|healthy|pidfile-ok|pidfile-missing|pidfile-stale|pidfile-invalid|pidfile-unreadable> [--timeout-ms N] [--interval-ms N] [--quiet] [--json]"
        );
        return ExitCode::from(2);
    };
    let mut statuses = surface_statuses();
    if let Some(name) = options.name {
        statuses.retain(|status| status.name == name);
    }
    if options.plain {
        print_surface_statuses_plain(&statuses);
        return ExitCode::SUCCESS;
    }
    match serde_json::to_string_pretty(&statuses) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to encode surfaces: {err}");
            ExitCode::from(1)
        }
    }
}

fn surface_check_command(args: &[String]) -> Option<ExitCode> {
    if args.first().map(String::as_str) != Some("check") {
        return None;
    }
    let [command, target, state, rest @ ..] = args else {
        surface_check_usage();
        return Some(ExitCode::from(2));
    };
    debug_assert_eq!(command, "check");
    let Some(target) = surface_target(target) else {
        surface_check_usage();
        return Some(ExitCode::from(2));
    };
    let Some(state) = surface_check_state(state) else {
        surface_check_usage();
        return Some(ExitCode::from(2));
    };
    let Some(options) = surface_check_options(rest) else {
        surface_check_usage();
        return Some(ExitCode::from(2));
    };
    Some(check_surface_state(target, state, options))
}

fn surface_check_usage() {
    eprintln!(
        "usage: hbctl surfaces check <quick|bar|osd|launcher|all> <running|stopped|healthy|pidfile-ok|pidfile-missing|pidfile-stale|pidfile-invalid|pidfile-unreadable> [--quiet] [--json]"
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SurfaceCheckState {
    Running,
    Stopped,
    Healthy,
    PidfileOk,
    PidfileMissing,
    PidfileStale,
    PidfileInvalid,
    PidfileUnreadable,
}

fn surface_check_state(value: &str) -> Option<SurfaceCheckState> {
    match value {
        "running" => Some(SurfaceCheckState::Running),
        "stopped" => Some(SurfaceCheckState::Stopped),
        "healthy" => Some(SurfaceCheckState::Healthy),
        "pidfile-ok" => Some(SurfaceCheckState::PidfileOk),
        "pidfile-missing" => Some(SurfaceCheckState::PidfileMissing),
        "pidfile-stale" => Some(SurfaceCheckState::PidfileStale),
        "pidfile-invalid" => Some(SurfaceCheckState::PidfileInvalid),
        "pidfile-unreadable" => Some(SurfaceCheckState::PidfileUnreadable),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SurfaceCheckOptions {
    quiet: bool,
    json: bool,
}

fn surface_check_options(args: &[String]) -> Option<SurfaceCheckOptions> {
    let mut options = SurfaceCheckOptions::default();
    for arg in args {
        match arg.as_str() {
            "--quiet" if !options.quiet => options.quiet = true,
            "--json" if !options.json => options.json = true,
            _ => return None,
        }
    }
    Some(options)
}

fn check_surface_state(
    target: SurfaceTarget,
    expected: SurfaceCheckState,
    options: SurfaceCheckOptions,
) -> ExitCode {
    let statuses = surface_target_statuses(target);
    let passed = surface_statuses_match_check(&statuses, expected);
    if options.json {
        let report = SurfaceCheckReport {
            target: surface_target_label(target),
            expected: surface_check_state_label(expected),
            passed,
            surfaces: statuses,
        };
        return match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                if passed {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(err) => {
                eprintln!("failed to encode surface check: {err}");
                ExitCode::from(1)
            }
        };
    }
    if passed {
        if !options.quiet {
            println!("surface check passed");
        }
        ExitCode::SUCCESS
    } else {
        if !options.quiet {
            eprintln!("surface check failed");
            print_surface_statuses_plain(&statuses);
        }
        ExitCode::from(1)
    }
}

#[derive(serde::Serialize)]
struct SurfaceCheckReport {
    target: &'static str,
    expected: &'static str,
    passed: bool,
    surfaces: Vec<SurfaceStatus>,
}

fn surface_status_matches_check(status: &SurfaceStatus, expected: SurfaceCheckState) -> bool {
    match expected {
        SurfaceCheckState::Running => status.running,
        SurfaceCheckState::Stopped => !status.running,
        SurfaceCheckState::Healthy => surface_status_healthy(status),
        SurfaceCheckState::PidfileOk => status.pid_file_state == "ok",
        SurfaceCheckState::PidfileMissing => status.pid_file_state == "missing",
        SurfaceCheckState::PidfileStale => status.pid_file_state == "stale",
        SurfaceCheckState::PidfileInvalid => status.pid_file_state == "invalid",
        SurfaceCheckState::PidfileUnreadable => status.pid_file_state == "unreadable",
    }
}

fn surface_statuses_match_check(statuses: &[SurfaceStatus], expected: SurfaceCheckState) -> bool {
    statuses
        .iter()
        .all(|status| surface_status_matches_check(status, expected))
}

fn surface_check_state_label(state: SurfaceCheckState) -> &'static str {
    match state {
        SurfaceCheckState::Running => "running",
        SurfaceCheckState::Stopped => "stopped",
        SurfaceCheckState::Healthy => "healthy",
        SurfaceCheckState::PidfileOk => "pidfile-ok",
        SurfaceCheckState::PidfileMissing => "pidfile-missing",
        SurfaceCheckState::PidfileStale => "pidfile-stale",
        SurfaceCheckState::PidfileInvalid => "pidfile-invalid",
        SurfaceCheckState::PidfileUnreadable => "pidfile-unreadable",
    }
}

fn surface_wait_command(args: &[String]) -> Option<ExitCode> {
    let [command, target, state, rest @ ..] = args else {
        return None;
    };
    if command != "wait" {
        return None;
    }
    let Some(target) = surface_target(target) else {
        surface_wait_usage();
        return Some(ExitCode::from(2));
    };
    let Some(state) = surface_check_state(state) else {
        surface_wait_usage();
        return Some(ExitCode::from(2));
    };
    let Some(options) = surface_wait_options(rest) else {
        surface_wait_usage();
        return Some(ExitCode::from(2));
    };
    Some(wait_for_surface_state(target, state, options))
}

fn surface_wait_usage() {
    eprintln!(
        "usage: hbctl surfaces wait <quick|bar|osd|launcher|all> <running|stopped|healthy|pidfile-ok|pidfile-missing|pidfile-stale|pidfile-invalid|pidfile-unreadable> [--timeout-ms N] [--interval-ms N] [--quiet] [--json]"
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SurfaceWaitOptions {
    timeout: Duration,
    interval: Duration,
    quiet: bool,
    json: bool,
}

impl Default for SurfaceWaitOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(3000),
            interval: Duration::from_millis(50),
            quiet: false,
            json: false,
        }
    }
}

fn surface_wait_options(args: &[String]) -> Option<SurfaceWaitOptions> {
    let mut options = SurfaceWaitOptions::default();
    let mut seen_timeout = false;
    let mut seen_interval = false;
    let mut seen_quiet = false;
    let mut seen_json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--quiet" => {
                if seen_quiet {
                    return None;
                }
                seen_quiet = true;
                options.quiet = true;
                index += 1;
            }
            "--json" => {
                if seen_json {
                    return None;
                }
                seen_json = true;
                options.json = true;
                index += 1;
            }
            "--timeout-ms" => {
                if seen_timeout {
                    return None;
                }
                seen_timeout = true;
                let value = args.get(index + 1)?.parse::<u64>().ok()?;
                options.timeout = Duration::from_millis(value);
                index += 2;
            }
            "--interval-ms" => {
                if seen_interval {
                    return None;
                }
                seen_interval = true;
                let value = args.get(index + 1)?.parse::<u64>().ok()?;
                if value == 0 {
                    return None;
                }
                options.interval = Duration::from_millis(value);
                index += 2;
            }
            _ => return None,
        }
    }
    Some(options)
}

fn wait_for_surface_state(
    target: SurfaceTarget,
    expected: SurfaceCheckState,
    options: SurfaceWaitOptions,
) -> ExitCode {
    let started = std::time::Instant::now();
    loop {
        let statuses = surface_target_statuses(target);
        let passed = surface_statuses_match_check(&statuses, expected);
        if passed {
            if options.json {
                return print_surface_wait_report(
                    target,
                    expected,
                    true,
                    started.elapsed(),
                    statuses,
                );
            }
            if !options.quiet {
                println!("surface state reached");
            }
            return ExitCode::SUCCESS;
        }
        if started.elapsed() >= options.timeout {
            if options.json {
                return print_surface_wait_report(
                    target,
                    expected,
                    false,
                    started.elapsed(),
                    statuses,
                );
            }
            eprintln!("timed out waiting for surface state");
            return ExitCode::from(1);
        }
        let remaining = options.timeout.saturating_sub(started.elapsed());
        thread::sleep(options.interval.min(remaining));
    }
}

#[derive(serde::Serialize)]
struct SurfaceWaitReport {
    target: &'static str,
    expected: &'static str,
    passed: bool,
    elapsed_ms: u128,
    surfaces: Vec<SurfaceStatus>,
}

fn print_surface_wait_report(
    target: SurfaceTarget,
    expected: SurfaceCheckState,
    passed: bool,
    elapsed: Duration,
    statuses: Vec<SurfaceStatus>,
) -> ExitCode {
    let report = SurfaceWaitReport {
        target: surface_target_label(target),
        expected: surface_check_state_label(expected),
        passed,
        elapsed_ms: elapsed.as_millis(),
        surfaces: statuses,
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            if passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(err) => {
            eprintln!("failed to encode surface wait: {err}");
            ExitCode::from(1)
        }
    }
}

fn surface_control_command(args: &[String]) -> Option<ExitCode> {
    let Some(action_arg) = args.first() else {
        return None;
    };
    let action = surface_action(action_arg)?;
    let [_, target_arg, rest @ ..] = args else {
        surface_control_usage();
        return Some(ExitCode::from(2));
    };
    let Some(target) = surface_target(target_arg) else {
        surface_control_usage();
        return Some(ExitCode::from(2));
    };
    let Some(dry_run) = surface_control_dry_run(action, rest) else {
        surface_control_usage();
        return Some(ExitCode::from(2));
    };
    Some(control_surfaces(action, target, dry_run))
}

fn surface_control_dry_run(action: SurfaceAction, args: &[String]) -> Option<bool> {
    match args {
        [] => Some(false),
        [flag] if action == SurfaceAction::Clean && flag == "--dry-run" => Some(true),
        _ => None,
    }
}

fn surface_control_usage() {
    eprintln!("usage: hbctl surfaces <start|stop|restart> <quick|bar|osd|launcher|all>");
    eprintln!("       hbctl surfaces clean <quick|bar|osd|launcher|all> [--dry-run]");
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SurfaceAction {
    Start,
    Stop,
    Restart,
    Clean,
}

pub(crate) fn restart_all() -> ExitCode {
    control_surfaces(SurfaceAction::Restart, SurfaceTarget::All, false)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SurfaceTarget {
    Quick,
    Bar,
    Osd,
    Launcher,
    All,
}

fn surface_action(action: &str) -> Option<SurfaceAction> {
    match action {
        "start" => Some(SurfaceAction::Start),
        "stop" => Some(SurfaceAction::Stop),
        "restart" => Some(SurfaceAction::Restart),
        "clean" => Some(SurfaceAction::Clean),
        _ => None,
    }
}

fn surface_target(target: &str) -> Option<SurfaceTarget> {
    match target {
        "quick" => Some(SurfaceTarget::Quick),
        "bar" => Some(SurfaceTarget::Bar),
        "osd" => Some(SurfaceTarget::Osd),
        "launcher" => Some(SurfaceTarget::Launcher),
        "all" => Some(SurfaceTarget::All),
        _ => None,
    }
}

fn control_surfaces(action: SurfaceAction, target: SurfaceTarget, dry_run: bool) -> ExitCode {
    let targets = match target {
        SurfaceTarget::Quick => vec![SurfaceTarget::Quick],
        SurfaceTarget::Bar => vec![SurfaceTarget::Bar],
        SurfaceTarget::Osd => vec![SurfaceTarget::Osd],
        SurfaceTarget::Launcher => vec![SurfaceTarget::Launcher],
        SurfaceTarget::All => vec![
            SurfaceTarget::Quick,
            SurfaceTarget::Bar,
            SurfaceTarget::Osd,
            SurfaceTarget::Launcher,
        ],
    };
    let mut code = ExitCode::SUCCESS;
    for target in targets {
        let exit = control_surface(action, target, dry_run);
        if !exit_success(exit) {
            code = exit;
        }
    }
    code
}

fn control_surface(action: SurfaceAction, target: SurfaceTarget, dry_run: bool) -> ExitCode {
    match (action, target) {
        (SurfaceAction::Start, SurfaceTarget::Quick) => launch_quick(&[]),
        (SurfaceAction::Start, SurfaceTarget::Bar) => launch_layer_spike(&[]),
        (SurfaceAction::Start, SurfaceTarget::Osd) => launch_osd(&[]),
        (SurfaceAction::Start, SurfaceTarget::Launcher) => {
            launch_launcher(&["--layer".to_string()])
        }
        (SurfaceAction::Stop, SurfaceTarget::Quick) => stop_quick_settings(),
        (SurfaceAction::Stop, SurfaceTarget::Bar) => stop_surface_bar(),
        (SurfaceAction::Stop, SurfaceTarget::Osd) => stop_surface_osd(),
        (SurfaceAction::Stop, SurfaceTarget::Launcher) => stop_surface_launcher(),
        (SurfaceAction::Restart, SurfaceTarget::Quick) => launch_quick(&["--restart".to_string()]),
        (SurfaceAction::Restart, SurfaceTarget::Bar) => {
            launch_layer_spike(&["--restart".to_string()])
        }
        (SurfaceAction::Restart, SurfaceTarget::Osd) => launch_osd(&["--restart".to_string()]),
        (SurfaceAction::Restart, SurfaceTarget::Launcher) => {
            launch_launcher(&["--layer".to_string(), "--restart".to_string()])
        }
        (SurfaceAction::Clean, SurfaceTarget::Quick) => clean_surface_pidfile("quick", dry_run),
        (SurfaceAction::Clean, SurfaceTarget::Bar) => clean_surface_pidfile("bar", dry_run),
        (SurfaceAction::Clean, SurfaceTarget::Osd) => clean_surface_pidfile("osd", dry_run),
        (SurfaceAction::Clean, SurfaceTarget::Launcher) => {
            clean_surface_pidfile("launcher", dry_run)
        }
        (_, SurfaceTarget::All) => unreachable!("expanded before dispatch"),
    }
}

fn clean_surface_pidfile(name: &str, dry_run: bool) -> ExitCode {
    let Some(path) = surface_pid_path(name) else {
        eprintln!("unknown surface: {name}");
        return ExitCode::from(2);
    };
    let value = match std::fs::read_to_string(&path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!("{name} pidfile already missing");
            return ExitCode::SUCCESS;
        }
        Err(err) => {
            eprintln!("failed to read {name} pidfile {}: {err}", path.display());
            return ExitCode::from(1);
        }
    };
    if parse_quick_identity(&value).is_none() {
        eprintln!("{name} pidfile is invalid; not removing automatically");
        return ExitCode::from(1);
    }
    if !cleanable_stale_pidfile(name, &value) {
        println!("{name} pidfile is live; not removing");
        return ExitCode::SUCCESS;
    }
    if dry_run {
        println!("would remove stale {name} pidfile {}", path.display());
        return ExitCode::SUCCESS;
    }
    let current = match std::fs::read_to_string(&path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!("{name} pidfile already missing");
            return ExitCode::SUCCESS;
        }
        Err(err) => {
            eprintln!("failed to re-read {name} pidfile {}: {err}", path.display());
            return ExitCode::from(1);
        }
    };
    if current != value {
        eprintln!("{name} pidfile changed during cleanup; not removing");
        return ExitCode::from(1);
    }
    match std::fs::remove_file(&path) {
        Ok(()) => {
            println!("removed stale {name} pidfile {}", path.display());
            ExitCode::SUCCESS
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!("{name} pidfile already missing");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!(
                "failed to remove stale {name} pidfile {}: {err}",
                path.display()
            );
            ExitCode::from(1)
        }
    }
}

fn cleanable_stale_pidfile(name: &str, value: &str) -> bool {
    let Some(identity) = parse_quick_identity(value) else {
        return false;
    };
    !match name {
        "quick" => quick_identity_alive(&identity),
        "bar" => bar_identity_alive(&identity),
        "osd" => osd_identity_alive(&identity),
        "launcher" => launcher_layer_identity_alive(&identity),
        _ => false,
    }
}

fn stop_quick_settings() -> ExitCode {
    if let Some((pid, identity)) = quick_settings_instance() {
        dismiss_quick_settings(pid, &identity)
    } else {
        println!("quick settings already closed");
        ExitCode::SUCCESS
    }
}

fn stop_surface_bar() -> ExitCode {
    if let Some((pid, identity)) = bar_instance() {
        stop_bar(pid, &identity)
    } else {
        println!("bar already closed");
        ExitCode::SUCCESS
    }
}

fn stop_surface_osd() -> ExitCode {
    if let Some((pid, identity)) = osd_instance() {
        stop_osd(pid, &identity)
    } else {
        println!("osd already closed");
        ExitCode::SUCCESS
    }
}

fn stop_surface_launcher() -> ExitCode {
    if let Some((pid, identity)) = launcher_layer_instance() {
        stop_launcher_layer(pid, &identity)
    } else {
        println!("launcher already closed");
        ExitCode::SUCCESS
    }
}

struct SurfaceCommandOptions {
    name: Option<&'static str>,
    plain: bool,
}

fn surface_command_options(args: &[String]) -> Option<SurfaceCommandOptions> {
    let mut name = None;
    let mut plain = false;
    for arg in args {
        match arg.as_str() {
            "--plain" => plain = true,
            "quick" | "bar" | "osd" | "launcher" if name.is_none() => {
                name = Some(surface_name(arg)?)
            }
            _ => return None,
        }
    }
    Some(SurfaceCommandOptions { name, plain })
}

fn surface_name(name: &str) -> Option<&'static str> {
    match name {
        "quick" => Some("quick"),
        "bar" => Some("bar"),
        "osd" => Some("osd"),
        "launcher" => Some("launcher"),
        _ => None,
    }
}

fn print_surface_statuses_plain(statuses: &[SurfaceStatus]) {
    println!("surface  state    pid       pidfile  log      log_path");
    for status in statuses {
        println!(
            "{:<8} {:<8} {:<9} {:<8} {:<8} {}",
            status.name,
            if status.running { "running" } else { "stopped" },
            status
                .pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "-".to_string()),
            status.pid_file_state,
            status.log_state,
            status.log_path.as_deref().unwrap_or("-")
        );
    }
}

#[derive(serde::Serialize)]
struct SurfaceStatus {
    name: &'static str,
    running: bool,
    pid: Option<u32>,
    identity: Option<String>,
    pid_file: Option<String>,
    pid_file_exists: bool,
    stale_pid_file: bool,
    invalid_pid_file: bool,
    unreadable_pid_file: bool,
    pid_file_state: &'static str,
    log_path: Option<String>,
    log_exists: bool,
    log_readable: bool,
    log_size_bytes: Option<u64>,
    log_state: &'static str,
}

fn surface_statuses() -> Vec<SurfaceStatus> {
    vec![
        surface_status("quick", quick_settings_instance_readonly()),
        surface_status("bar", bar_instance_readonly()),
        surface_status("osd", osd_instance_readonly()),
        surface_status("launcher", launcher_layer_instance()),
    ]
}

fn surface_target_statuses(target: SurfaceTarget) -> Vec<SurfaceStatus> {
    let mut statuses = surface_statuses();
    let Some(name) = surface_target_name(target) else {
        return statuses;
    };
    statuses.retain(|status| status.name == name);
    statuses
}

fn surface_target_name(target: SurfaceTarget) -> Option<&'static str> {
    match target {
        SurfaceTarget::Quick => Some("quick"),
        SurfaceTarget::Bar => Some("bar"),
        SurfaceTarget::Osd => Some("osd"),
        SurfaceTarget::Launcher => Some("launcher"),
        SurfaceTarget::All => None,
    }
}

fn surface_target_label(target: SurfaceTarget) -> &'static str {
    match target {
        SurfaceTarget::Quick => "quick",
        SurfaceTarget::Bar => "bar",
        SurfaceTarget::Osd => "osd",
        SurfaceTarget::Launcher => "launcher",
        SurfaceTarget::All => "all",
    }
}

fn surface_status_healthy(status: &SurfaceStatus) -> bool {
    if status.running {
        status.pid_file_state == "ok"
    } else {
        status.pid_file_state == "missing"
    }
}

fn surface_status(name: &'static str, instance: Option<(u32, String)>) -> SurfaceStatus {
    let (pid, identity) = instance
        .map(|(pid, identity)| (Some(pid), Some(identity)))
        .unwrap_or((None, None));
    let pid_path = surface_pid_path(name);
    let pidfile_health = surface_pidfile_health(name);
    let log_path = hyprbole_core::runtime::log_path(name).ok();
    let log_health = surface_log_health(log_path.as_deref());
    SurfaceStatus {
        name,
        running: pid.is_some(),
        pid,
        identity,
        pid_file: pid_path.map(|path| path.display().to_string()),
        pid_file_exists: pidfile_health.exists,
        stale_pid_file: pidfile_health.stale,
        invalid_pid_file: pidfile_health.invalid,
        unreadable_pid_file: pidfile_health.unreadable,
        pid_file_state: pidfile_health.state,
        log_path: log_path.map(|path| path.display().to_string()),
        log_exists: log_health.exists,
        log_readable: log_health.readable,
        log_size_bytes: log_health.size_bytes,
        log_state: log_health.state,
    }
}

struct LogHealth {
    exists: bool,
    readable: bool,
    size_bytes: Option<u64>,
    state: &'static str,
}

fn surface_log_health(path: Option<&Path>) -> LogHealth {
    surface_log_health_with_io(
        path,
        |path| std::fs::metadata(path),
        |path| std::fs::File::open(path).map(|_| ()),
    )
}

#[cfg(test)]
fn surface_log_health_with_open(
    path: Option<&Path>,
    open_log: impl Fn(&Path) -> std::io::Result<()>,
) -> LogHealth {
    surface_log_health_with_io(path, |path| std::fs::metadata(path), open_log)
}

fn surface_log_health_with_io(
    path: Option<&Path>,
    metadata: impl Fn(&Path) -> std::io::Result<std::fs::Metadata>,
    open_log: impl Fn(&Path) -> std::io::Result<()>,
) -> LogHealth {
    let Some(path) = path else {
        return LogHealth {
            exists: false,
            readable: false,
            size_bytes: None,
            state: "missing",
        };
    };
    let metadata = match metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return LogHealth {
                exists: false,
                readable: false,
                size_bytes: None,
                state: "missing",
            };
        }
        Err(_) => {
            return LogHealth {
                exists: true,
                readable: false,
                size_bytes: None,
                state: "unreadable",
            };
        }
    };
    if open_log(path).is_err() {
        return LogHealth {
            exists: true,
            readable: false,
            size_bytes: Some(metadata.len()),
            state: "unreadable",
        };
    }
    LogHealth {
        exists: true,
        readable: true,
        size_bytes: Some(metadata.len()),
        state: if metadata.len() == 0 { "empty" } else { "ok" },
    }
}

fn pid_file_state(exists: bool, stale: bool) -> &'static str {
    pid_file_state_from_health(exists, stale, false, false)
}

fn pid_file_state_from_health(
    exists: bool,
    stale: bool,
    invalid: bool,
    unreadable: bool,
) -> &'static str {
    match (exists, unreadable, invalid, stale) {
        (false, _, _, _) => "missing",
        (true, true, _, _) => "unreadable",
        (true, _, true, _) => "invalid",
        (true, _, _, true) => "stale",
        (true, _, _, false) => "ok",
    }
}

struct PidfileHealth {
    exists: bool,
    stale: bool,
    invalid: bool,
    unreadable: bool,
    state: &'static str,
}

fn surface_pidfile_health(name: &str) -> PidfileHealth {
    let Some(path) = surface_pid_path(name) else {
        return PidfileHealth {
            exists: false,
            stale: false,
            invalid: false,
            unreadable: false,
            state: pid_file_state(false, false),
        };
    };
    let value = match std::fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return PidfileHealth {
                exists: false,
                stale: false,
                invalid: false,
                unreadable: false,
                state: pid_file_state(false, false),
            };
        }
        Err(_) => {
            return PidfileHealth {
                exists: true,
                stale: false,
                invalid: false,
                unreadable: true,
                state: pid_file_state_from_health(true, false, false, true),
            };
        }
    };
    let Some(identity) = parse_quick_identity(&value) else {
        return PidfileHealth {
            exists: true,
            stale: false,
            invalid: true,
            unreadable: false,
            state: pid_file_state_from_health(true, false, true, false),
        };
    };
    let stale = !match name {
        "quick" => quick_identity_alive(&identity),
        "bar" => bar_identity_alive(&identity),
        "osd" => osd_identity_alive(&identity),
        "launcher" => launcher_layer_identity_alive(&identity),
        _ => true,
    };
    PidfileHealth {
        exists: true,
        stale,
        invalid: false,
        unreadable: false,
        state: pid_file_state(true, stale),
    }
}

fn surface_pid_path(name: &str) -> Option<PathBuf> {
    let file = match name {
        "quick" => "quick.pid",
        "bar" => "bar.pid",
        "osd" => "osd.pid",
        "launcher" => "launcher.pid",
        _ => return None,
    };
    hyprbole_core::runtime::runtime_dir()
        .ok()
        .map(|dir| dir.join(file))
}

fn quick_settings_instance_readonly() -> Option<(u32, String)> {
    let pidfile_identity = hyprbole_core::runtime::runtime_dir()
        .ok()
        .map(|dir| dir.join("quick.pid"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|value| parse_quick_identity(&value));
    if let Some(identity) = pidfile_identity.filter(quick_identity_alive) {
        return Some(identity);
    }
    running_quick_instance()
}

fn bar_instance_readonly() -> Option<(u32, String)> {
    let pidfile_identity = hyprbole_core::runtime::runtime_dir()
        .ok()
        .map(|dir| dir.join("bar.pid"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|value| parse_quick_identity(&value));
    if let Some(identity) = pidfile_identity.filter(bar_identity_alive) {
        return Some(identity);
    }
    running_bar_instance()
}

fn osd_instance_readonly() -> Option<(u32, String)> {
    let pidfile_identity = hyprbole_core::runtime::runtime_dir()
        .ok()
        .map(|dir| dir.join("osd.pid"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|value| parse_quick_identity(&value));
    if let Some(identity) = pidfile_identity.filter(osd_identity_alive) {
        return Some(identity);
    }
    running_osd_instance()
}

#[cfg(test)]
mod tests {
    use super::doctor::*;
    use super::*;
    use std::path::Path;
    use std::process::ExitCode;
    use std::time::Duration;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn surface_status_marks_running_instance() {
        let status = surface_status("quick", Some((42, "42:123".to_string())));
        assert_eq!(status.name, "quick");
        assert!(status.running);
        assert_eq!(status.pid, Some(42));
        assert_eq!(status.identity.as_deref(), Some("42:123"));
    }

    #[test]
    fn pid_file_state_classifies_health() {
        assert_eq!(pid_file_state(false, false), "missing");
        assert_eq!(pid_file_state(false, true), "missing");
        assert_eq!(pid_file_state(true, false), "ok");
        assert_eq!(pid_file_state(true, true), "stale");
        assert_eq!(
            pid_file_state_from_health(true, false, true, false),
            "invalid"
        );
        assert_eq!(
            pid_file_state_from_health(true, false, false, true),
            "unreadable"
        );
    }

    #[test]
    fn surface_status_health_requires_matching_pidfile_state() {
        let running_ok = test_surface_status("bar", true, "ok");
        let running_missing = test_surface_status("bar", true, "missing");
        let stopped_missing = test_surface_status("bar", false, "missing");
        let stopped_stale = test_surface_status("bar", false, "stale");
        assert!(surface_status_healthy(&running_ok));
        assert!(!surface_status_healthy(&running_missing));
        assert!(surface_status_healthy(&stopped_missing));
        assert!(!surface_status_healthy(&stopped_stale));
    }

    #[test]
    fn surface_doctor_options_parse_target_and_json() {
        assert_eq!(
            surface_doctor_options(&[]),
            Some(SurfaceDoctorOptions::default())
        );
        assert_eq!(
            surface_doctor_options(&args(&["bar", "--json"])),
            Some(SurfaceDoctorOptions {
                target: SurfaceTarget::Bar,
                json: true,
                commands: false,
                unhealthy_only: false,
                actionable_only: false,
                zero_ok: false,
                warnings_only: false,
                errors_only: false,
            })
        );
        assert_eq!(
            surface_doctor_options(&args(&[
                "--json",
                "--unhealthy-only",
                "--warnings-only",
                "quick"
            ])),
            Some(SurfaceDoctorOptions {
                target: SurfaceTarget::Quick,
                json: true,
                commands: false,
                unhealthy_only: true,
                actionable_only: false,
                zero_ok: false,
                warnings_only: true,
                errors_only: false,
            })
        );
        assert_eq!(
            surface_doctor_options(&args(&["--actionable-only", "osd"])),
            Some(SurfaceDoctorOptions {
                target: SurfaceTarget::Osd,
                json: false,
                commands: false,
                unhealthy_only: false,
                actionable_only: true,
                zero_ok: false,
                warnings_only: false,
                errors_only: false,
            })
        );
        assert_eq!(
            surface_doctor_options(&args(&["--zero-ok", "bar"])),
            Some(SurfaceDoctorOptions {
                target: SurfaceTarget::Bar,
                json: false,
                commands: false,
                unhealthy_only: false,
                actionable_only: false,
                zero_ok: true,
                warnings_only: false,
                errors_only: false,
            })
        );
        assert!(surface_doctor_options(&args(&["bar", "quick"])).is_none());
        assert!(surface_doctor_options(&args(&["--json", "--json"])).is_none());
        assert!(surface_doctor_options(&args(&["--commands", "--commands"])).is_none());
        assert!(surface_doctor_options(&args(&["--json", "--commands"])).is_none());
        assert!(surface_doctor_options(&args(&["--unhealthy-only", "--unhealthy-only"])).is_none());
        assert!(
            surface_doctor_options(&args(&["--actionable-only", "--actionable-only"])).is_none()
        );
        assert!(surface_doctor_options(&args(&["--zero-ok", "--zero-ok"])).is_none());
        assert!(surface_doctor_options(&args(&["--warnings-only", "--warnings-only"])).is_none());
        assert!(surface_doctor_options(&args(&["--errors-only", "--errors-only"])).is_none());
        assert!(surface_doctor_options(&args(&["--warnings-only", "--errors-only"])).is_none());
        assert!(surface_doctor_options(&args(&["--errors-only", "--warnings-only"])).is_none());
        assert!(surface_doctor_options(&args(&["daemon"])).is_none());
    }

    #[test]
    fn surface_doctor_classifies_issues_and_recommendations() {
        let healthy = test_surface_status("bar", true, "ok");
        let stale = test_surface_status("quick", false, "stale");
        let invalid = test_surface_status("quick", false, "invalid");
        let unreadable = test_surface_status("quick", false, "unreadable");
        let missing = test_surface_status("bar", true, "missing");
        let missing_log = test_surface_status_with_log("bar", true, "ok", "missing");
        let unreadable_log = test_surface_status_with_log("bar", true, "ok", "unreadable");
        let empty_log = test_surface_status_with_log("bar", true, "ok", "empty");
        let stopped_unreadable_log =
            test_surface_status_with_log("quick", false, "missing", "unreadable");
        assert_eq!(surface_doctor_issue(&healthy), "none");
        assert_eq!(surface_doctor_issue(&stale), "stale-pidfile");
        assert_eq!(surface_doctor_issue(&invalid), "invalid-pidfile");
        assert_eq!(surface_doctor_issue(&unreadable), "unreadable-pidfile");
        assert_eq!(surface_doctor_issue(&missing), "missing-pidfile");
        assert_eq!(surface_doctor_log_issue(&missing_log), "missing-log");
        assert_eq!(surface_doctor_log_issue(&unreadable_log), "unreadable-log");
        assert_eq!(surface_doctor_log_issue(&empty_log), "empty-log");
        assert_eq!(
            surface_doctor_log_issue(&stopped_unreadable_log),
            "unreadable-log"
        );
        assert_eq!(surface_doctor_entry_severity("none", "none"), "ok");
        assert_eq!(
            surface_doctor_entry_severity("none", "missing-log"),
            "warning"
        );
        assert_eq!(
            surface_doctor_entry_severity("none", "unreadable-log"),
            "error"
        );
        assert_eq!(surface_doctor_severity("none"), "ok");
        assert_eq!(surface_doctor_severity("stale-pidfile"), "warning");
        assert_eq!(surface_doctor_severity("missing-pidfile"), "warning");
        assert_eq!(surface_doctor_severity("missing-log"), "warning");
        assert_eq!(surface_doctor_severity("empty-log"), "warning");
        assert_eq!(surface_doctor_severity("invalid-pidfile"), "error");
        assert_eq!(surface_doctor_severity("unreadable-log"), "error");
        assert_eq!(
            surface_doctor_recommendation(&stale),
            "run `hbctl surfaces clean quick`"
        );
        assert_eq!(
            surface_doctor_recommended_command(&stale).as_deref(),
            Some("hbctl surfaces clean quick")
        );
        assert_eq!(
            surface_doctor_recommended_command(&missing).as_deref(),
            Some("hbctl surfaces restart bar")
        );
        assert!(surface_doctor_recommended_command(&invalid).is_none());
        assert_eq!(
            surface_doctor_recommended_command(&missing_log).as_deref(),
            Some("hbctl surfaces restart bar")
        );
        assert!(surface_doctor_recommendation(&invalid).contains("remove it manually"));
        assert!(surface_doctor_recommendation(&unreadable).contains("check permissions"));
        assert!(surface_doctor_recommendation(&missing).contains("restart bar"));
        assert!(surface_doctor_recommendation(&missing_log).contains("restart bar"));
        assert!(surface_doctor_recommendation(&unreadable_log).contains("check permissions"));
    }

    #[test]
    fn surface_log_health_classifies_paths() {
        let missing_path = temp_test_path("hyprbole-missing-test-log");
        let missing = surface_log_health(Some(&missing_path));
        assert!(!missing.exists);
        assert!(!missing.readable);
        assert_eq!(missing.size_bytes, None);
        assert_eq!(missing.state, "missing");

        let temp_log = temp_test_path("hyprbole-empty-test-log");
        std::fs::write(&temp_log, "").unwrap();
        let empty = surface_log_health(Some(&temp_log));
        assert!(empty.exists);
        assert!(empty.readable);
        assert_eq!(empty.size_bytes, Some(0));
        assert_eq!(empty.state, "empty");

        std::fs::write(&temp_log, "surface output\n").unwrap();
        let ok = surface_log_health(Some(&temp_log));
        assert!(ok.exists);
        assert!(ok.readable);
        assert_eq!(ok.size_bytes, Some(15));
        assert_eq!(ok.state, "ok");

        std::fs::remove_file(&temp_log).unwrap();
    }

    #[test]
    fn surface_log_health_marks_open_failures_unreadable() {
        let temp_log = temp_test_path("hyprbole-unreadable-test-log");
        std::fs::write(&temp_log, "surface output\n").unwrap();

        let unreadable = surface_log_health_with_open(Some(&temp_log), |_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "blocked by test",
            ))
        });

        std::fs::remove_file(&temp_log).unwrap();

        assert!(unreadable.exists);
        assert!(!unreadable.readable);
        assert_eq!(unreadable.size_bytes, Some(15));
        assert_eq!(unreadable.state, "unreadable");
    }

    #[test]
    fn surface_log_health_marks_metadata_failures_unreadable() {
        let unreadable = surface_log_health_with_io(
            Some(Path::new("/tmp/hyprbole-blocked-metadata-test-log")),
            |_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "blocked by test",
                ))
            },
            |_| Ok(()),
        );

        assert!(unreadable.exists);
        assert!(!unreadable.readable);
        assert_eq!(unreadable.size_bytes, None);
        assert_eq!(unreadable.state, "unreadable");
    }

    #[test]
    fn surface_doctor_entry_includes_metadata() {
        let entry = surface_doctor_entry(test_surface_status("quick", false, "stale"));
        assert!(!entry.healthy);
        assert_eq!(entry.severity, "warning");
        assert_eq!(entry.issue, "stale-pidfile");
        assert_eq!(entry.log_issue, "none");
        assert_eq!(
            entry.recommended_command.as_deref(),
            Some("hbctl surfaces clean quick")
        );
        let log_entry =
            surface_doctor_entry(test_surface_status_with_log("bar", true, "ok", "missing"));
        assert!(!log_entry.healthy);
        assert_eq!(log_entry.issue, "none");
        assert_eq!(log_entry.log_issue, "missing-log");
        assert_eq!(log_entry.severity, "warning");
    }

    #[test]
    fn surface_doctor_report_filters_unhealthy_entries_after_counting_issues() {
        let report = surface_doctor_report_from_statuses(
            SurfaceDoctorOptions {
                target: SurfaceTarget::All,
                json: true,
                commands: false,
                unhealthy_only: true,
                actionable_only: false,
                zero_ok: false,
                warnings_only: false,
                errors_only: false,
            },
            vec![
                test_surface_status("quick", false, "missing"),
                test_surface_status("bar", true, "ok"),
                test_surface_status_with_log("osd", true, "ok", "empty"),
            ],
        );

        assert!(!report.healthy);
        assert_eq!(report.issues, 1);
        assert_eq!(report.warnings, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.actionable, 0);
        assert_eq!(report.displayed, 1);
        assert!(report.filtered);
        assert_eq!(report.severity_filter, "all");
        assert_eq!(report.surfaces.len(), 1);
        assert_eq!(report.surfaces[0].name, "osd");
        assert_eq!(report.surfaces[0].log_issue, "empty-log");
    }

    #[test]
    fn surface_doctor_report_filters_warnings_after_counting_all_issues() {
        let report = surface_doctor_report_from_statuses(
            SurfaceDoctorOptions {
                target: SurfaceTarget::All,
                json: true,
                commands: false,
                unhealthy_only: false,
                actionable_only: false,
                zero_ok: false,
                warnings_only: true,
                errors_only: false,
            },
            vec![
                test_surface_status("quick", false, "stale"),
                test_surface_status("bar", true, "invalid"),
                test_surface_status_with_log("osd", true, "ok", "empty"),
            ],
        );

        assert!(!report.healthy);
        assert_eq!(report.issues, 3);
        assert_eq!(report.warnings, 2);
        assert_eq!(report.errors, 1);
        assert_eq!(report.actionable, 1);
        assert_eq!(report.displayed, 2);
        assert!(report.filtered);
        assert_eq!(report.severity_filter, "warning");
        assert_eq!(report.surfaces.len(), 2);
        assert_eq!(
            report.recommended_commands,
            vec!["hbctl surfaces clean quick".to_string()]
        );
        assert_eq!(report.output_mode, "json");
        assert_eq!(report.selected, 3);
        assert_eq!(report.healthy_count, 0);
        assert_eq!(report.active_filters, vec!["warning"]);
        assert_eq!(report.surface_names, vec!["quick", "osd"]);
        assert_eq!(report.issue_labels, vec!["stale-pidfile", "empty-log"]);
        assert_eq!(report.command_count, 1);
        assert!(report.has_commands);
        assert_eq!(report.surfaces[0].name, "quick");
        assert_eq!(report.surfaces[0].severity, "warning");
        assert_eq!(report.surfaces[1].name, "osd");
        assert_eq!(report.surfaces[1].log_issue, "empty-log");
    }

    #[test]
    fn surface_doctor_report_filters_errors_after_counting_all_issues() {
        let report = surface_doctor_report_from_statuses(
            SurfaceDoctorOptions {
                target: SurfaceTarget::All,
                json: true,
                commands: false,
                unhealthy_only: false,
                actionable_only: false,
                zero_ok: false,
                warnings_only: false,
                errors_only: true,
            },
            vec![
                test_surface_status("quick", false, "stale"),
                test_surface_status("bar", true, "invalid"),
                test_surface_status_with_log("osd", true, "ok", "empty"),
            ],
        );

        assert!(!report.healthy);
        assert_eq!(report.issues, 3);
        assert_eq!(report.warnings, 2);
        assert_eq!(report.errors, 1);
        assert_eq!(report.actionable, 1);
        assert_eq!(report.displayed, 1);
        assert!(report.filtered);
        assert_eq!(report.severity_filter, "error");
        assert_eq!(report.surfaces.len(), 1);
        assert_eq!(report.surfaces[0].name, "bar");
        assert_eq!(report.surfaces[0].severity, "error");
        assert_eq!(report.surfaces[0].issue, "invalid-pidfile");
    }

    #[test]
    fn surface_doctor_report_combines_unhealthy_and_severity_filters() {
        let report = surface_doctor_report_from_statuses(
            SurfaceDoctorOptions {
                target: SurfaceTarget::All,
                json: true,
                commands: false,
                unhealthy_only: true,
                actionable_only: false,
                zero_ok: false,
                warnings_only: true,
                errors_only: false,
            },
            vec![
                test_surface_status("quick", false, "missing"),
                test_surface_status("bar", true, "invalid"),
                test_surface_status_with_log("osd", true, "ok", "empty"),
            ],
        );

        assert!(!report.healthy);
        assert_eq!(report.issues, 2);
        assert_eq!(report.warnings, 1);
        assert_eq!(report.errors, 1);
        assert_eq!(report.actionable, 0);
        assert_eq!(report.displayed, 1);
        assert!(report.filtered);
        assert_eq!(report.severity_filter, "warning");
        assert_eq!(report.surfaces.len(), 1);
        assert_eq!(report.surfaces[0].name, "osd");
    }

    #[test]
    fn surface_doctor_summary_formats_counts_and_filters() {
        let report = surface_doctor_report_from_statuses(
            SurfaceDoctorOptions {
                target: SurfaceTarget::Quick,
                json: false,
                commands: false,
                unhealthy_only: false,
                actionable_only: false,
                zero_ok: false,
                warnings_only: true,
                errors_only: false,
            },
            vec![
                test_surface_status("quick", false, "stale"),
                test_surface_status("bar", true, "invalid"),
            ],
        );

        assert_eq!(
            surface_doctor_summary(&report),
            "summary target=quick mode=plain healthy=false selected=2 healthy_count=0 issues=2 warnings=1 errors=1 actionable=1 displayed=1 commands=1 filtered=true zero_ok=false severity_filter=warning filters=warning"
        );
    }

    #[test]
    fn surface_doctor_report_filters_actionable_entries_after_counting_all_issues() {
        let report = surface_doctor_report_from_statuses(
            SurfaceDoctorOptions {
                target: SurfaceTarget::All,
                json: true,
                commands: false,
                unhealthy_only: false,
                actionable_only: true,
                zero_ok: false,
                warnings_only: false,
                errors_only: false,
            },
            vec![
                test_surface_status("quick", false, "stale"),
                test_surface_status("bar", true, "missing"),
                test_surface_status_with_log("osd", true, "ok", "empty"),
            ],
        );

        assert!(!report.healthy);
        assert_eq!(report.issues, 3);
        assert_eq!(report.warnings, 3);
        assert_eq!(report.errors, 0);
        assert_eq!(report.actionable, 2);
        assert_eq!(report.displayed, 2);
        assert!(report.filtered);
        assert_eq!(report.severity_filter, "all");
        assert_eq!(report.surfaces.len(), 2);
        assert_eq!(report.surfaces[0].name, "quick");
        assert_eq!(
            report.surfaces[0].recommended_command.as_deref(),
            Some("hbctl surfaces clean quick")
        );
        assert_eq!(report.surfaces[1].name, "bar");
        assert_eq!(
            report.surfaces[1].recommended_command.as_deref(),
            Some("hbctl surfaces restart bar")
        );
    }

    #[test]
    fn surface_doctor_report_tracks_filtered_empty_display() {
        let report = surface_doctor_report_from_statuses(
            SurfaceDoctorOptions {
                target: SurfaceTarget::All,
                json: false,
                commands: false,
                unhealthy_only: false,
                actionable_only: false,
                zero_ok: false,
                warnings_only: false,
                errors_only: true,
            },
            vec![test_surface_status("quick", false, "stale")],
        );

        assert!(!report.healthy);
        assert_eq!(report.issues, 1);
        assert_eq!(report.warnings, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.actionable, 1);
        assert_eq!(report.displayed, 0);
        assert!(report.filtered);
        assert!(report.surfaces.is_empty());
        assert_eq!(report.severity_filter, "error");
    }

    #[test]
    fn surface_doctor_exit_code_honors_zero_ok() {
        let failing = surface_doctor_report_from_statuses(
            SurfaceDoctorOptions::default(),
            vec![test_surface_status("quick", false, "stale")],
        );
        let zero_ok = surface_doctor_report_from_statuses(
            SurfaceDoctorOptions {
                zero_ok: true,
                ..SurfaceDoctorOptions::default()
            },
            vec![test_surface_status("quick", false, "stale")],
        );

        assert_eq!(surface_doctor_exit_code(&failing), ExitCode::from(1));
        assert_eq!(surface_doctor_exit_code(&zero_ok), ExitCode::SUCCESS);
        assert!(zero_ok.zero_ok);
    }

    #[test]
    fn surface_doctor_report_serializes_json_contract() {
        let report = surface_doctor_report_from_statuses(
            SurfaceDoctorOptions::default(),
            vec![test_surface_status_with_log("bar", true, "ok", "missing")],
        );
        let json = serde_json::to_value(&report).unwrap();

        assert_eq!(json["target"], "all");
        assert_eq!(json["output_mode"], "plain");
        assert_eq!(json["healthy"], false);
        assert_eq!(json["selected"], 1);
        assert_eq!(json["healthy_count"], 0);
        assert_eq!(json["issues"], 1);
        assert_eq!(json["warnings"], 1);
        assert_eq!(json["errors"], 0);
        assert_eq!(json["actionable"], 1);
        assert_eq!(json["displayed"], 1);
        assert_eq!(json["filtered"], false);
        assert_eq!(json["active_filters"], serde_json::json!([]));
        assert_eq!(json["zero_ok"], false);
        assert_eq!(json["severity_filter"], "all");
        assert_eq!(json["surface_names"], serde_json::json!(["bar"]));
        assert_eq!(json["issue_labels"], serde_json::json!(["missing-log"]));
        assert_eq!(json["command_count"], 1);
        assert_eq!(json["has_commands"], true);
        assert_eq!(
            json["recommended_commands"],
            serde_json::json!(["hbctl surfaces restart bar"])
        );
        assert_eq!(json["surfaces"][0]["name"], "bar");
        assert_eq!(json["surfaces"][0]["severity"], "warning");
        assert_eq!(json["surfaces"][0]["issue"], "none");
        assert_eq!(json["surfaces"][0]["log_issue"], "missing-log");
        assert_eq!(
            json["surfaces"][0]["recommended_command"],
            "hbctl surfaces restart bar"
        );
        assert_eq!(json["surfaces"][0]["status"]["log_state"], "missing");
    }

    #[test]
    fn surface_doctor_command_rejects_conflicting_severity_filters() {
        assert_eq!(
            surface_doctor_command(&args(&["doctor", "--warnings-only", "--errors-only"])),
            Some(ExitCode::from(2))
        );
    }

    #[test]
    fn surface_status_matches_check_supports_pidfile_states() {
        let running_ok = test_surface_status("bar", true, "ok");
        let missing = test_surface_status("quick", false, "missing");
        let stale = test_surface_status("quick", false, "stale");
        let invalid = test_surface_status("quick", false, "invalid");
        let unreadable = test_surface_status("quick", false, "unreadable");
        assert!(surface_status_matches_check(
            &running_ok,
            SurfaceCheckState::PidfileOk
        ));
        assert!(surface_status_matches_check(
            &missing,
            SurfaceCheckState::PidfileMissing
        ));
        assert!(surface_status_matches_check(
            &stale,
            SurfaceCheckState::PidfileStale
        ));
        assert!(surface_status_matches_check(
            &invalid,
            SurfaceCheckState::PidfileInvalid
        ));
        assert!(surface_status_matches_check(
            &unreadable,
            SurfaceCheckState::PidfileUnreadable
        ));
        assert!(!surface_status_matches_check(
            &missing,
            SurfaceCheckState::PidfileOk
        ));
    }

    #[test]
    fn cleanable_stale_pidfile_rejects_invalid_identity() {
        assert!(!cleanable_stale_pidfile("quick", "not-a-pid"));
        assert!(!cleanable_stale_pidfile("quick", "123:not-a-start-time"));
    }

    #[test]
    fn surface_pid_path_rejects_unknown_surface() {
        assert!(surface_pid_path("unknown").is_none());
    }

    #[test]
    fn surface_statuses_include_managed_surfaces() {
        let statuses = surface_statuses();
        assert_eq!(statuses.len(), 4);
        assert_eq!(statuses[0].name, "quick");
        assert_eq!(statuses[1].name, "bar");
        assert_eq!(statuses[2].name, "osd");
        assert_eq!(statuses[3].name, "launcher");
    }

    #[test]
    fn surface_command_options_parse_filter_and_plain() {
        let options = surface_command_options(&args(&["bar", "--plain"])).unwrap();
        assert_eq!(options.name, Some("bar"));
        assert!(options.plain);
    }

    #[test]
    fn surface_command_options_reject_duplicate_filter() {
        assert!(surface_command_options(&args(&["bar", "osd"])).is_none());
    }

    #[test]
    fn surface_name_rejects_unknown_value() {
        assert_eq!(surface_name("quick"), Some("quick"));
        assert!(surface_name("daemon").is_none());
    }

    #[test]
    fn surface_action_parses_control_verbs() {
        assert!(matches!(
            surface_action("start"),
            Some(SurfaceAction::Start)
        ));
        assert!(matches!(surface_action("stop"), Some(SurfaceAction::Stop)));
        assert!(matches!(
            surface_action("restart"),
            Some(SurfaceAction::Restart)
        ));
        assert!(matches!(
            surface_action("clean"),
            Some(SurfaceAction::Clean)
        ));
        assert!(surface_action("status").is_none());
    }

    #[test]
    fn surface_target_parses_all_managed_targets() {
        assert!(matches!(
            surface_target("quick"),
            Some(SurfaceTarget::Quick)
        ));
        assert!(matches!(surface_target("bar"), Some(SurfaceTarget::Bar)));
        assert!(matches!(surface_target("osd"), Some(SurfaceTarget::Osd)));
        assert!(matches!(surface_target("all"), Some(SurfaceTarget::All)));
        assert!(surface_target("daemon").is_none());
    }

    #[test]
    fn surface_control_dry_run_only_allows_clean() {
        assert_eq!(
            surface_control_dry_run(SurfaceAction::Clean, &args(&["--dry-run"])),
            Some(true)
        );
        assert_eq!(
            surface_control_dry_run(SurfaceAction::Clean, &[]),
            Some(false)
        );
        assert!(surface_control_dry_run(SurfaceAction::Start, &args(&["--dry-run"])).is_none());
        assert!(surface_control_dry_run(SurfaceAction::Clean, &args(&["--bad"])).is_none());
    }

    #[test]
    fn surface_check_state_parses_expected_values() {
        assert!(matches!(
            surface_check_state("running"),
            Some(SurfaceCheckState::Running)
        ));
        assert!(matches!(
            surface_check_state("stopped"),
            Some(SurfaceCheckState::Stopped)
        ));
        assert!(matches!(
            surface_check_state("healthy"),
            Some(SurfaceCheckState::Healthy)
        ));
        assert!(matches!(
            surface_check_state("pidfile-ok"),
            Some(SurfaceCheckState::PidfileOk)
        ));
        assert!(matches!(
            surface_check_state("pidfile-missing"),
            Some(SurfaceCheckState::PidfileMissing)
        ));
        assert!(matches!(
            surface_check_state("pidfile-stale"),
            Some(SurfaceCheckState::PidfileStale)
        ));
        assert!(matches!(
            surface_check_state("pidfile-invalid"),
            Some(SurfaceCheckState::PidfileInvalid)
        ));
        assert!(matches!(
            surface_check_state("pidfile-unreadable"),
            Some(SurfaceCheckState::PidfileUnreadable)
        ));
        assert!(surface_check_state("clean").is_none());
    }

    #[test]
    fn surface_check_options_allow_quiet_and_json() {
        assert_eq!(
            surface_check_options(&[]),
            Some(SurfaceCheckOptions::default())
        );
        assert_eq!(
            surface_check_options(&args(&["--quiet"])),
            Some(SurfaceCheckOptions {
                quiet: true,
                json: false,
            })
        );
        assert_eq!(
            surface_check_options(&args(&["--json", "--quiet"])),
            Some(SurfaceCheckOptions {
                quiet: true,
                json: true,
            })
        );
        assert!(surface_check_options(&args(&["--quiet", "--quiet"])).is_none());
        assert!(surface_check_options(&args(&["--json", "--json"])).is_none());
        assert!(surface_check_options(&args(&["--timeout-ms", "1"])).is_none());
    }

    #[test]
    fn surface_check_command_handles_incomplete_check_usage() {
        assert!(surface_check_command(&args(&["check"])).is_some());
        assert!(surface_check_command(&args(&["wait"])).is_none());
    }

    #[test]
    fn surface_wait_options_parse_optional_flags() {
        assert_eq!(
            surface_wait_options(&[]),
            Some(SurfaceWaitOptions::default())
        );
        assert_eq!(
            surface_wait_options(&args(&[
                "--quiet",
                "--json",
                "--timeout-ms",
                "42",
                "--interval-ms",
                "7"
            ])),
            Some(SurfaceWaitOptions {
                timeout: Duration::from_millis(42),
                interval: Duration::from_millis(7),
                quiet: true,
                json: true,
            })
        );
        assert!(surface_wait_options(&args(&["--timeout-ms", "bad"])).is_none());
        assert!(surface_wait_options(&args(&["--interval-ms", "bad"])).is_none());
        assert!(surface_wait_options(&args(&["--interval-ms", "0"])).is_none());
        assert!(surface_wait_options(&args(&["--bad", "42"])).is_none());
        assert!(surface_wait_options(&args(&["--quiet", "--quiet"])).is_none());
        assert!(surface_wait_options(&args(&["--json", "--json"])).is_none());
        assert!(surface_wait_options(&args(&["--timeout-ms", "1", "--timeout-ms", "2"])).is_none());
        assert!(
            surface_wait_options(&args(&["--interval-ms", "1", "--interval-ms", "2"])).is_none()
        );
    }

    fn test_surface_status(
        name: &'static str,
        running: bool,
        pid_file_state: &'static str,
    ) -> SurfaceStatus {
        test_surface_status_with_log(name, running, pid_file_state, "ok")
    }

    fn test_surface_status_with_log(
        name: &'static str,
        running: bool,
        pid_file_state: &'static str,
        log_state: &'static str,
    ) -> SurfaceStatus {
        SurfaceStatus {
            name,
            running,
            pid: running.then_some(42),
            identity: running.then(|| "42:123".to_string()),
            pid_file: Some(format!("/tmp/{name}.pid")),
            pid_file_exists: pid_file_state != "missing",
            stale_pid_file: pid_file_state == "stale",
            invalid_pid_file: pid_file_state == "invalid",
            unreadable_pid_file: pid_file_state == "unreadable",
            pid_file_state,
            log_path: Some(format!("/tmp/{name}.log")),
            log_exists: log_state != "missing",
            log_readable: !matches!(log_state, "missing" | "unreadable"),
            log_size_bytes: match log_state {
                "missing" | "unreadable" => None,
                "empty" => Some(0),
                _ => Some(42),
            },
            log_state,
        }
    }

    fn temp_test_path(name: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{}-{nonce}", std::process::id()))
    }
}
