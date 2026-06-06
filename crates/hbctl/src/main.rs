use std::env;
use std::fs::OpenOptions;
use std::io::{Read, Write, stdout};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::{thread, time::Duration};

mod surfaces;

const QUICK_DISMISS_TIMEOUT: Duration = Duration::from_millis(3000);

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();

    match args.first().map(String::as_str) {
        Some("daemon") => launch_daemon(args.iter().any(|arg| arg == "--foreground")),
        Some("ui") => launch_ui(&args[1..]),
        Some("launcher") => launch_launcher(&args[1..]),
        Some("control") => launch_control(&args[1..]),
        Some("quick") => launch_quick(&args[1..]),
        Some("osd") => launch_osd(&args[1..]),
        Some("bar") => launch_layer_spike(&args[1..]),
        Some("layer-spike") => launch_layer_spike(&args[1..]),
        Some("dev") => dev_command(&args[1..]),
        Some("bind-mouse-ipc") => bind_mouse_via_ipc(),
        Some("ping") => send_daemon_request(hyprbole_core::daemon::DaemonRequest::Ping),
        Some("status") => send_shell_request(&hyprbole_core::daemon::ShellRequest::StatusGet),
        Some("state") => send_shell_request(&hyprbole_core::daemon::ShellRequest::StateGet),
        Some("binds") => send_shell_request(&hyprbole_core::daemon::ShellRequest::BindsGet),
        Some("events") => events_command(&args[1..]),
        Some("logs") => logs_command(&args[1..]),
        Some("surfaces") => surfaces::surfaces_command(&args[1..]),
        Some("settings") => settings_command(&args[1..]),
        Some("ui-settings") => ui_settings_command(&args[1..]),
        Some("theme") => theme_command(&args[1..]),
        Some("notifications") => notifications_command(&args[1..]),
        Some("ownership") => ownership_command(&args[1..]),
        Some("reconcile") => reconcile_command(&args[1..]),
        Some("shutdown") => send_daemon_request(hyprbole_core::daemon::DaemonRequest::Shutdown),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_usage();
            ExitCode::SUCCESS
        }
        Some(command) => {
            eprintln!("unknown command: {command}");
            print_usage();
            ExitCode::from(2)
        }
    }
}

fn print_usage() {
    println!(
        r#"Hyprbole control

Usage:
  hbctl daemon [--foreground]
  hbctl ui [--foreground] [--debug-direct-fallback]
  hbctl launcher [--foreground]        # GTK4 layer-shell default
  hbctl launcher --layer [--foreground] [--toggle] [--restart] [--prompt TEXT] [--placeholder TEXT] [--lines N]
  hbctl launcher --layer --stdin --foreground [--prompt TEXT] [--placeholder TEXT] [--lines N]
  hbctl launcher --dev|--gtk [--foreground]
  hbctl control [--foreground]
  hbctl quick [--foreground] [--toggle] [--restart] [--dev]
  hbctl osd [--foreground] [--toggle] [--restart]
  hbctl bar [--foreground] [--toggle] [--restart] [--gtk] [--debug-direct-fallback]
  hbctl layer-spike [--foreground] [--toggle] [--restart] [--debug-direct-fallback]
  hbctl dev restart-all
  hbctl ping
  hbctl status
  hbctl state
  hbctl binds
  hbctl events [--after <id>] [--follow]
  hbctl logs [daemon|ui|control|quick|bar|osd|launcher|list] [--path] [--follow]
  hbctl surfaces [quick|bar|osd|launcher] [--plain]
  hbctl surfaces <start|stop|restart> <quick|bar|osd|launcher|all>
  hbctl surfaces clean <quick|bar|osd|launcher|all> [--dry-run]
  hbctl surfaces doctor [quick|bar|osd|launcher|all] [--json|--commands] [--unhealthy-only] [--actionable-only] [--zero-ok] [--warnings-only|--errors-only]
  hbctl surfaces check <quick|bar|osd|launcher|all> <running|stopped|healthy|pidfile-ok|pidfile-missing|pidfile-stale|pidfile-invalid|pidfile-unreadable> [--quiet] [--json]
  hbctl surfaces wait <quick|bar|osd|launcher|all> <running|stopped|healthy|pidfile-ok|pidfile-missing|pidfile-stale|pidfile-invalid|pidfile-unreadable> [--timeout-ms N] [--interval-ms N] [--quiet] [--json]
  hbctl settings
  hbctl settings set <subsystem> <respect|runtime|persisted>
  hbctl ui-settings [set <bar|osd>.<field> <value>|reset <bar|osd>]
  hbctl theme [mode <system|light|dark>|reset]
  hbctl notifications [dnd on|off|toggle|clear|push <summary> [body]]
  hbctl ownership [capabilities|set <subsystem> <respect|runtime|persisted>]
  hbctl reconcile [subsystem]
  hbctl bind-mouse-ipc
  hbctl shutdown
  hbctl help"#
    );
}

fn dev_command(args: &[String]) -> ExitCode {
    match args {
        [command] if command == "restart-all" => dev_restart_all(),
        _ => {
            eprintln!("usage: hbctl dev restart-all");
            ExitCode::from(2)
        }
    }
}

fn dev_restart_all() -> ExitCode {
    println!("restarting hyprbole daemon");
    if daemon_running() {
        let exit = send_daemon_request(hyprbole_core::daemon::DaemonRequest::Shutdown);
        if !exit_success(exit) {
            return exit;
        }
        if !wait_for_daemon(false, Duration::from_secs(3)) {
            eprintln!("timed out waiting for hyprbole daemon to stop");
            return ExitCode::from(1);
        }
    }

    let exit = launch_daemon(false);
    if !exit_success(exit) {
        return exit;
    }
    if !wait_for_daemon(true, Duration::from_secs(5)) {
        eprintln!("timed out waiting for hyprbole daemon to start");
        return ExitCode::from(1);
    }

    println!("reconciling runtime-owned state");
    let exit = reconcile_command(&[]);
    if !exit_success(exit) {
        return exit;
    }

    println!("restarting managed shell surfaces");
    surfaces::restart_all()
}

fn daemon_running() -> bool {
    hyprbole_core::daemon::DaemonClient::from_env()
        .and_then(|client| client.send_shell(&hyprbole_core::daemon::ShellRequest::Ping))
        .is_ok()
}

