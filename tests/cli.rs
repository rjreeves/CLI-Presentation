//! End-to-end tests against the compiled `presentation` binary — these
//! exercise the CLI dispatch/wiring in main.rs (subcommands, schema
//! auto-selection, --live/--export/--output) that the in-module unit
//! tests under src/ can't reach, since that logic lives in the binary
//! entry point rather than a testable library function.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn run(args: &[&str], stdin_data: &str) -> (String, String, bool) {
    run_with_env(args, stdin_data, &[])
}

fn run_with_env(args: &[&str], stdin_data: &str, env: &[(&str, &str)]) -> (String, String, bool) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_presentation"));
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("failed to spawn presentation binary");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin_data.as_bytes())
        .unwrap();
    let output = child.wait_with_output().expect("failed to wait on child");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

/// A fresh, isolated `$HOME`/`%USERPROFILE%` for tests that touch the user
/// schema registry, so they never read or write the real
/// `~/.presentation/registry.json` and never see state left by another
/// test run.
fn fresh_fake_home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("presentation-cli-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn with_fake_home(home: &Path) -> [(&str, &str); 2] {
    let home = home.to_str().unwrap();
    [("HOME", home), ("USERPROFILE", home)]
}

const NETWORK_IP: &str = r#"{"schema":"network.ip","data":[{"interface":"eth0","gateway":"1.1.1.1","dns":["8.8.8.8"],"status":"Up"}]}"#;
const DOCKER_STATS: &str = r#"{"schema":"docker.stats","data":[{"name":"web","cpu_percent":40}]}"#;
const LOGS_EVENTS: &str =
    r#"{"schema":"logs.events","data":[{"timestamp":"10:00","level":"info","message":"started"}]}"#;
const SYSTEM_PERF_CPU: &str =
    r#"{"schema":"system.perf.cpu","data":[{"timestamp":"t1","usage":10.0}]}"#;
const K8S_PODS: &str = r#"{"schema":"k8s.pods","data":[{"name":"pod-a","status":"Running"}]}"#;
const PLAIN: &str = r#"{"data":[{"a":1,"b":2}]}"#;

// ---- golden path per explicit mode ----

#[test]
fn table_mode_renders_a_table() {
    let (out, _, ok) = run(&["table"], PLAIN);
    assert!(ok);
    assert!(out.contains("a | b"));
}

#[test]
fn dashboard_mode_renders_panels() {
    let (out, _, ok) = run(
        &["dashboard"],
        r#"{"data":[{"name":"web","cpu_percent":50}]}"#,
    );
    assert!(ok);
    assert!(out.contains("┌─ web"));
    assert!(out.contains('%'));
}

