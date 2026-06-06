use std::process::ExitCode;

use super::{
    SurfaceStatus, SurfaceTarget, surface_status_healthy, surface_target, surface_target_label,
    surface_target_statuses,
};

pub(super) fn surface_doctor_command(args: &[String]) -> Option<ExitCode> {
    if args.first().map(String::as_str) != Some("doctor") {
        return None;
    }
    let Some(options) = surface_doctor_options(&args[1..]) else {
        surface_doctor_usage();
        return Some(ExitCode::from(2));
    };
    Some(run_surface_doctor(options))
}

fn surface_doctor_usage() {
    eprintln!(
        "usage: hbctl surfaces doctor [quick|bar|osd|launcher|all] [--json|--commands] [--unhealthy-only] [--actionable-only] [--zero-ok] [--warnings-only|--errors-only]"
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SurfaceDoctorOptions {
    pub(super) target: SurfaceTarget,
    pub(super) json: bool,
    pub(super) commands: bool,
    pub(super) unhealthy_only: bool,
    pub(super) actionable_only: bool,
    pub(super) zero_ok: bool,
    pub(super) warnings_only: bool,
    pub(super) errors_only: bool,
}

impl Default for SurfaceDoctorOptions {
    fn default() -> Self {
        Self {
            target: SurfaceTarget::All,
            json: false,
            commands: false,
            unhealthy_only: false,
            actionable_only: false,
            zero_ok: false,
            warnings_only: false,
            errors_only: false,
        }
    }
}

pub(super) fn surface_doctor_options(args: &[String]) -> Option<SurfaceDoctorOptions> {
    let mut options = SurfaceDoctorOptions::default();
    let mut seen_target = false;
    let mut seen_json = false;
    let mut seen_commands = false;
    let mut seen_unhealthy_only = false;
    let mut seen_actionable_only = false;
    let mut seen_zero_ok = false;
    let mut seen_warnings_only = false;
    let mut seen_errors_only = false;
    for arg in args {
        match arg.as_str() {
            "--json" if !seen_json => {
                seen_json = true;
                options.json = true;
            }
            "--commands" if !seen_commands => {
                seen_commands = true;
                options.commands = true;
            }
            "--unhealthy-only" if !seen_unhealthy_only => {
                seen_unhealthy_only = true;
                options.unhealthy_only = true;
            }
            "--actionable-only" if !seen_actionable_only => {
                seen_actionable_only = true;
                options.actionable_only = true;
            }
            "--zero-ok" if !seen_zero_ok => {
                seen_zero_ok = true;
                options.zero_ok = true;
            }
            "--warnings-only" if !seen_warnings_only && !seen_errors_only => {
                seen_warnings_only = true;
                options.warnings_only = true;
            }
            "--errors-only" if !seen_errors_only => {
                if seen_warnings_only {
                    return None;
                }
                seen_errors_only = true;
                options.errors_only = true;
            }
            target if !seen_target => {
                options.target = surface_target(target)?;
                seen_target = true;
            }
            _ => return None,
        }
    }
    if options.json && options.commands {
        return None;
    }
    Some(options)
}

#[derive(serde::Serialize)]
pub(super) struct SurfaceDoctorReport {
    pub(super) target: &'static str,
    pub(super) output_mode: &'static str,
    pub(super) healthy: bool,
    pub(super) selected: usize,
    pub(super) healthy_count: usize,
    pub(super) issues: usize,
    pub(super) warnings: usize,
    pub(super) errors: usize,
    pub(super) actionable: usize,
    pub(super) displayed: usize,
    pub(super) filtered: bool,
    pub(super) active_filters: Vec<&'static str>,
    pub(super) zero_ok: bool,
    pub(super) severity_filter: &'static str,
    pub(super) surface_names: Vec<&'static str>,
    pub(super) issue_labels: Vec<String>,
    pub(super) command_count: usize,
    pub(super) has_commands: bool,
    pub(super) recommended_commands: Vec<String>,
    pub(super) surfaces: Vec<SurfaceDoctorEntry>,
}

#[derive(serde::Serialize)]
pub(super) struct SurfaceDoctorEntry {
    pub(super) name: &'static str,
    pub(super) healthy: bool,
    pub(super) severity: &'static str,
    pub(super) state: &'static str,
    pub(super) pid_file_state: &'static str,
    pub(super) log_state: &'static str,
    pub(super) issue: &'static str,
    pub(super) log_issue: &'static str,
    pub(super) recommendation: String,
    pub(super) recommended_command: Option<String>,
    pub(super) status: SurfaceStatus,
}

fn run_surface_doctor(options: SurfaceDoctorOptions) -> ExitCode {
    let report =
        surface_doctor_report_from_statuses(options, surface_target_statuses(options.target));
    if options.json {
        return match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                surface_doctor_exit_code(&report)
            }
            Err(err) => {
                eprintln!("failed to encode surface doctor: {err}");
                ExitCode::from(1)
            }
        };
    }
    if options.commands {
        print_surface_doctor_commands(&report);
        return surface_doctor_exit_code(&report);
    }
    print_surface_doctor_report(&report);
    surface_doctor_exit_code(&report)
}

