use assert_cmd::Command;
use predicates::prelude::*;

fn neo_cmd() -> Command {
    let mut cmd = Command::cargo_bin("neo").unwrap();
    cmd.timeout(std::time::Duration::from_secs(30));
    cmd
}

#[test]
fn test_version() {
    neo_cmd()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("neo 0.1.0"))
        .stdout(predicate::str::contains("Platform:"));
}

#[test]
fn test_help() {
    neo_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Neo AGI Operating System"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("shell"))
        .stdout(predicate::str::contains("chat"))
        .stdout(predicate::str::contains("server"))
        .stdout(predicate::str::contains("daemon"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("benchmark"))
        .stdout(predicate::str::contains("config"))
        .stdout(predicate::str::contains("models"))
        .stdout(predicate::str::contains("memory"))
        .stdout(predicate::str::contains("graph"))
        .stdout(predicate::str::contains("reasoning"));
}

#[test]
fn test_version_subcommand_help() {
    neo_cmd()
        .args(["version", "--help"])
        .assert()
        .success();
}

#[test]
fn test_config_show() {
    neo_cmd()
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[core]"))
        .stdout(predicate::str::contains("[logging]"))
        .stdout(predicate::str::contains("[network]"));
}

#[test]
fn test_config_validate() {
    neo_cmd()
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("core.environment"))
        .stdout(predicate::str::contains("Configuration is valid"));
}

#[test]
fn test_models() {
    neo_cmd()
        .arg("models")
        .assert()
        .success()
        .stdout(predicate::str::contains("Loaded Models"));
}

#[test]
fn test_doctor() {
    neo_cmd()
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Neo System Doctor"))
        .stdout(predicate::str::contains("Configuration"))
        .stdout(predicate::str::contains("Runtime"))
        .stdout(predicate::str::contains("Executive"))
        .stdout(predicate::str::contains("Memory"))
        .stdout(predicate::str::contains("Knowledge Graph"))
        .stdout(predicate::str::contains("Reasoning"))
        .stdout(predicate::str::contains("All checks passed"));
}

#[test]
fn test_status() {
    neo_cmd()
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("System Status"))
        .stdout(predicate::str::contains("Runtime:"))
        .stdout(predicate::str::contains("Modules:"))
        .stdout(predicate::str::contains("Executive:"));
}

#[test]
fn test_benchmark_quick() {
    neo_cmd()
        .args(["benchmark", "--duration-secs", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Benchmarking for 2s"))
        .stdout(predicate::str::contains("Results"));
}

#[test]
fn test_config_init() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join("neo.toml");

    neo_cmd()
        .args(["config", "init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(config_path.exists());
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[core]"));
    assert!(content.contains("[logging]"));
}

#[test]
fn test_cli_no_subcommand_shows_help() {
    neo_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn test_config_set_shows_note() {
    neo_cmd()
        .args(["config", "set", "core.environment", "production"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Setting"))
        .stdout(predicate::str::contains("Note:"));
}

#[test]
fn test_memory_stats() {
    neo_cmd()
        .args(["memory", "stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Memory Statistics"));
}

#[test]
fn test_graph_stats() {
    neo_cmd()
        .args(["graph", "stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Knowledge Graph Statistics"));
}

#[test]
fn test_graph_entities() {
    neo_cmd()
        .args(["graph", "entities"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Entities:"));
}

#[test]
fn test_log_level_override() {
    neo_cmd()
        .env("NEO_LOG_LEVEL", "warn")
        .args(["version"])
        .assert()
        .success();
}

#[test]
fn test_config_env_override() {
    neo_cmd()
        .env("NEO_ENVIRONMENT", "test")
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test"));
}

#[test]
fn test_reasoning_query() {
    neo_cmd()
        .args(["reasoning", "what is 2+2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reasoning:"));
}