fn wait_for_daemon(running: bool, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if daemon_running() == running {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn bind_mouse_via_ipc() -> ExitCode {
    match hyprbole_core::daemon::DaemonClient::from_env().and_then(|client| {
        client.send_shell(&hyprbole_core::daemon::ShellRequest::ActionCall {
            action: hyprbole_core::daemon::ShellAction::Compositor {
                action: hyprbole_core::compositor::CompositorAction::RegisterMouseWindowControls,
            },
        })
    }) {
        Ok(reply) => shell_response_exit_code(reply),
        Err(daemon_err) => {
            eprintln!("daemon unavailable, falling back to direct Hyprland IPC: {daemon_err}");
            if let Err(err) = hyprbole_core::hyprland::register_shell_mouse_binds() {
                eprintln!("failed to register mouse binds: {err}");
                return ExitCode::from(1);
            }
            println!("registered Super mouse drag/resize binds via direct Hyprland IPC");
            ExitCode::SUCCESS
        }
    }
}

fn send_daemon_request(request: hyprbole_core::daemon::DaemonRequest) -> ExitCode {
    let shell_request = match request {
        hyprbole_core::daemon::DaemonRequest::Ping => hyprbole_core::daemon::ShellRequest::Ping,
        hyprbole_core::daemon::DaemonRequest::BindMouse => {
            hyprbole_core::daemon::ShellRequest::ActionCall {
                action: hyprbole_core::daemon::ShellAction::Compositor {
                    action:
                        hyprbole_core::compositor::CompositorAction::RegisterMouseWindowControls,
                },
            }
        }
        hyprbole_core::daemon::DaemonRequest::Shutdown => {
            hyprbole_core::daemon::ShellRequest::Shutdown
        }
    };
    send_shell_request(&shell_request)
}

fn settings_command(args: &[String]) -> ExitCode {
    match args {
        [] => send_shell_request(&hyprbole_core::daemon::ShellRequest::SettingsGet),
        [command, subsystem, mode] if command == "set" => set_ownership(subsystem, mode),
        _ => {
            eprintln!("usage: hbctl settings [set <subsystem> <respect|runtime|persisted>]");
            ExitCode::from(2)
        }
    }
}

fn ownership_command(args: &[String]) -> ExitCode {
    match args {
        [] => send_shell_request(&hyprbole_core::daemon::ShellRequest::SettingsGet),
        [command] if command == "capabilities" => ownership_capabilities(),
        [command, subsystem, mode] if command == "set" => set_ownership(subsystem, mode),
        [subsystem, mode] => set_ownership(subsystem, mode),
        _ => {
            eprintln!("usage: hbctl ownership [set <subsystem> <respect|runtime|persisted>]");
            ExitCode::from(2)
        }
    }
}

fn ui_settings_command(args: &[String]) -> ExitCode {
    match ui_settings_request(args) {
        Ok(request) => send_shell_request(&request),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn ui_settings_request(
    args: &[String],
) -> Result<hyprbole_core::daemon::ShellRequest, &'static str> {
    match args {
        [] => Ok(hyprbole_core::daemon::ShellRequest::UiSettingsGet),
        [command, field, value] if command == "set" && field.starts_with("bar.") => {
            Ok(hyprbole_core::daemon::ShellRequest::BarSettingsSetField {
                field: field.to_string(),
                value: value.to_string(),
            })
        }
        [command, field, value] if command == "set" && field.starts_with("osd.") => {
            Ok(hyprbole_core::daemon::ShellRequest::OsdSettingsSetField {
                field: field.to_string(),
                value: value.to_string(),
            })
        }
        [command, target] if command == "reset" && target == "bar" => {
            Ok(hyprbole_core::daemon::ShellRequest::BarSettingsReset)
        }
        [command, target] if command == "reset" && target == "osd" => {
            Ok(hyprbole_core::daemon::ShellRequest::OsdSettingsReset)
        }
        _ => Err("usage: hbctl ui-settings [set <bar|osd>.<field> <value>|reset <bar|osd>]"),
    }
}

fn notifications_command(args: &[String]) -> ExitCode {
    match notifications_request(args) {
        Ok(request) => send_shell_request(&request),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn theme_command(args: &[String]) -> ExitCode {
    match theme_request(args) {
        Ok(request) => send_shell_request(&request),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn theme_request(args: &[String]) -> Result<hyprbole_core::daemon::ShellRequest, String> {
    let usage = "usage: hbctl theme [mode <system|light|dark>|reset]";
    match args {
        [] => Ok(hyprbole_core::daemon::ShellRequest::ThemeGet),
        [command, mode] if command == "mode" => {
            Ok(hyprbole_core::daemon::ShellRequest::ThemeSetMode {
                mode: mode.parse().map_err(|err: String| err)?,
            })
        }
        [mode] if matches!(mode.as_str(), "system" | "auto" | "light" | "dark") => {
            Ok(hyprbole_core::daemon::ShellRequest::ThemeSetMode {
                mode: mode.parse().map_err(|err: String| err)?,
            })
        }
        [command] if command == "reset" => Ok(hyprbole_core::daemon::ShellRequest::ThemeReset),
        _ => Err(usage.to_string()),
    }
}

fn notifications_request(
    args: &[String],
) -> Result<hyprbole_core::daemon::ShellRequest, &'static str> {
    use hyprbole_core::notifications::{NotificationAction, NotificationUrgency};
    let usage = "usage: hbctl notifications [dnd on|off|toggle|clear|push <summary> [body]]";
    match args {
        [] => Ok(hyprbole_core::daemon::ShellRequest::NotificationsGet),
        [command, value] if command == "dnd" && value == "on" => {
            Ok(notification_action_request(NotificationAction::SetDnd {
                enabled: true,
            }))
        }
        [command, value] if command == "dnd" && value == "off" => {
            Ok(notification_action_request(NotificationAction::SetDnd {
                enabled: false,
            }))
        }
        [command, value] if command == "dnd" && value == "toggle" => {
            Ok(notification_action_request(NotificationAction::ToggleDnd))
        }
        [command] if command == "clear" => Ok(notification_action_request(
            NotificationAction::ClearHistory,
        )),
        [command, summary] if command == "push" => {
            Ok(notification_action_request(NotificationAction::Push {
                summary: summary.to_string(),
                body: String::new(),
                urgency: NotificationUrgency::Normal,
            }))
        }
        [command, summary, body] if command == "push" => {
            Ok(notification_action_request(NotificationAction::Push {
                summary: summary.to_string(),
                body: body.to_string(),
                urgency: NotificationUrgency::Normal,
            }))
        }
        _ => Err(usage),
    }
}

fn notification_action_request(
    action: hyprbole_core::notifications::NotificationAction,
) -> hyprbole_core::daemon::ShellRequest {
    hyprbole_core::daemon::ShellRequest::ActionCall {
        action: hyprbole_core::daemon::ShellAction::Notifications { action },
    }
}

fn ownership_capabilities() -> ExitCode {
    match hyprbole_core::daemon::DaemonClient::from_env()
        .and_then(|client| client.send_shell(&hyprbole_core::daemon::ShellRequest::StatusGet))
    {
        Ok(hyprbole_core::daemon::ShellResponse::Status { daemon }) => {
            print_json(&daemon.ownership_capabilities);
            ExitCode::SUCCESS
        }
        Ok(response) => shell_response_exit_code(response),
        Err(err) => {
            eprintln!("daemon request failed: {err}");
            ExitCode::from(1)
        }
    }
}

fn reconcile_command(args: &[String]) -> ExitCode {
    let subsystem = match args {
        [] => None,
        [subsystem] => match subsystem.parse::<hyprbole_core::settings::ShellSubsystem>() {
            Ok(subsystem) => Some(subsystem),
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(2);
            }
        },
        _ => {
            eprintln!("usage: hbctl reconcile [subsystem]");
            return ExitCode::from(2);
        }
    };
    send_shell_request(&hyprbole_core::daemon::ShellRequest::Reconcile { subsystem })
}

fn set_ownership(subsystem: &str, mode: &str) -> ExitCode {
    let subsystem = match subsystem.parse::<hyprbole_core::settings::ShellSubsystem>() {
        Ok(subsystem) => subsystem,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };
    let mode = match mode.parse::<hyprbole_core::settings::OwnershipMode>() {
        Ok(mode) => mode,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };
    if mode != hyprbole_core::settings::OwnershipMode::RespectUserConfig
        && ownership_mode_supported(subsystem, mode).is_some_and(|supported| !supported)
    {
        eprintln!("{subsystem} {mode} ownership is not supported yet; use respect");
        return ExitCode::from(2);
    }
    send_shell_request(&hyprbole_core::daemon::ShellRequest::SettingsSetOwnership {
        subsystem,
        mode,
    })
}

fn ownership_mode_supported(
    subsystem: hyprbole_core::settings::ShellSubsystem,
    mode: hyprbole_core::settings::OwnershipMode,
) -> Option<bool> {
    let Ok(hyprbole_core::daemon::ShellResponse::Status { daemon }) =
        hyprbole_core::daemon::DaemonClient::from_env()
            .and_then(|client| client.send_shell(&hyprbole_core::daemon::ShellRequest::StatusGet))
    else {
        return None;
    };
    let capability = daemon
        .ownership_capabilities
        .iter()
        .find(|capability| capability.subsystem == subsystem)?;
    let support = match mode {
        hyprbole_core::settings::OwnershipMode::RespectUserConfig => return Some(true),
        hyprbole_core::settings::OwnershipMode::RuntimeOwned => capability.runtime_reconcile,
        hyprbole_core::settings::OwnershipMode::PersistedOwned => capability.persisted_ownership,
    };
    Some(matches!(
        support,
        hyprbole_core::daemon::CapabilitySupport::Implemented
            | hyprbole_core::daemon::CapabilitySupport::Partial
    ))
}

fn events_command(args: &[String]) -> ExitCode {
    let mut after = None;
    let mut follow = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--follow" => {
                follow = true;
                index += 1;
            }
            "--after" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("--after requires an event id");
                    return ExitCode::from(2);
                };
                after = match value.parse::<u64>() {
                    Ok(value) => Some(value),
                    Err(err) => {
                        eprintln!("invalid event id `{value}`: {err}");
                        return ExitCode::from(2);
                    }
                };
                index += 2;
            }
            value => {
                eprintln!("unknown events argument: {value}");
                return ExitCode::from(2);
            }
        }
    }

    if follow {
        follow_events(after)
    } else {
        send_shell_request(&hyprbole_core::daemon::ShellRequest::EventsSubscribe { after })
    }
}

