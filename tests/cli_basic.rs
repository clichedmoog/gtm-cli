use assert_cmd::Command;
use predicates::prelude::*;

fn gtm() -> Command {
    Command::cargo_bin("gtm").expect("binary exists")
}

#[test]
fn test_help() {
    gtm()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Google Tag Manager CLI"))
        .stdout(predicate::str::contains("tags"))
        .stdout(predicate::str::contains("triggers"))
        .stdout(predicate::str::contains("variables"));
}

#[test]
fn test_version() {
    gtm()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("gtm"));
}

#[test]
fn test_subcommand_help() {
    let subcommands = [
        "accounts",
        "containers",
        "workspaces",
        "tags",
        "triggers",
        "variables",
        "folders",
        "templates",
        "versions",
        "version-headers",
        "environments",
        "permissions",
        "clients",
        "gtag-configs",
        "transformations",
        "zones",
        "builtin-variables",
        "setup",
        "completions",
    ];
    for sub in subcommands {
        gtm()
            .args([sub, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Usage:"));
    }
}

#[test]
fn test_completions_bash() {
    gtm()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_gtm"));
}

#[test]
fn test_completions_zsh() {
    gtm()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef gtm"));
}

#[test]
fn test_completions_fish() {
    gtm()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_unknown_subcommand() {
    gtm().arg("nonexistent").assert().failure();
}

#[test]
fn test_missing_required_flags() {
    // tags list requires --account-id and --container-id
    gtm()
        .args(["tags", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--account-id"));
}

#[test]
fn test_invalid_format() {
    gtm()
        .args(["--format", "xml", "accounts", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_format_values_accepted() {
    // These should parse correctly (will fail on auth, not on flag parsing)
    for format in ["json", "table"] {
        let result = gtm()
            .args(["--format", format, "accounts", "list"])
            .assert();
        // Should NOT fail with "invalid value" — may fail with auth error which is fine
        let output = result.get_output();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("invalid value"),
            "format '{format}' should be accepted"
        );
    }
}

#[test]
fn test_dry_run_flag_accepted() {
    // --dry-run should be accepted as a global flag
    let result = gtm().args(["--dry-run", "--help"]).assert();
    result.success().stdout(predicate::str::contains("dry-run"));
}