pub(super) fn surface_doctor_exit_code(report: &SurfaceDoctorReport) -> ExitCode {
    if report.healthy || report.zero_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

pub(super) fn surface_doctor_report_from_statuses(
    options: SurfaceDoctorOptions,
    statuses: Vec<SurfaceStatus>,
) -> SurfaceDoctorReport {
    let mut entries = statuses
        .into_iter()
        .map(surface_doctor_entry)
        .collect::<Vec<_>>();
    let selected = entries.len();
    let healthy_count = entries.iter().filter(|entry| entry.healthy).count();
    let issues = entries.iter().filter(|entry| !entry.healthy).count();
    let warnings = entries
        .iter()
        .filter(|entry| entry.severity == "warning")
        .count();
    let errors = entries
        .iter()
        .filter(|entry| entry.severity == "error")
        .count();
    let actionable = entries
        .iter()
        .filter(|entry| entry.recommended_command.is_some())
        .count();
    if options.unhealthy_only {
        entries.retain(|entry| !entry.healthy);
    }
    if options.actionable_only {
        entries.retain(|entry| entry.recommended_command.is_some());
    }
    if options.warnings_only {
        entries.retain(|entry| entry.severity == "warning");
    }
    if options.errors_only {
        entries.retain(|entry| entry.severity == "error");
    }
    let surface_names = entries.iter().map(|entry| entry.name).collect::<Vec<_>>();
    let issue_labels = surface_doctor_issue_labels(&entries);
    let recommended_commands = surface_doctor_recommended_commands(&entries);
    let command_count = recommended_commands.len();
    let has_commands = command_count > 0;
    let displayed = entries.len();
    SurfaceDoctorReport {
        target: surface_target_label(options.target),
        output_mode: surface_doctor_output_mode(&options),
        healthy: issues == 0,
        selected,
        healthy_count,
        issues,
        warnings,
        errors,
        actionable,
        displayed,
        filtered: options.unhealthy_only
            || options.actionable_only
            || options.warnings_only
            || options.errors_only,
        active_filters: surface_doctor_active_filters(&options),
        zero_ok: options.zero_ok,
        severity_filter: surface_doctor_severity_filter(&options),
        surface_names,
        issue_labels,
        command_count,
        has_commands,
        recommended_commands,
        surfaces: entries,
    }
}

fn surface_doctor_output_mode(options: &SurfaceDoctorOptions) -> &'static str {
    if options.json {
        "json"
    } else if options.commands {
        "commands"
    } else {
        "plain"
    }
}

fn surface_doctor_active_filters(options: &SurfaceDoctorOptions) -> Vec<&'static str> {
    let mut filters = Vec::new();
    if options.unhealthy_only {
        filters.push("unhealthy");
    }
    if options.actionable_only {
        filters.push("actionable");
    }
    if options.warnings_only {
        filters.push("warning");
    }
    if options.errors_only {
        filters.push("error");
    }
    filters
}

pub(super) fn surface_doctor_issue_labels(entries: &[SurfaceDoctorEntry]) -> Vec<String> {
    let mut labels = Vec::new();
    for entry in entries {
        for label in [entry.issue, entry.log_issue] {
            if label != "none" && !labels.iter().any(|existing| existing == label) {
                labels.push(label.to_string());
            }
        }
    }
    labels
}

pub(super) fn surface_doctor_recommended_commands(entries: &[SurfaceDoctorEntry]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|entry| entry.recommended_command.clone())
        .collect()
}

pub(super) fn surface_doctor_severity_filter(options: &SurfaceDoctorOptions) -> &'static str {
    if options.warnings_only {
        "warning"
    } else if options.errors_only {
        "error"
    } else {
        "all"
    }
}

pub(super) fn surface_doctor_entry(status: SurfaceStatus) -> SurfaceDoctorEntry {
    let state = if status.running { "running" } else { "stopped" };
    let issue = surface_doctor_issue(&status);
    let log_issue = surface_doctor_log_issue(&status);
    let healthy = issue == "none" && log_issue == "none";
    let severity = surface_doctor_entry_severity(issue, log_issue);
    let recommendation = surface_doctor_recommendation(&status).to_string();
    let recommended_command = surface_doctor_recommended_command(&status);
    SurfaceDoctorEntry {
        name: status.name,
        healthy,
        severity,
        state,
        pid_file_state: status.pid_file_state,
        log_state: status.log_state,
        issue,
        log_issue,
        recommendation,
        recommended_command,
        status,
    }
}

pub(super) fn surface_doctor_entry_severity(issue: &str, log_issue: &str) -> &'static str {
    let issue_severity = surface_doctor_severity(issue);
    let log_severity = surface_doctor_severity(log_issue);
    match (issue_severity, log_severity) {
        ("error", _) | (_, "error") => "error",
        ("warning", _) | (_, "warning") => "warning",
        _ => "ok",
    }
}