fn follow_events(after: Option<u64>) -> ExitCode {
    let events = match hyprbole_core::daemon::DaemonClient::from_env()
        .and_then(|client| client.follow_events(after))
    {
        Ok(events) => events,
        Err(err) => {
            eprintln!("daemon request failed: {err}");
            return ExitCode::from(1);
        }
    };
    let mut output = stdout();
    for event in events {
        match event {
            Ok(event) => {
                let line = match serde_json::to_string(&event) {
                    Ok(line) => line,
                    Err(err) => {
                        eprintln!("failed to encode event: {err}");
                        return ExitCode::from(1);
                    }
                };
                if let Err(err) = writeln!(output, "{line}").and_then(|()| output.flush()) {
                    eprintln!("failed to write event: {err}");
                    return ExitCode::from(1);
                }
            }
            Err(err) => {
                eprintln!("daemon request failed: read event: {err}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}

fn logs_command(args: &[String]) -> ExitCode {
    let mut name = "daemon";
    let mut path_only = false;
    let mut follow = false;
    let mut list = false;
    for arg in args {
        match arg.as_str() {
            "daemon" | "ui" | "control" | "quick" | "bar" | "osd" | "launcher" => name = arg,
            "--path" => path_only = true,
            "--follow" => follow = true,
            "list" => list = true,
            value => {
                eprintln!("unknown logs argument: {value}");
                eprintln!(
                    "usage: hbctl logs [daemon|ui|control|quick|bar|osd|launcher|list] [--path] [--follow]"
                );
                return ExitCode::from(2);
            }
        }
    }
    if list {
        for name in ["daemon", "ui", "control", "quick", "bar", "osd", "launcher"] {
            match hyprbole_core::runtime::log_path(name) {
                Ok(path) => println!("{name}: {}", path.display()),
                Err(err) => eprintln!("{name}: {err}"),
            }
        }
        return ExitCode::SUCCESS;
    }
    let path = match hyprbole_core::runtime::log_path(name) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("failed to locate log path: {err}");
            return ExitCode::from(1);
        }
    };
    if path_only {
        println!("{}", path.display());
        return ExitCode::SUCCESS;
    }
    if follow {
        return follow_log(&path);
    }
    let mut file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("log file does not exist yet: {}", path.display());
            return ExitCode::from(2);
        }
        Err(err) => {
            eprintln!("failed to open {}: {err}", path.display());
            return ExitCode::from(1);
        }
    };
    let mut contents = String::new();
    if let Err(err) = file.read_to_string(&mut contents) {
        eprintln!("failed to read {}: {err}", path.display());
        return ExitCode::from(1);
    }
    print!("{contents}");
    ExitCode::SUCCESS
}

fn follow_log(path: &Path) -> ExitCode {
    match Command::new("tail")
        .args(["-n", "80", "-F"])
        .arg(path)
        .status()
    {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            eprintln!("tail exited with {status}");
            ExitCode::from(1)
        }
        Err(err) => {
            eprintln!("failed to follow {}: {err}", path.display());
            ExitCode::from(1)
        }
    }
}

fn send_shell_request(request: &hyprbole_core::daemon::ShellRequest) -> ExitCode {
    match hyprbole_core::daemon::DaemonClient::from_env()
        .and_then(|client| client.send_shell(request))
    {
        Ok(reply) => shell_response_exit_code(reply),
        Err(err) => {
            eprintln!("daemon request failed: {err}");
            ExitCode::from(1)
        }
    }
}

fn print_shell_response(response: hyprbole_core::daemon::ShellResponse) {
    match response {
        hyprbole_core::daemon::ShellResponse::Ok { message } => println!("ok {message}"),
        hyprbole_core::daemon::ShellResponse::State { snapshot } => {
            print_json(&snapshot);
        }
        hyprbole_core::daemon::ShellResponse::Settings { settings } => {
            print_json(&settings);
        }
        hyprbole_core::daemon::ShellResponse::UiSettings { settings } => {
            print_json(&settings);
        }
        hyprbole_core::daemon::ShellResponse::Theme { snapshot } => {
            print_json(&snapshot);
        }
        hyprbole_core::daemon::ShellResponse::Binds { keybindings } => {
            print_json(&keybindings);
        }
        hyprbole_core::daemon::ShellResponse::Notifications { snapshot } => {
            print_json(&snapshot);
        }
        hyprbole_core::daemon::ShellResponse::Events { events } => {
            print_json(&events);
        }
        hyprbole_core::daemon::ShellResponse::Status { daemon } => {
            print_json(&daemon);
        }
        hyprbole_core::daemon::ShellResponse::Error { message } => println!("error {message}"),
    }
}

