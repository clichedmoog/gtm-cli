//! Integration tests that call the real GTM API.
//! Run with: cargo test --test integration -- --ignored --test-threads=1
//!
//! Requires valid credentials at ~/.config/gtm/ and the test container:
//! Account: 6343477577, Container: 245887900
//!
//! NOTE: Must run single-threaded to avoid GTM API rate limits (429).

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

const ACCOUNT_ID: &str = "6343477577";
const CONTAINER_ID: &str = "245887900";

fn gtm() -> Command {
    let mut cmd = Command::cargo_bin("gtm").expect("binary exists");
    cmd.args(["--format", "json"]);
    cmd
}

fn gtm_table() -> Command {
    let mut cmd = Command::cargo_bin("gtm").expect("binary exists");
    cmd.args(["--format", "table"]);
    cmd
}

fn parse_json(output: &assert_cmd::assert::Assert) -> Value {
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    serde_json::from_str(&stdout).expect("should be valid JSON")
}

fn unique_suffix() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{ts}")
}

// ─── Accounts ───

#[test]
#[ignore]
fn test_accounts_list() {
    let assert = gtm().args(["accounts", "list"]).assert().success();
    let json = parse_json(&assert);
    assert!(json["account"].is_array());
    assert!(json["account"].as_array().unwrap().len() > 0);
}

#[test]
#[ignore]
fn test_accounts_list_table() {
    gtm_table()
        .args(["accounts", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID"))
        .stdout(predicate::str::contains("Name"));
}

#[test]
#[ignore]
fn test_accounts_get() {
    let assert = gtm()
        .args(["accounts", "get", "--account-id", ACCOUNT_ID])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["accountId"], ACCOUNT_ID);
}

// ─── Containers ───

#[test]
#[ignore]
fn test_containers_list() {
    let assert = gtm()
        .args(["containers", "list", "--account-id", ACCOUNT_ID])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json["container"].is_array());
}

#[test]
#[ignore]
fn test_containers_list_table() {
    gtm_table()
        .args(["containers", "list", "--account-id", ACCOUNT_ID])
        .assert()
        .success()
        .stdout(predicate::str::contains(CONTAINER_ID));
}

#[test]
#[ignore]
fn test_containers_get() {
    let assert = gtm()
        .args([
            "containers",
            "get",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["containerId"], CONTAINER_ID);
}

// ─── Workspaces ───

#[test]
#[ignore]
fn test_workspaces_list() {
    let assert = gtm()
        .args([
            "workspaces",
            "list",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json["workspace"].is_array());
}

// ─── Full CRUD: Tag + Trigger + Variable + Folder ───

#[test]
#[ignore]
fn test_crud_lifecycle() {
    let suffix = unique_suffix();

    // 1. Create trigger
    let trigger_name = format!("Test Trigger {suffix}");
    let assert = gtm()
        .args([
            "triggers",
            "create",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--name",
            &trigger_name,
            "--type",
            "pageview",
        ])
        .assert()
        .success();
    let trigger = parse_json(&assert);
    let trigger_id = trigger["triggerId"].as_str().unwrap();

    // 2. Create tag with that trigger
    let tag_name = format!("Test Tag {suffix}");
    let assert = gtm()
        .args([
            "tags",
            "create",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--name",
            &tag_name,
            "--type",
            "html",
            "--firing-trigger-id",
            trigger_id,
            "--params",
            r#"{"html":"<script>/* test */</script>"}"#,
        ])
        .assert()
        .success();
    let tag = parse_json(&assert);
    let tag_id = tag["tagId"].as_str().unwrap();

    // 3. Create variable
    let var_name = format!("Test Variable {suffix}");
    let assert = gtm()
        .args([
            "variables",
            "create",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--name",
            &var_name,
            "--type",
            "c",
            "--value",
            "test-value",
        ])
        .assert()
        .success();
    let variable = parse_json(&assert);
    let variable_id = variable["variableId"].as_str().unwrap();

    // 4. Create folder
    let folder_name = format!("Test Folder {suffix}");
    let assert = gtm()
        .args([
            "folders",
            "create",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--name",
            &folder_name,
        ])
        .assert()
        .success();
    let folder = parse_json(&assert);
    let folder_id = folder["folderId"].as_str().unwrap();

    // 5. Move entities to folder
    gtm()
        .args([
            "folders",
            "move-entities",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--folder-id",
            folder_id,
            "--tag-id",
            tag_id,
            "--variable-id",
            variable_id,
        ])
        .assert()
        .success();

    // 6. Verify folder entities
    let assert = gtm()
        .args([
            "folders",
            "entities",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--folder-id",
            folder_id,
        ])
        .assert()
        .success();
    let entities = parse_json(&assert);
    assert!(entities["tag"].is_array());
    assert!(entities["variable"].is_array());

    // 7. Update tag
    let updated_name = format!("Test Tag Updated {suffix}");
    let assert = gtm()
        .args([
            "tags",
            "update",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--tag-id",
            tag_id,
            "--name",
            &updated_name,
        ])
        .assert()
        .success();
    let updated = parse_json(&assert);
    assert_eq!(updated["name"].as_str().unwrap(), updated_name);

    // 8. List tags in table format — should show updated name
    gtm_table()
        .args([
            "tags",
            "list",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(&updated_name));

    // 9. Cleanup: delete tag, trigger, variable, folder
    gtm()
        .args([
            "tags",
            "delete",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--tag-id",
            tag_id,
        ])
        .assert()
        .success();
    gtm()
        .args([
            "triggers",
            "delete",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--trigger-id",
            trigger_id,
        ])
        .assert()
        .success();
    gtm()
        .args([
            "variables",
            "delete",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--variable-id",
            variable_id,
        ])
        .assert()
        .success();
    gtm()
        .args([
            "folders",
            "delete",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--folder-id",
            folder_id,
        ])
        .assert()
        .success();
}

// ─── Built-in Variables ───

#[test]
#[ignore]
fn test_builtin_variables_lifecycle() {
    // Use a less commonly enabled variable to avoid conflicts
    let var_type = "containerVersion";

    // Enable
    gtm()
        .args([
            "builtin-variables",
            "create",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--type",
            var_type,
        ])
        .assert()
        .success();

    // List
    let assert = gtm()
        .args([
            "builtin-variables",
            "list",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    if let Some(vars) = json["builtInVariable"].as_array() {
        let types: Vec<&str> = vars.iter().filter_map(|v| v["type"].as_str()).collect();
        assert!(
            types.contains(&var_type),
            "should contain {var_type}: {types:?}"
        );
    }

    // Table output
    gtm_table()
        .args([
            "builtin-variables",
            "list",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(var_type));

    // Disable
    gtm()
        .args([
            "builtin-variables",
            "delete",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--type",
            var_type,
        ])
        .assert()
        .success();
}

// ─── Workspace Status ───

#[test]
#[ignore]
fn test_workspace_status() {
    gtm()
        .args([
            "workspaces",
            "status",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
            "--workspace-id",
            "2",
        ])
        .assert()
        .success();
}

// ─── Version Headers ───

#[test]
#[ignore]
fn test_version_headers_list() {
    gtm()
        .args([
            "version-headers",
            "list",
            "--account-id",
            ACCOUNT_ID,
            "--container-id",
            CONTAINER_ID,
        ])
        .assert()
        .success();
}

// ─── Auth Status ───

#[test]
#[ignore]
fn test_auth_status() {
    gtm().args(["auth", "status"]).assert().success();
}