pub(super) fn surface_doctor_severity(issue: &str) -> &'static str {
    match issue {
        "none" => "ok",
        "stale-pidfile" | "missing-pidfile" | "missing-log" | "empty-log" => "warning",
        _ => "error",
    }
}

pub(super) fn surface_doctor_issue(status: &SurfaceStatus) -> &'static str {
    if status.stale_pid_file {
        "stale-pidfile"
    } else if status.invalid_pid_file {
        "invalid-pidfile"
    } else if status.unreadable_pid_file {
        "unreadable-pidfile"
    } else if status.running && status.pid_file_state == "missing" {
        "missing-pidfile"
    } else if surface_status_healthy(status) {
        "none"
    } else {
        "unhealthy-pidfile"
    }
}

pub(super) fn surface_doctor_log_issue(status: &SurfaceStatus) -> &'static str {
    match status.log_state {
        "missing" if status.running => "missing-log",
        "unreadable" => "unreadable-log",
        "empty" => "empty-log",
        _ => "none",
    }
}

pub(super) fn surface_doctor_recommendation(status: &SurfaceStatus) -> String {
    let issue = surface_doctor_issue(status);
    let issue = if issue == "none" {
        surface_doctor_log_issue(status)
    } else {
        issue
    };
    match issue {
        "none" => "no action needed".to_string(),
        "stale-pidfile" => format!("run `hbctl surfaces clean {}`", status.name),
        "invalid-pidfile" => format!(
            "inspect {} and remove it manually if obsolete",
            surface_pid_file_display(status)
        ),
        "unreadable-pidfile" => {
            format!("check permissions for {}", surface_pid_file_display(status))
        }
        "missing-pidfile" => format!("restart {} to refresh its runtime pidfile", status.name),
        "missing-log" => format!("restart {} or inspect its launch path", status.name),
        "unreadable-log" => format!("check permissions for {}", surface_log_display(status)),
        "empty-log" => format!(
            "inspect {} if {} is misbehaving",
            surface_log_display(status),
            status.name
        ),
        _ => format!("inspect {}", status.name),
    }
}

pub(super) fn surface_doctor_recommended_command(status: &SurfaceStatus) -> Option<String> {
    let issue = surface_doctor_issue(status);
    let issue = if issue == "none" {
        surface_doctor_log_issue(status)
    } else {
        issue
    };
    match issue {
        "stale-pidfile" => Some(format!("hbctl surfaces clean {}", status.name)),
        "missing-pidfile" => Some(format!("hbctl surfaces restart {}", status.name)),
        "missing-log" => Some(format!("hbctl surfaces restart {}", status.name)),
        _ => None,
    }
}

fn surface_pid_file_display(status: &SurfaceStatus) -> &str {
    status.pid_file.as_deref().unwrap_or("the surface pidfile")
}

fn surface_log_display(status: &SurfaceStatus) -> &str {
    status.log_path.as_deref().unwrap_or("the surface log")
}

fn print_surface_doctor_report(report: &SurfaceDoctorReport) {
    println!("{}", surface_doctor_summary(report));
    if report.surfaces.is_empty() && report.filtered {
        println!("no surfaces matched the active doctor filters");
        return;
    }
    println!(
        "surface  health     severity  state    pidfile  log      issue               log_issue           recommendation"
    );
    for entry in &report.surfaces {
        println!(
            "{:<8} {:<10} {:<9} {:<8} {:<8} {:<8} {:<19} {:<19} {}",
            entry.name,
            if entry.healthy {
                "healthy"
            } else {
                "unhealthy"
            },
            entry.severity,
            entry.state,
            entry.pid_file_state,
            entry.log_state,
            entry.issue,
            entry.log_issue,
            entry.recommendation
        );
    }
}

fn print_surface_doctor_commands(report: &SurfaceDoctorReport) {
    for command in &report.recommended_commands {
        println!("{command}");
    }
}

pub(super) fn surface_doctor_summary(report: &SurfaceDoctorReport) -> String {
    format!(
        "summary target={} mode={} healthy={} selected={} healthy_count={} issues={} warnings={} errors={} actionable={} displayed={} commands={} filtered={} zero_ok={} severity_filter={} filters={}",
        report.target,
        report.output_mode,
        report.healthy,
        report.selected,
        report.healthy_count,
        report.issues,
        report.warnings,
        report.errors,
        report.actionable,
        report.displayed,
        report.command_count,
        report.filtered,
        report.zero_ok,
        report.severity_filter,
        surface_doctor_summary_filters(report)
    )
}

fn surface_doctor_summary_filters(report: &SurfaceDoctorReport) -> String {
    if report.active_filters.is_empty() {
        "none".to_string()
    } else {
        report.active_filters.join(",")
    }
}