fn print_json(value: &impl serde::Serialize) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(err) => eprintln!("failed to encode JSON: {err}"),
    }
}

fn shell_response_exit_code(response: hyprbole_core::daemon::ShellResponse) -> ExitCode {
    let ok = response.is_ok();
    print_shell_response(response);
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn launch_ui(args: &[String]) -> ExitCode {
    let Some(launch) = ui_launch_options(args) else {
        return ExitCode::from(2);
    };
    launch_ui_with_args(launch.foreground, "ui", &launch.ui_args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn ui_settings_parser_builds_get_request() {
        assert!(matches!(
            ui_settings_request(&[]),
            Ok(hyprbole_core::daemon::ShellRequest::UiSettingsGet)
        ));
    }

    #[test]
    fn ui_settings_parser_builds_set_field_request() {
        let request = ui_settings_request(&args(&["set", "bar.height", "44"])).unwrap();
        assert!(matches!(
            request,
            hyprbole_core::daemon::ShellRequest::BarSettingsSetField { field, value }
                if field == "bar.height" && value == "44"
        ));
    }

    #[test]
    fn ui_settings_parser_builds_osd_set_field_request() {
        let request = ui_settings_request(&args(&["set", "osd.timeout_ms", "2500"])).unwrap();
        assert!(matches!(
            request,
            hyprbole_core::daemon::ShellRequest::OsdSettingsSetField { field, value }
                if field == "osd.timeout_ms" && value == "2500"
        ));
    }

    #[test]
    fn ui_settings_parser_builds_reset_request() {
        assert!(matches!(
            ui_settings_request(&args(&["reset", "bar"])),
            Ok(hyprbole_core::daemon::ShellRequest::BarSettingsReset)
        ));
    }

    #[test]
    fn ui_settings_parser_builds_osd_reset_request() {
        assert!(matches!(
            ui_settings_request(&args(&["reset", "osd"])),
            Ok(hyprbole_core::daemon::ShellRequest::OsdSettingsReset)
        ));
    }

    #[test]
    fn ui_settings_parser_rejects_unknown_shape() {
        assert!(ui_settings_request(&args(&["reset", "all"])).is_err());
    }

    #[test]
    fn theme_parser_builds_get_request() {
        assert!(matches!(
            theme_request(&[]),
            Ok(hyprbole_core::daemon::ShellRequest::ThemeGet)
        ));
    }

    #[test]
    fn theme_parser_builds_mode_request() {
        let request = theme_request(&args(&["mode", "light"])).unwrap();
        assert!(matches!(
            request,
            hyprbole_core::daemon::ShellRequest::ThemeSetMode {
                mode: hyprbole_core::theme::ThemeMode::Light
            }
        ));
    }

    #[test]
    fn theme_parser_rejects_unknown_shape() {
        assert!(theme_request(&args(&["mode", "blue"])).is_err());
    }

    #[test]
    fn notifications_parser_builds_get_request() {
        assert!(matches!(
            notifications_request(&[]),
            Ok(hyprbole_core::daemon::ShellRequest::NotificationsGet)
        ));
    }

    #[test]
    fn notifications_parser_builds_dnd_toggle_action() {
        let request = notifications_request(&args(&["dnd", "toggle"])).unwrap();
        assert!(matches!(
            request,
            hyprbole_core::daemon::ShellRequest::ActionCall {
                action: hyprbole_core::daemon::ShellAction::Notifications {
                    action: hyprbole_core::notifications::NotificationAction::ToggleDnd
                }
            }
        ));
    }

    #[test]
    fn notifications_parser_builds_push_action() {
        let request = notifications_request(&args(&["push", "Hello", "World"])).unwrap();
        assert!(matches!(
            request,
            hyprbole_core::daemon::ShellRequest::ActionCall {
                action: hyprbole_core::daemon::ShellAction::Notifications {
                    action: hyprbole_core::notifications::NotificationAction::Push { summary, body, .. }
                }
            } if summary == "Hello" && body == "World"
        ));
    }

    #[test]
    fn notifications_parser_rejects_unknown_shape() {
        assert!(notifications_request(&args(&["dnd", "maybe"])).is_err());
    }

    #[test]
    fn bar_cmdline_requires_ui_binary_and_bar_flag() {
        assert!(cmdline_is_bar(b"/tmp/hyprbole-ui\0--bar\0"));
        assert!(cmdline_is_bar(b"hyprbole-ui\0--layer-spike\0"));
        assert!(!cmdline_is_bar(b"/tmp/hyprbole-ui\0--quick\0"));
        assert!(!cmdline_is_bar(b"/tmp/other\0--bar\0"));
    }

    #[test]
    fn osd_cmdline_requires_ui_binary_and_osd_flag() {
        assert!(cmdline_is_osd(b"/tmp/hyprbole-ui\0--osd\0"));
        assert!(!cmdline_is_osd(b"/tmp/hyprbole-ui\0--bar\0"));
        assert!(!cmdline_is_osd(b"/tmp/other\0--osd\0"));
    }

    #[test]
    fn launcher_layer_cmdline_requires_layer_mode() {
        assert!(!cmdline_is_launcher_layer(
            b"/tmp/hyprbole-ui\0--launcher\0"
        ));
        assert!(cmdline_is_launcher_layer(
            b"/tmp/hyprbole-ui\0--launcher-layer\0"
        ));
        assert!(cmdline_is_launcher_layer(
            b"/tmp/hyprbole-ui\0--launcher-layer\0--stdin\0"
        ));
        assert!(!cmdline_is_launcher_layer(
            b"/tmp/hyprbole-ui\0--launcher-gtk\0"
        ));
        assert!(!cmdline_is_launcher_layer(b"/tmp/hyprbole-ui\0--quick\0"));
        assert!(!cmdline_is_launcher_layer(
            b"/tmp/other\0--launcher-layer\0"
        ));
        assert!(cmdline_is_launcher_gtk(
            b"/tmp/hyprbole-ui\0--launcher-gtk\0"
        ));
        assert!(!cmdline_is_launcher_gtk(
            b"/tmp/hyprbole-ui\0--launcher-layer\0"
        ));
    }

    #[test]
    fn remove_matching_surface_pidfile_preserves_changed_identity() {
        let dir = hyprbole_core::runtime::ensure_runtime_dir().expect("runtime dir");
        let surface = format!("hbctl-test-{}", std::process::id());
        let path = dir.join(format!("{surface}.pid"));
        let _ = std::fs::remove_file(&path);

        std::fs::write(&path, "1:old").expect("write pidfile");
        remove_matching_surface_pidfile(&surface, "2:new");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "1:old");

        remove_matching_surface_pidfile(&surface, "1:old");
        assert!(!path.exists());
    }

    #[test]
    fn wait_for_bar_identity_exit_reports_absent_target() {
        assert!(wait_for_bar_identity_exit(
            0,
            "0:0",
            std::time::Duration::from_millis(1)
        ));
    }

    #[test]
    fn wait_for_osd_identity_exit_reports_absent_target() {
        assert!(wait_for_osd_identity_exit(
            0,
            "0:0",
            std::time::Duration::from_millis(1)
        ));
    }

    #[test]
    fn wait_for_quick_identity_exit_reports_absent_target() {
        assert!(wait_for_quick_identity_exit(
            0,
            "0:0",
            std::time::Duration::from_millis(1)
        ));
    }
}

fn launch_control(args: &[String]) -> ExitCode {
    let Some(mut launch) = ui_launch_options(args) else {
        return ExitCode::from(2);
    };
    if launch
        .ui_args
        .iter()
        .any(|arg| arg == "--debug-direct-fallback")
    {
        eprintln!("hbctl control does not support --debug-direct-fallback");
        return ExitCode::from(2);
    }
    launch.ui_args.insert(0, "--control".to_string());
    launch_ui_with_args(launch.foreground, "control", &launch.ui_args)
}

pub(crate) fn launch_launcher(args: &[String]) -> ExitCode {
    let explicit_gtk = args.iter().any(|arg| arg == "--gtk");
    let dev = args.iter().any(|arg| arg == "--dev");
    let explicit_layer = args.iter().any(|arg| arg == "--layer");
    let gtk = explicit_gtk || (!explicit_layer && !dev);
    let layer = explicit_layer;
    let stdin = args.iter().any(|arg| arg == "--stdin");
    let toggle = args.iter().any(|arg| arg == "--toggle");
    let restart = args.iter().any(|arg| arg == "--restart");
    let Some(layer_options) = launcher_layer_options(args) else {
        return ExitCode::from(2);
    };
    if toggle && restart {
        eprintln!("hbctl launcher supports only one of --toggle or --restart");
        return ExitCode::from(2);
    }
    if explicit_gtk && dev {
        eprintln!("hbctl launcher supports only one of --dev or --gtk");
        return ExitCode::from(2);
    }
    if explicit_layer && (explicit_gtk || dev) {
        eprintln!("hbctl launcher --layer cannot be combined with --dev or --gtk");
        return ExitCode::from(2);
    }
    if stdin && !layer {
        eprintln!("hbctl launcher --stdin requires the layer launcher");
        return ExitCode::from(2);
    }
    if !layer
        && (layer_options.prompt.is_some()
            || layer_options.placeholder.is_some()
            || layer_options.lines.is_some())
    {
        eprintln!("hbctl launcher --prompt/--placeholder/--lines require the layer launcher");
        return ExitCode::from(2);
    }
    if (toggle || restart) && !layer {
        eprintln!("hbctl launcher --toggle/--restart require the layer launcher");
        return ExitCode::from(2);
    }
    if stdin && (toggle || restart) {
        eprintln!("hbctl launcher --stdin does not support --toggle or --restart");
        return ExitCode::from(2);
    }
    let filtered_args = launcher_ui_launch_args(args);
    let Some(mut launch) = ui_launch_options(&filtered_args) else {
        return ExitCode::from(2);
    };
    if stdin && !launch.foreground {
        eprintln!("hbctl launcher --stdin requires --foreground so stdin is available");
        return ExitCode::from(2);
    }
    if launch
        .ui_args
        .iter()
        .any(|arg| arg == "--debug-direct-fallback")
    {
        eprintln!("hbctl launcher does not support --debug-direct-fallback");
        return ExitCode::from(2);
    }
    if layer {
        if let Some((pid, identity)) = launcher_layer_instance() {
            if toggle || restart {
                let exit = stop_launcher_layer(pid, &identity);
                if !exit_success(exit) || toggle {
                    return exit;
                }
            } else {
                println!("launcher already running pid {pid}");
                return ExitCode::SUCCESS;
            }
        }
    } else if gtk && let Some(pid) = launcher_gtk_instance() {
        println!("gtk launcher already running pid {pid}");
        return ExitCode::SUCCESS;
    }
    launch.ui_args.insert(
        0,
        if layer {
            "--launcher-layer".to_string()
        } else if gtk {
            "--launcher-gtk".to_string()
        } else {
            "--launcher".to_string()
        },
    );
    if stdin {
        launch.ui_args.push("--stdin".to_string());
    }
    if let Some(prompt) = layer_options.prompt {
        launch.ui_args.push("--prompt".to_string());
        launch.ui_args.push(prompt);
    }
    if let Some(placeholder) = layer_options.placeholder {
        launch.ui_args.push("--placeholder".to_string());
        launch.ui_args.push(placeholder);
    }
    if let Some(lines) = layer_options.lines {
        launch.ui_args.push("--lines".to_string());
        launch.ui_args.push(lines.to_string());
    }
    launch_ui_with_args(launch.foreground, "launcher", &launch.ui_args)
}

struct LauncherLayerOptions {
    prompt: Option<String>,
    placeholder: Option<String>,
    lines: Option<usize>,
}

fn launcher_layer_options(args: &[String]) -> Option<LauncherLayerOptions> {
    let mut options = LauncherLayerOptions {
        prompt: None,
        placeholder: None,
        lines: None,
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--prompt" => {
                let Some(value) = args.get(index + 1).filter(|value| !value.starts_with("--"))
                else {
                    eprintln!("hbctl launcher --prompt requires a value");
                    return None;
                };
                options.prompt = Some(value.clone());
                index += 1;
            }
            "--placeholder" => {
                let Some(value) = args.get(index + 1).filter(|value| !value.starts_with("--"))
                else {
                    eprintln!("hbctl launcher --placeholder requires a value");
                    return None;
                };
                options.placeholder = Some(value.clone());
                index += 1;
            }
            "--lines" => {
                let Some(value) = args.get(index + 1).filter(|value| !value.starts_with("--"))
                else {
                    eprintln!("hbctl launcher --lines requires a value");
                    return None;
                };
                let Ok(lines) = value.parse::<usize>() else {
                    eprintln!("hbctl launcher --lines requires a positive integer");
                    return None;
                };
                if lines == 0 {
                    eprintln!("hbctl launcher --lines requires a positive integer");
                    return None;
                }
                options.lines = Some(lines);
                index += 1;
            }
            _ => {}
        }
        index += 1;
    }
    Some(options)
}