#[test]
fn chart_mode_renders_bars_by_default() {
    let (out, _, ok) = run(&["chart"], r#"{"data":[{"name":"requests","value":10}]}"#);
    assert!(ok);
    assert!(out.contains("requests"));
}

#[test]
fn chart_mode_respects_chart_type_flag() {
    let (out, _, ok) = run(
        &["chart", "--chart-type", "pie"],
        r#"{"data":[{"name":"a","value":1}]}"#,
    );
    assert!(ok);
    assert!(out.starts_with('['));
}

#[test]
fn timeline_mode_renders_chronologically() {
    let data = r#"{"data":[{"timestamp":"10:05","message":"later"},{"timestamp":"10:00","message":"earlier"}]}"#;
    let (out, _, ok) = run(&["timeline"], data);
    assert!(ok);
    assert!(out.find("earlier").unwrap() < out.find("later").unwrap());
}

#[test]
fn map_mode_renders_a_tree() {
    let (out, _, ok) = run(&["map"], NETWORK_IP);
    assert!(ok);
    assert!(out.starts_with("PC\n"));
    assert!(out.contains("eth0"));
}

#[test]
fn html_mode_renders_a_document() {
    let (out, _, ok) = run(&["html"], PLAIN);
    assert!(ok);
    assert!(out.contains("<!doctype html>"));
    assert!(out.contains("<table>"));
}

#[test]
fn md_mode_renders_a_table() {
    let (out, _, ok) = run(&["md"], PLAIN);
    assert!(ok);
    assert!(out.contains("| a | b |"));
}

#[test]
fn explain_mode_describes_the_data() {
    let (out, _, ok) = run(&["explain"], r#"{"data":[{"status":"up"}]}"#);
    assert!(ok);
    assert!(out.contains("record(s)"));
}

#[test]
fn ai_summary_mode_is_a_single_condensed_line() {
    let (out, _, ok) = run(&["ai-summary"], r#"{"data":[{"status":"up"}]}"#);
    assert!(ok);
    assert_eq!(out.lines().count(), 1);
}

// ---- schema-based auto-selection ----

#[test]
fn auto_selects_dashboard_for_docker_stats() {
    let (out, _, ok) = run(&[], DOCKER_STATS);
    assert!(ok);
    assert!(out.contains("┌─"));
}

#[test]
fn auto_selects_map_for_network_ip() {
    let (out, _, ok) = run(&[], NETWORK_IP);
    assert!(ok);
    assert!(out.starts_with("PC\n"));
}

#[test]
fn auto_selects_timeline_for_logs_events() {
    let (out, _, ok) = run(&[], LOGS_EVENTS);
    assert!(ok);
    assert!(out.contains("├─") || out.contains("└─"));
}

#[test]
fn auto_selects_chart_for_system_perf_wildcard() {
    let (out, _, ok) = run(&[], SYSTEM_PERF_CPU);
    assert!(ok);
    assert!(out.contains("t1"));
}

#[test]
fn auto_selects_table_for_k8s_pods() {
    let (out, _, ok) = run(&[], K8S_PODS);
    assert!(ok);
    assert!(out.contains("pod-a"));
}

#[test]
fn explicit_mode_overrides_schema_auto_selection() {
    let (out, _, ok) = run(&["table"], DOCKER_STATS);
    assert!(ok);
    // Explicit table mode must win over docker.stats' dashboard default.
    assert!(out.contains("cpu_percent | name"));
}

#[test]
fn unknown_schema_warns_and_falls_back_to_table() {
    let (out, err, ok) = run(&[], r#"{"schema":"totally.unknown","data":[{"a":1}]}"#);
    assert!(ok);
    assert!(err.contains("unknown schema"));
    assert!(out.contains('a'));
}

// ---- sort / filter / width ----

#[test]
fn sort_orders_rows_numerically() {
    let data = r#"{"data":[{"v":3},{"v":1},{"v":2}]}"#;
    let (out, _, ok) = run(&["table", "--sort", "v"], data);
    assert!(ok);
    let lines: Vec<&str> = out.lines().skip(2).collect(); // skip header + divider
    assert_eq!(lines, vec!["1", "2", "3"]);
}

#[test]
fn filter_keeps_only_matching_rows() {
    let data = r#"{"data":[{"status":"up"},{"status":"down"}]}"#;
    let (out, _, ok) = run(&["table", "--filter", "status=down"], data);
    assert!(ok);
    assert!(!out.contains("up") || out.matches("down").count() >= out.matches("up").count());
    assert!(out.contains("down"));
    assert_eq!(out.lines().count(), 3); // header + divider + one row
}

// ---- --output ----

#[test]
fn output_flag_writes_to_a_file_instead_of_stdout() {
    let path = std::env::temp_dir().join("presentation-cli-test-output.txt");
    let _ = std::fs::remove_file(&path);
    let (out, _, ok) = run(&["table", "--output", path.to_str().unwrap()], PLAIN);
    assert!(ok);
    assert_eq!(out, "");
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("a | b"));
    let _ = std::fs::remove_file(&path);
}

// ---- --export ----

#[test]
fn export_html_wraps_any_modes_output() {
    let (out, _, ok) = run(&["table", "--export", "html"], PLAIN);
    assert!(ok);
    assert!(out.contains("<!doctype html>"));
    assert!(out.contains("<pre>"));
}

#[test]
fn export_md_wraps_in_a_fenced_code_block() {
    let (out, _, ok) = run(&["table", "--export", "md"], PLAIN);
    assert!(ok);
    assert!(out.starts_with("```text\n"));
}

#[test]
fn export_svg_produces_an_svg_document() {
    let (out, _, ok) = run(&["table", "--export", "svg"], PLAIN);
    assert!(ok);
    assert!(out.starts_with("<svg "));
}

#[test]
fn export_pdf_fails_cleanly() {
    let (_, err, ok) = run(&["table", "--export", "pdf"], PLAIN);
    assert!(!ok);
    assert!(err.contains("not implemented"));
}

// ---- --live ----

#[test]
fn live_dashboard_renders_every_frame() {
    let frames = "{\"data\":[{\"name\":\"web\",\"cpu_percent\":10}]}\n{\"data\":[{\"name\":\"web\",\"cpu_percent\":90}]}\n";
    let (out, _, ok) = run(&["dashboard", "--live"], frames);
    assert!(ok);
    assert_eq!(out.matches("┌─ web").count(), 2);
    assert!(out.contains("10%"));
    assert!(out.contains("90%"));
}

#[test]
fn live_with_unsupported_mode_warns_and_renders_once() {
    let frames = "{\"data\":[{\"a\":1}]}\n{\"data\":[{\"a\":2}]}\n";
    let (out, err, ok) = run(&["table", "--live"], frames);
    assert!(ok);
    assert!(err.contains("only supports dashboard/timeline"));
    assert_eq!(out.matches('a').count(), 1); // header only, one frame rendered
}

#[test]
fn live_with_empty_stdin_exits_cleanly() {
    let (out, _, ok) = run(&["dashboard", "--live"], "");
    assert!(ok);
    assert_eq!(out, "");
}

// ---- error handling ----

#[test]
fn invalid_json_input_fails() {
    let (_, _, ok) = run(&["table"], "not json");
    assert!(!ok);
}

#[test]
fn missing_data_field_fails() {
    let (_, _, ok) = run(&["table"], r#"{"schema": "x"}"#);
    assert!(!ok);
}

// ---- register / validate ----

#[test]
fn validate_with_no_schema_is_always_ok() {
    let (out, _, ok) = run(&["validate"], PLAIN);
    assert!(ok);
    assert!(out.contains("No schema specified"));
}

#[test]
fn validate_unknown_schema_fails() {
    let home = fresh_fake_home("validate-unknown");
    let (_, err, ok) = run_with_env(
        &["validate"],
        r#"{"schema":"nope.nope","data":[]}"#,
        &with_fake_home(&home),
    );
    assert!(!ok);
    assert!(err.contains("SRD_SCHEMA_NOT_FOUND"));
}

#[test]
fn validate_builtin_schema_flags_missing_required_field() {
    let home = fresh_fake_home("validate-builtin");
    let data = r#"{"schema":"network.ip","data":[{"ipv4":"1.2.3.4"}]}"#;
    let (out, _, ok) = run_with_env(&["validate"], data, &with_fake_home(&home));
    assert!(ok); // warnings are non-fatal
    assert!(out.contains("missing required field 'interface'"));
}

#[test]
fn register_then_validate_and_render_round_trip() {
    let home = fresh_fake_home("register-roundtrip");
    let env = with_fake_home(&home);

    let fields_path = home.join("fields.json");
    std::fs::write(&fields_path, r#"{"name": "string", "value": "number"}"#).unwrap();

    let (reg_out, _, reg_ok) = run_with_env(
        &[
            "register",
            "--schema",
            "acme.widgets",
            "--renderer",
            "chart",
            "--fields",
            fields_path.to_str().unwrap(),
        ],
        "",
        &env,
    );
    assert!(reg_ok);
    assert!(reg_out.contains("Registered schema 'acme.widgets' -> chart"));

    let payload = r#"{"schema":"acme.widgets","data":[{"name":"requests","value":100}]}"#;

    let (val_out, _, val_ok) = run_with_env(&["validate"], payload, &env);
    assert!(val_ok);
    assert!(val_out.contains("match schema 'acme.widgets'"));
    assert!(val_out.contains("user-registered"));

    // The registered schema must actually drive rendering, not just validate.
    let (render_out, _, render_ok) = run_with_env(&[], payload, &env);
    assert!(render_ok);
    assert!(render_out.contains("requests")); // chart mode renders the label
}

#[test]
fn corrupt_registry_file_fails_validate_but_not_rendering() {
    let home = fresh_fake_home("corrupt-registry");
    let env = with_fake_home(&home);
    let registry_path = home.join(".presentation").join("registry.json");
    std::fs::create_dir_all(registry_path.parent().unwrap()).unwrap();
    std::fs::write(&registry_path, "not json").unwrap();

    let (_, err, val_ok) = run_with_env(&["validate"], NETWORK_IP, &env);
    assert!(!val_ok);
    assert!(err.contains("corrupt"));

    // Rendering must still work: a broken user registry falls back to the
    // built-in table rather than breaking normal use.
    let (render_out, _, render_ok) = run_with_env(&["map"], NETWORK_IP, &env);
    assert!(render_ok);
    assert!(render_out.starts_with("PC\n"));
}
