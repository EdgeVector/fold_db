//! End-to-end integration tests for the `folddb` CLI binary.
//!
//! Each test spawns the real binary with `--json` mode, a temp data directory,
//! and `mock://test` as the schema service URL so no network access is needed.
//!
//! Run with:
//!   cargo test --test cli_integration_test -- --nocapture

// Suppress cargo_bin deprecation warning (it still works; the new macro is
// for custom build-dir setups which we don't use).
#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a temp directory and write a node config with generated identity keys.
/// Returns `(tmpdir, config_path)`. The `TempDir` must be kept alive for the
/// duration of the test so the directory isn't cleaned up early.
fn setup() -> (TempDir, PathBuf) {
    let tmpdir = TempDir::new().expect("create temp dir");
    let keypair = fold_db::security::Ed25519KeyPair::generate().expect("generate keypair");

    let db_path = tmpdir.path().join("db");
    let config = serde_json::json!({
        "database": {
            "type": "local",
            "path": db_path.to_str().unwrap()
        },
        "network_listen_address": "/ip4/0.0.0.0/tcp/0",
        "security_config": {
            "require_tls": false,
            "encrypt_at_rest": false
        },
        "schema_service_url": "mock://test",
        "public_key": keypair.public_key_base64(),
        "private_key": keypair.secret_key_base64()
    });

    let config_path = tmpdir.path().join("node_config.json");
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&config).unwrap(),
    )
    .expect("write config");

    (tmpdir, config_path)
}

/// Return a `Command` pre-configured with `--json --config <path>`.
fn cli(config_path: &Path) -> Command {
    let mut cmd = Command::cargo_bin("folddb").expect("find folddb binary");
    cmd.arg("--json").arg("--config").arg(config_path);
    cmd
}

/// Parse the stdout of a command as JSON. Panics with a clear message on
/// failure.
fn parse_stdout(output: &std::process::Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "Failed to parse JSON from stdout: {}\nstdout: {}\nstderr: {}",
            e,
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn status_returns_ok() {
    let (_tmpdir, config_path) = setup();

    let output = cli(&config_path)
        .arg("status")
        .output()
        .expect("run folddb status");

    assert!(output.status.success(), "exit code was not 0");

    let json = parse_stdout(&output);
    assert_eq!(json["ok"], true);
    assert!(json["node_public_key"].is_string());
    assert!(!json["node_public_key"].as_str().unwrap().is_empty());
    assert!(json["user_hash"].is_string());
    assert!(!json["user_hash"].as_str().unwrap().is_empty());
    assert!(json["database_config"].is_object());
    assert!(json["indexing_status"].is_object());
}

#[test]
fn config_show() {
    let (tmpdir, config_path) = setup();

    let output = cli(&config_path)
        .arg("config")
        .arg("show")
        .output()
        .expect("run folddb config show");

    assert!(output.status.success(), "exit code was not 0");

    let json = parse_stdout(&output);
    assert_eq!(json["ok"], true);

    let config = &json["config"];
    assert_eq!(config["type"], "local");
    // The path should point inside our temp dir
    let path_str = config["path"].as_str().expect("config.path is string");
    assert!(
        path_str.contains(tmpdir.path().to_str().unwrap()),
        "config path '{}' should contain tmpdir '{}'",
        path_str,
        tmpdir.path().display()
    );
}

#[test]
fn config_path() {
    let (_tmpdir, config_path) = setup();

    let output = cli(&config_path)
        .arg("config")
        .arg("path")
        .output()
        .expect("run folddb config path");

    assert!(output.status.success(), "exit code was not 0");

    let json = parse_stdout(&output);
    assert_eq!(json["ok"], true);
    // The path should match the --config value we provided
    let path_str = json["path"].as_str().expect("path field is string");
    assert_eq!(path_str, config_path.to_str().unwrap());
}

#[test]
fn schema_list_empty() {
    let (_tmpdir, config_path) = setup();

    let output = cli(&config_path)
        .arg("schema")
        .arg("list")
        .output()
        .expect("run folddb schema list");

    assert!(output.status.success(), "exit code was not 0");

    let json = parse_stdout(&output);
    assert_eq!(json["ok"], true);
    assert_eq!(json["schemas"], serde_json::json!([]));
}

#[test]
fn schema_get_not_found() {
    let (_tmpdir, config_path) = setup();

    cli(&config_path)
        .arg("schema")
        .arg("get")
        .arg("NonExistentSchema")
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"))
        .stdout(predicate::str::contains("NonExistentSchema"));
}

#[test]
fn schema_load_with_mock_fails() {
    let (_tmpdir, config_path) = setup();

    cli(&config_path)
        .arg("schema")
        .arg("load")
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"));
}

#[test]
fn search_empty() {
    let (_tmpdir, config_path) = setup();

    let output = cli(&config_path)
        .arg("search")
        .arg("nothing_will_match")
        .output()
        .expect("run folddb search");

    assert!(output.status.success(), "exit code was not 0");

    let json = parse_stdout(&output);
    assert_eq!(json["ok"], true);
    assert_eq!(json["results"], serde_json::json!([]));
}

#[test]
fn reset_requires_confirm_in_json_mode() {
    let (_tmpdir, config_path) = setup();

    cli(&config_path)
        .arg("reset")
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"))
        .stdout(predicate::str::contains("--confirm"));
}

#[test]
fn reset_with_confirm() {
    let (_tmpdir, config_path) = setup();

    let output = cli(&config_path)
        .arg("reset")
        .arg("--confirm")
        .output()
        .expect("run folddb reset --confirm");

    assert!(output.status.success(), "exit code was not 0");

    let json = parse_stdout(&output);
    assert_eq!(json["ok"], true);
    assert_eq!(json["message"], "Database reset complete");
}

#[test]
fn completions_bash() {
    let (_tmpdir, config_path) = setup();

    // Completions are tested without --json because the JSON renderer
    // discards the actual script and only outputs a status envelope.
    Command::cargo_bin("folddb")
        .expect("find folddb binary")
        .arg("--config")
        .arg(&config_path)
        .arg("completions")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("folddb"));
}

#[test]
fn mutate_nonexistent_schema_fails() {
    let (_tmpdir, config_path) = setup();

    cli(&config_path)
        .arg("mutate")
        .arg("run")
        .arg("FakeSchema")
        .arg("--type")
        .arg("create")
        .arg("--fields")
        .arg(r#"{"name":"test"}"#)
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"));
}

#[test]
fn mutate_invalid_json_fields_fails() {
    let (_tmpdir, config_path) = setup();

    cli(&config_path)
        .arg("mutate")
        .arg("run")
        .arg("SomeSchema")
        .arg("--type")
        .arg("create")
        .arg("--fields")
        .arg("not-valid-json")
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"))
        .stdout(predicate::str::contains("Invalid fields JSON"));
}

#[test]
fn query_nonexistent_schema_fails() {
    let (_tmpdir, config_path) = setup();

    cli(&config_path)
        .arg("query")
        .arg("NoSuchSchema")
        .arg("--fields")
        .arg("some_field")
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"));
}