fn launcher_ui_launch_args(args: &[String]) -> Vec<String> {
    let mut filtered = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--gtk" | "--dev" | "--layer" | "--stdin" | "--toggle" | "--restart" | "--launcher"
            | "--launcher-layer" | "--launcher-gtk" => {}
            "--prompt" | "--placeholder" | "--lines" => index += 1,
            _ => filtered.push(args[index].clone()),
        }
        index += 1;
    }
    filtered
}

fn launch_quick(args: &[String]) -> ExitCode {
    let toggle = args.iter().any(|arg| arg == "--toggle");
    let restart = args.iter().any(|arg| arg == "--restart");
    let dev = args.iter().any(|arg| arg == "--dev");
    if toggle && restart {
        eprintln!("hbctl quick supports only one of --toggle or --restart");
        return ExitCode::from(2);
    }
    let filtered_args = args
        .iter()
        .filter(|arg| !matches!(arg.as_str(), "--toggle" | "--restart" | "--dev"))
        .cloned()
        .collect::<Vec<_>>();
    let Some(mut launch) = ui_launch_options(&filtered_args) else {
        return ExitCode::from(2);
    };
    if launch
        .ui_args
        .iter()
        .any(|arg| arg == "--debug-direct-fallback")
    {
        eprintln!("hbctl quick does not support --debug-direct-fallback");
        return ExitCode::from(2);
    }
    if let Some((pid, identity)) = quick_settings_instance() {
        if toggle || restart {
            let exit = dismiss_quick_settings(pid, &identity);
            if !exit_success(exit) || toggle {
                return exit;
            }
        } else {
            println!("quick settings already running pid {pid}");
            return ExitCode::SUCCESS;
        }
    }
    if toggle {
        launch.ui_args.insert(0, "--quick-toggle".to_string());
    }
    if dev {
        launch.ui_args.insert(0, "--quick-dev".to_string());
    }
    launch.ui_args.insert(0, "--quick".to_string());
    launch_ui_with_args(launch.foreground, "quick", &launch.ui_args)
}

