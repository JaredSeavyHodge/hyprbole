use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(0);

fn hbctl(args: &[&str]) -> Output {
    let runtime = isolated_runtime_dir();
    let output = Command::new(env!("CARGO_BIN_EXE_hbctl"))
        .args(args)
        .env("XDG_RUNTIME_DIR", &runtime)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&runtime).unwrap();
    output
}

fn isolated_runtime_dir() -> std::path::PathBuf {
    loop {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed);
        let runtime = std::env::temp_dir().join(format!(
            "hyprbole-hbctl-test-{}-{sequence}-{nonce}",
            std::process::id()
        ));
        if std::fs::create_dir(&runtime).is_ok() {
            return runtime;
        }
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn surfaces_plain_routes_to_status_output() {
    let output = hbctl(&["surfaces", "--plain"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("surface  state"));
    assert!(stdout.contains("quick"));
    assert!(stdout.contains("bar"));
    assert!(stdout.contains("osd"));
    assert!(stdout.contains("launcher"));
}

#[test]
fn surfaces_json_routes_to_status_output() {
    let output = hbctl(&["surfaces"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 4);
    assert_eq!(json[0]["name"], "quick");
    assert_eq!(json[1]["name"], "bar");
    assert_eq!(json[2]["name"], "osd");
    assert_eq!(json[3]["name"], "launcher");
}

#[test]
fn surfaces_check_json_routes_and_reports_success() {
    // Use a pidfile predicate so live process fallback cannot affect the result.
    let output = hbctl(&["surfaces", "check", "all", "pidfile-missing", "--json"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["target"], "all");
    assert_eq!(json["expected"], "pidfile-missing");
    assert_eq!(json["passed"], true);
    assert_eq!(json["surfaces"].as_array().unwrap().len(), 4);
}

#[test]
fn surfaces_wait_json_routes_and_reports_success() {
    // Use a pidfile predicate so live process fallback cannot affect the result.
    let output = hbctl(&[
        "surfaces",
        "wait",
        "all",
        "pidfile-missing",
        "--timeout-ms",
        "50",
        "--json",
    ]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["target"], "all");
    assert_eq!(json["expected"], "pidfile-missing");
    assert_eq!(json["passed"], true);
}

#[test]
fn surfaces_doctor_json_routes_with_zero_ok() {
    let output = hbctl(&["surfaces", "doctor", "--zero-ok", "--json"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["target"], "all");
    assert_eq!(json["output_mode"], "json");
    assert_eq!(json["zero_ok"], true);
    assert_eq!(json["selected"], 4);
}

#[test]
fn surfaces_clean_dry_run_routes_without_side_effects() {
    let output = hbctl(&["surfaces", "clean", "all", "--dry-run"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("quick pidfile already missing"));
    assert!(stdout.contains("bar pidfile already missing"));
    assert!(stdout.contains("osd pidfile already missing"));
    assert!(stdout.contains("launcher pidfile already missing"));
}

#[test]
fn surfaces_launcher_target_routes_to_status_output() {
    let output = hbctl(&["surfaces", "launcher", "--plain"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("launcher"));
    assert!(!stdout.contains("quick"));
}

#[test]
fn surfaces_side_effect_verbs_route_to_usage_before_dispatch_when_incomplete() {
    for args in [
        ["surfaces", "start"].as_slice(),
        ["surfaces", "stop"].as_slice(),
        ["surfaces", "restart"].as_slice(),
    ] {
        let output = hbctl(args);

        assert_eq!(output.status.code(), Some(2));
        assert!(stderr(&output).contains("usage: hbctl surfaces"));
    }
}

#[test]
fn dev_restart_all_usage_rejects_unknown_args() {
    let output = hbctl(&["dev"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("usage: hbctl dev restart-all"));

    let output = hbctl(&["dev", "restart-all", "--foreground"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("usage: hbctl dev restart-all"));
}

#[test]
fn lifecycle_commands_reject_toggle_restart_conflict() {
    for args in [
        ["quick", "--toggle", "--restart"].as_slice(),
        ["osd", "--toggle", "--restart"].as_slice(),
        ["bar", "--toggle", "--restart"].as_slice(),
        ["launcher", "--layer", "--toggle", "--restart"].as_slice(),
    ] {
        let output = hbctl(args);
        assert_eq!(output.status.code(), Some(2));
        assert!(stderr(&output).contains("only one of --toggle or --restart"));
    }
}

#[test]
fn launcher_stdin_requires_foreground_and_layer_mode() {
    let output = hbctl(&["launcher", "--stdin"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--stdin requires the layer launcher"));

    let output = hbctl(&["launcher", "--dev", "--stdin", "--foreground"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--stdin requires the layer launcher"));
}

#[test]
fn launcher_layer_contract_rejects_conflicting_modes() {
    let output = hbctl(&["launcher", "--layer", "--gtk"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("cannot be combined"));

    let output = hbctl(&["launcher", "--dev", "--gtk"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("only one of --dev or --gtk"));

    let output = hbctl(&["launcher", "--layer", "--stdin", "--foreground", "--toggle"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--stdin does not support"));
}

#[test]
fn launcher_lifecycle_requires_layer_mode() {
    let output = hbctl(&["launcher", "--dev", "--toggle"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("require the layer launcher"));
}

#[test]
fn launcher_layer_options_require_layer_mode_and_values() {
    let output = hbctl(&["launcher", "--dev", "--prompt", "Run"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("require the layer launcher"));

    let output = hbctl(&["launcher", "--prompt"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--prompt requires a value"));

    let output = hbctl(&["launcher", "--prompt", "--foreground"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--prompt requires a value"));

    let output = hbctl(&["launcher", "--lines", "0"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("positive integer"));
}