fn launch_osd(args: &[String]) -> ExitCode {
    let toggle = args.iter().any(|arg| arg == "--toggle");
    let restart = args.iter().any(|arg| arg == "--restart");
    if toggle && restart {
        eprintln!("hbctl osd supports only one of --toggle or --restart");
        return ExitCode::from(2);
    }
    let filtered_args = args
        .iter()
        .filter(|arg| !matches!(arg.as_str(), "--toggle" | "--restart"))
        .cloned()
        .collect::<Vec<_>>();
    let Some(mut launch) = ui_launch_options(&filtered_args) else {
        return ExitCode::from(2);
    };
    if launch
        .ui_args
        .iter()
        .any(|arg| arg == "--debug-direct-fallback")
    {
        eprintln!("hbctl osd does not support --debug-direct-fallback");
        return ExitCode::from(2);
    }
    if let Some((pid, identity)) = osd_instance() {
        if toggle || restart {
            let exit = stop_osd(pid, &identity);
            if !exit_success(exit) || toggle {
                return exit;
            }
        } else {
            println!("osd already running pid {pid}");
            return ExitCode::SUCCESS;
        }
    } else if toggle {
        // Toggle means open when absent.
    }
    launch.ui_args.insert(0, "--osd".to_string());
    launch_ui_with_args(launch.foreground, "osd", &launch.ui_args)
}

fn launch_layer_spike(args: &[String]) -> ExitCode {
    let toggle = args.iter().any(|arg| arg == "--toggle");
    let restart = args.iter().any(|arg| arg == "--restart");
    let gtk = args.iter().any(|arg| arg == "--gtk");
    if toggle && restart {
        eprintln!("hbctl bar supports only one of --toggle or --restart");
        return ExitCode::from(2);
    }
    let filtered_args = args
        .iter()
        .filter(|arg| !matches!(arg.as_str(), "--toggle" | "--restart" | "--gtk"))
        .cloned()
        .collect::<Vec<_>>();
    let Some(mut launch) = ui_launch_options(&filtered_args) else {
        return ExitCode::from(2);
    };
    if let Some((pid, identity)) = bar_instance() {
        if toggle || restart {
            let exit = stop_bar(pid, &identity);
            if !exit_success(exit) || toggle {
                return exit;
            }
        } else {
            println!("bar already running pid {pid}");
            return ExitCode::SUCCESS;
        }
    } else if toggle {
        // Toggle means open when absent.
    }
    launch.ui_args.insert(
        0,
        if gtk {
            "--bar-gtk".to_string()
        } else {
            "--bar".to_string()
        },
    );
    launch_ui_with_args(launch.foreground, "bar", &launch.ui_args)
}

fn exit_success(code: ExitCode) -> bool {
    code == ExitCode::SUCCESS
}

fn osd_instance() -> Option<(u32, String)> {
    let Ok(path) = hyprbole_core::runtime::ensure_runtime_dir().map(|dir| dir.join("osd.pid"))
    else {
        return running_osd_instance();
    };
    let pidfile_identity = std::fs::read_to_string(path)
        .ok()
        .and_then(|value| parse_quick_identity(&value));
    if let Some(identity) = pidfile_identity.filter(osd_identity_alive) {
        return Some(identity);
    }
    running_osd_instance()
}

fn running_osd_instance() -> Option<(u32, String)> {
    let entries = std::fs::read_dir("/proc").ok()?;
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if osd_process_alive(pid) {
            if let Some(identity) = quick_process_identity(pid) {
                return Some((pid, identity));
            }
        }
    }
    None
}

fn osd_identity_alive(identity: &(u32, String)) -> bool {
    osd_process_alive(identity.0)
        && quick_process_identity(identity.0).as_deref() == Some(&identity.1)
}

fn osd_process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
        && std::fs::read(format!("/proc/{pid}/cmdline"))
            .map(|cmdline| cmdline_is_osd(&cmdline))
            .unwrap_or(false)
}

fn cmdline_is_osd(cmdline: &[u8]) -> bool {
    let args = cmdline
        .split(|byte| *byte == 0)
        .filter_map(|arg| std::str::from_utf8(arg).ok())
        .collect::<Vec<_>>();
    args.first().is_some_and(|arg| arg.ends_with("hyprbole-ui")) && args.contains(&"--osd")
}

fn stop_osd(pid: u32, identity: &str) -> ExitCode {
    if !osd_identity_alive(&(pid, identity.to_string())) {
        println!("osd already closed");
        return ExitCode::SUCCESS;
    }
    match signal_osd_identity(pid, identity) {
        Ok(true) => {
            if !wait_for_osd_identity_exit(pid, identity, Duration::from_millis(1000)) {
                eprintln!("osd pid {pid} did not stop within timeout");
                return ExitCode::from(1);
            }
            remove_matching_surface_pidfile("osd", identity);
            println!("stopped osd pid {pid}");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            println!("osd already closed");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to stop osd pid {pid}: {err}");
            ExitCode::from(1)
        }
    }
}

fn wait_for_osd_identity_exit(pid: u32, identity: &str, timeout: Duration) -> bool {
    let target = (pid, identity.to_string());
    let started = std::time::Instant::now();
    while osd_identity_alive(&target) && started.elapsed() < timeout {
        thread::sleep(Duration::from_millis(50));
    }
    !osd_identity_alive(&target)
}

fn signal_osd_identity(pid: u32, identity: &str) -> Result<bool, String> {
    signal_process_identity(pid, identity, osd_identity_alive)
}

fn remove_matching_surface_pidfile(surface: &str, identity: &str) {
    let Ok(path) =
        hyprbole_core::runtime::ensure_runtime_dir().map(|dir| dir.join(format!("{surface}.pid")))
    else {
        return;
    };
    let Ok(value) = std::fs::read_to_string(&path) else {
        return;
    };
    if value.trim() != identity {
        return;
    }

    let trash = path.with_file_name(format!(".{surface}.pid.hbctl-stop-{}", std::process::id()));
    let _ = std::fs::remove_file(&trash);
    if std::fs::rename(&path, &trash).is_err() {
        return;
    }
    if std::fs::read_to_string(&trash).ok().as_deref() == Some(value.as_str()) {
        let _ = std::fs::remove_file(trash);
    } else if !path.exists() {
        if std::fs::hard_link(&trash, &path).is_ok() {
            let _ = std::fs::remove_file(trash);
        }
    } else {
        let _ = std::fs::remove_file(trash);
    }
}

fn bar_instance() -> Option<(u32, String)> {
    let Ok(path) = hyprbole_core::runtime::ensure_runtime_dir().map(|dir| dir.join("bar.pid"))
    else {
        return running_bar_instance();
    };
    let pidfile_identity = std::fs::read_to_string(path)
        .ok()
        .and_then(|value| parse_quick_identity(&value));
    if let Some(identity) = pidfile_identity.filter(bar_identity_alive) {
        return Some(identity);
    }
    running_bar_instance()
}

pub(crate) fn launcher_layer_instance() -> Option<(u32, String)> {
    let entries = std::fs::read_dir("/proc").ok()?;
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if launcher_layer_process_alive(pid)
            && let Some(identity) = quick_process_identity(pid)
        {
            return Some((pid, identity));
        }
    }
    None
}

fn launcher_gtk_instance() -> Option<u32> {
    let current = std::process::id();
    let entries = std::fs::read_dir("/proc").ok()?;
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if pid != current && launcher_gtk_process_alive(pid) {
            return Some(pid);
        }
    }
    None
}

fn launcher_gtk_process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
        && std::fs::read(format!("/proc/{pid}/cmdline"))
            .map(|cmdline| cmdline_is_launcher_gtk(&cmdline))
            .unwrap_or(false)
}

fn launcher_layer_process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
        && std::fs::read(format!("/proc/{pid}/cmdline"))
            .map(|cmdline| cmdline_is_launcher_layer(&cmdline))
            .unwrap_or(false)
}

pub(crate) fn launcher_layer_identity_alive(identity: &(u32, String)) -> bool {
    launcher_layer_process_alive(identity.0)
        && quick_process_identity(identity.0).as_deref() == Some(&identity.1)
}

fn cmdline_is_launcher_gtk(cmdline: &[u8]) -> bool {
    let args = cmdline
        .split(|byte| *byte == 0)
        .filter_map(|arg| std::str::from_utf8(arg).ok())
        .collect::<Vec<_>>();
    args.first().is_some_and(|arg| arg.ends_with("hyprbole-ui")) && args.contains(&"--launcher-gtk")
}

fn cmdline_is_launcher_layer(cmdline: &[u8]) -> bool {
    let args = cmdline
        .split(|byte| *byte == 0)
        .filter_map(|arg| std::str::from_utf8(arg).ok())
        .collect::<Vec<_>>();
    args.first().is_some_and(|arg| arg.ends_with("hyprbole-ui"))
        && args.contains(&"--launcher-layer")
}

pub(crate) fn stop_launcher_layer(pid: u32, identity: &str) -> ExitCode {
    if !launcher_layer_identity_alive(&(pid, identity.to_string())) {
        println!("launcher already closed");
        return ExitCode::SUCCESS;
    }
    match signal_process_identity(pid, identity, launcher_layer_identity_alive) {
        Ok(true) => {
            if !wait_for_launcher_layer_identity_exit(pid, identity, Duration::from_millis(1000)) {
                eprintln!("launcher pid {pid} did not stop within timeout");
                return ExitCode::from(1);
            }
            remove_matching_surface_pidfile("launcher", identity);
            println!("stopped launcher pid {pid}");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            println!("launcher already closed");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to stop launcher pid {pid}: {err}");
            ExitCode::from(1)
        }
    }
}

fn wait_for_launcher_layer_identity_exit(pid: u32, identity: &str, timeout: Duration) -> bool {
    let target = (pid, identity.to_string());
    let started = std::time::Instant::now();
    while launcher_layer_identity_alive(&target) && started.elapsed() < timeout {
        thread::sleep(Duration::from_millis(50));
    }
    !launcher_layer_identity_alive(&target)
}

fn running_bar_instance() -> Option<(u32, String)> {
    let entries = std::fs::read_dir("/proc").ok()?;
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if bar_process_alive(pid) {
            if let Some(identity) = quick_process_identity(pid) {
                return Some((pid, identity));
            }
        }
    }
    None
}

fn bar_identity_alive(identity: &(u32, String)) -> bool {
    bar_process_alive(identity.0)
        && quick_process_identity(identity.0).as_deref() == Some(&identity.1)
}

fn bar_process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
        && std::fs::read(format!("/proc/{pid}/cmdline"))
            .map(|cmdline| cmdline_is_bar(&cmdline))
            .unwrap_or(false)
}

fn cmdline_is_bar(cmdline: &[u8]) -> bool {
    let args = cmdline
        .split(|byte| *byte == 0)
        .filter_map(|arg| std::str::from_utf8(arg).ok())
        .collect::<Vec<_>>();
    args.first().is_some_and(|arg| arg.ends_with("hyprbole-ui"))
        && (args.contains(&"--bar")
            || args.contains(&"--layer-spike")
            || args.contains(&"--bar-gtk"))
}

fn stop_bar(pid: u32, identity: &str) -> ExitCode {
    if !bar_identity_alive(&(pid, identity.to_string())) {
        println!("bar already closed");
        return ExitCode::SUCCESS;
    }
    match signal_bar_identity(pid, identity) {
        Ok(true) => {
            if !wait_for_bar_identity_exit(pid, identity, Duration::from_millis(1000)) {
                eprintln!("bar pid {pid} did not stop within timeout");
                return ExitCode::from(1);
            }
            remove_matching_surface_pidfile("bar", identity);
            println!("stopped bar pid {pid}");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            println!("bar already closed");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to stop bar pid {pid}: {err}");
            ExitCode::from(1)
        }
    }
}

fn wait_for_bar_identity_exit(pid: u32, identity: &str, timeout: Duration) -> bool {
    let target = (pid, identity.to_string());
    let started = std::time::Instant::now();
    while bar_identity_alive(&target) && started.elapsed() < timeout {
        thread::sleep(Duration::from_millis(50));
    }
    !bar_identity_alive(&target)
}

fn signal_bar_identity(pid: u32, identity: &str) -> Result<bool, String> {
    signal_process_identity(pid, identity, bar_identity_alive)
}

fn signal_process_identity(
    pid: u32,
    identity: &str,
    identity_alive: fn(&(u32, String)) -> bool,
) -> Result<bool, String> {
    let Some(fd) = pidfd_open(pid)? else {
        return Ok(false);
    };
    let target = (pid, identity.to_string());
    if !identity_alive(&target) {
        close_fd(fd);
        return Ok(false);
    }
    let result = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd,
            libc::SIGTERM,
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    };
    let error = if result == -1 {
        Some(std::io::Error::last_os_error())
    } else {
        None
    };
    close_fd(fd);
    match error {
        None => Ok(true),
        Some(err) if err.raw_os_error() == Some(libc::ESRCH) => Ok(false),
        Some(err) => Err(err.to_string()),
    }
}

fn pidfd_open(pid: u32) -> Result<Option<libc::c_int>, String> {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
    if fd == -1 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            return Ok(None);
        }
        return Err(err.to_string());
    }
    Ok(Some(fd as libc::c_int))
}

fn close_fd(fd: libc::c_int) {
    let _ = unsafe { libc::close(fd) };
}

fn quick_settings_instance() -> Option<(u32, String)> {
    let Ok(path) = hyprbole_core::runtime::ensure_runtime_dir().map(|dir| dir.join("quick.pid"))
    else {
        return running_quick_instance();
    };
    let pidfile_identity = std::fs::read_to_string(path)
        .ok()
        .and_then(|value| parse_quick_identity(&value));
    if let Some(identity) = pidfile_identity.filter(quick_identity_alive) {
        return Some(identity);
    }
    running_quick_instance()
}

fn running_quick_instance() -> Option<(u32, String)> {
    let entries = std::fs::read_dir("/proc").ok()?;
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if quick_process_alive(pid) {
            if let Some(identity) = quick_process_identity(pid) {
                return Some((pid, identity));
            }
        }
    }
    None
}

fn quick_process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
        && std::fs::read(format!("/proc/{pid}/cmdline"))
            .map(|cmdline| cmdline_is_quick_settings(&cmdline))
            .unwrap_or(false)
}

fn quick_identity_alive(identity: &(u32, String)) -> bool {
    quick_process_alive(identity.0)
        && quick_process_identity(identity.0).as_deref() == Some(&identity.1)
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
    let (pid, start_time) = value.split_once(':')?;
    let pid = pid.parse().ok()?;
    start_time.parse::<u64>().ok()?;
    Some((pid, value.to_string()))
}

fn dismiss_quick_settings(pid: u32, identity: &str) -> ExitCode {
    if quick_settings_instance()
        .as_ref()
        .map(|(_pid, id)| id.as_str())
        != Some(identity)
    {
        println!("quick settings already closed");
        return ExitCode::SUCCESS;
    }
    match quick_dismiss_path().and_then(|path| std::fs::write(path, identity).ok()) {
        Some(()) => {
            if !wait_for_quick_identity_exit(pid, identity, QUICK_DISMISS_TIMEOUT) {
                eprintln!("quick settings pid {pid} did not dismiss within timeout");
                return ExitCode::from(1);
            }
            println!("requested quick settings dismiss pid {pid}");
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("failed to request quick settings dismiss");
            ExitCode::from(1)
        }
    }
}

fn wait_for_quick_identity_exit(pid: u32, identity: &str, timeout: Duration) -> bool {
    let target = (pid, identity.to_string());
    let started = std::time::Instant::now();
    while quick_identity_alive(&target) && started.elapsed() < timeout {
        thread::sleep(Duration::from_millis(50));
    }
    !quick_identity_alive(&target)
}

fn quick_dismiss_path() -> Option<PathBuf> {
    hyprbole_core::runtime::ensure_runtime_dir()
        .ok()
        .map(|dir| dir.join("quick.dismiss"))
}

struct UiLaunchOptions {
    foreground: bool,
    ui_args: Vec<String>,
}

fn ui_launch_options(args: &[String]) -> Option<UiLaunchOptions> {
    let mut foreground = false;
    let mut ui_args = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--foreground" => foreground = true,
            "--debug-direct-fallback" => ui_args.push("--debug-direct-fallback".to_string()),
            unknown => {
                eprintln!("unknown UI launch argument: {unknown}");
                return None;
            }
        }
    }
    Some(UiLaunchOptions {
        foreground,
        ui_args,
    })
}

fn launch_ui_with_args(foreground: bool, log_name: &str, args: &[String]) -> ExitCode {
    let Some(ui_bin) = ui_binary_path() else {
        eprintln!("failed to locate hyprbole-ui binary");
        return ExitCode::from(1);
    };

    if build_workspace_package("hyprbole-ui").is_err() {
        return ExitCode::from(1);
    }

    launch_binary_with_args(&ui_bin, foreground, log_name, "hyprbole-ui", args)
}

fn launch_daemon(foreground: bool) -> ExitCode {
    if hyprbole_core::daemon::DaemonClient::from_env()
        .and_then(|client| client.send_shell(&hyprbole_core::daemon::ShellRequest::Ping))
        .is_ok()
    {
        eprintln!("hyprbole daemon is already running");
        return ExitCode::from(1);
    }

    let Some(daemon_bin) = sibling_binary_path("hyprbole") else {
        eprintln!("failed to locate hyprbole daemon binary");
        return ExitCode::from(1);
    };

    if !daemon_bin.exists() && build_workspace_package("hyprbole-daemon").is_err() {
        return ExitCode::from(1);
    }

    launch_binary(&daemon_bin, foreground, "daemon", "hyprbole daemon")
}

fn ui_binary_path() -> Option<PathBuf> {
    sibling_binary_path("hyprbole-ui")
}

fn sibling_binary_path(name: &str) -> Option<PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join(name)))
}

fn launch_binary(binary: &Path, foreground: bool, log_name: &str, label: &str) -> ExitCode {
    launch_binary_with_args(binary, foreground, log_name, label, &[])
}

fn launch_binary_with_args(
    binary: &Path,
    foreground: bool,
    log_name: &str,
    label: &str,
    args: &[String],
) -> ExitCode {
    if foreground {
        match Command::new(binary).args(args).status() {
            Ok(status) if status.success() => ExitCode::SUCCESS,
            Ok(status) => {
                eprintln!("{label} exited with {status}");
                ExitCode::from(1)
            }
            Err(err) => {
                eprintln!("failed to launch {}: {err}", binary.display());
                ExitCode::from(1)
            }
        }
    } else {
        let log_file = match open_log_file(log_name) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("failed to open {log_name} log: {err}");
                return ExitCode::from(1);
            }
        };
        let log_file_stderr = match log_file.try_clone() {
            Ok(file) => file,
            Err(err) => {
                eprintln!("failed to clone {log_name} log handle: {err}");
                return ExitCode::from(1);
            }
        };
        match Command::new(binary)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_file_stderr))
            .spawn()
        {
            Ok(child) => {
                println!("launched {label} pid {}", child.id());
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("failed to launch {}: {err}", binary.display());
                ExitCode::from(1)
            }
        }
    }
}

fn open_log_file(name: &str) -> std::io::Result<std::fs::File> {
    hyprbole_core::runtime::ensure_runtime_dir()
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    let path = hyprbole_core::runtime::log_path(name)
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    OpenOptions::new().create(true).append(true).open(path)
}

fn build_workspace_package(package: &str) -> Result<(), ()> {
    let Some(root) = workspace_root() else {
        eprintln!("failed to locate Hyprbole workspace root");
        return Err(());
    };

    let build_status = Command::new("cargo")
        .args([
            "build",
            "--quiet",
            "--manifest-path",
            root.join("Cargo.toml").to_string_lossy().as_ref(),
            "-p",
            package,
        ])
        .status();

    match build_status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => {
            eprintln!("cargo build failed with {status}");
            Err(())
        }
        Err(err) => {
            eprintln!("failed to run cargo build: {err}");
            Err(())
        }
    }
}

fn workspace_root() -> Option<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent()?.parent().map(Path::to_path_buf)
}
