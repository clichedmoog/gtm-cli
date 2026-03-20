//! Tests using a mock HTTP server to test CLI commands without real API calls.
//! Uses wiremock to simulate GTM API responses based on real API response shapes.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_server() -> MockServer {
    MockServer::start().await
}

fn gtm_with_server(server: &MockServer) -> Command {
    let mut cmd = Command::cargo_bin("gtm").expect("binary exists");
    cmd.env("GTM_API_BASE", server.uri());
    cmd.env("GTM_QUIET", "1"); // suppress update check
    cmd.args(["--format", "json"]);
    cmd
}

fn gtm_table_with_server(server: &MockServer) -> Command {
    let mut cmd = Command::cargo_bin("gtm").expect("binary exists");
    cmd.env("GTM_API_BASE", server.uri());
    cmd.env("GTM_QUIET", "1");
    cmd.args(["--format", "table"]);
    cmd
}

fn parse_json(output: &assert_cmd::assert::Assert) -> Value {
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    serde_json::from_str(&stdout).expect("should be valid JSON")
}

/// Mount a workspace resolution mock (many commands auto-resolve workspace)
async fn mount_workspace_mock(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workspace": [{"workspaceId": "1"}]
        })))
        .mount(server)
        .await;
}

// ─── Accounts ───

#[tokio::test]
async fn test_mock_accounts_list() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "account": [
                {
                    "accountId": "123456",
                    "name": "Test Account",
                    "path": "accounts/123456",
                    "features": {
                        "supportMultipleContainers": true,
                        "supportUserPermissions": true
                    }
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args(["accounts", "list"])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array());
    assert_eq!(json[0]["accountId"], "123456");
    assert_eq!(json[0]["name"], "Test Account");
}

#[tokio::test]
async fn test_mock_accounts_list_table() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "account": [
                {
                    "accountId": "123456",
                    "name": "Test Account",
                    "path": "accounts/123456"
                }
            ]
        })))
        .mount(&server)
        .await;

    gtm_table_with_server(&server)
        .args(["accounts", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("123456"))
        .stdout(predicate::str::contains("Test Account"));
}

#[tokio::test]
async fn test_mock_accounts_get() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "accountId": "123456",
            "name": "Test Account",
            "path": "accounts/123456",
            "fingerprint": "1234567890"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args(["accounts", "get", "--account-id", "123456"])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["accountId"], "123456");
}

// ─── Containers ───

#[tokio::test]
async fn test_mock_containers_list() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "container": [
                {
                    "accountId": "123456",
                    "containerId": "789",
                    "name": "My Container",
                    "publicId": "GTM-XXXX",
                    "usageContext": ["web"],
                    "path": "accounts/123456/containers/789"
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args(["containers", "list", "--account-id", "123456"])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array());
    assert_eq!(json[0]["publicId"], "GTM-XXXX");
}

#[tokio::test]
async fn test_mock_containers_create() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "accountId": "123456",
            "containerId": "999",
            "name": "New Container",
            "usageContext": ["web"],
            "path": "accounts/123456/containers/999"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "containers",
            "create",
            "--account-id",
            "123456",
            "--name",
            "New Container",
            "--usage-context",
            "web",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["name"], "New Container");
    assert_eq!(json["containerId"], "999");
}

// ─── Workspaces ───

#[tokio::test]
async fn test_mock_workspaces_list() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workspace": [
                {
                    "accountId": "123456",
                    "containerId": "789",
                    "workspaceId": "1",
                    "name": "Default Workspace",
                    "path": "accounts/123456/containers/789/workspaces/1"
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "workspaces",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array());
    assert_eq!(json[0]["name"], "Default Workspace");
}

// ─── Tags ───

#[tokio::test]
async fn test_mock_tags_list() {
    let server = setup_server().await;
    // Workspace resolution
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workspace": [{"workspaceId": "1"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces/1/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tag": [
                {
                    "tagId": "42",
                    "name": "GA4 Config",
                    "type": "gaawc",
                    "path": "accounts/123456/containers/789/workspaces/1/tags/42"
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "tags",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array());
    assert_eq!(json[0]["name"], "GA4 Config");
}

#[tokio::test]
async fn test_mock_tags_create() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workspace": [{"workspaceId": "1"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers/789/workspaces/1/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tagId": "99",
            "name": "Test Tag",
            "type": "html",
            "firingTriggerId": ["1"]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "tags",
            "create",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--name",
            "Test Tag",
            "--type",
            "html",
            "--firing-trigger-id",
            "1",
            "--params",
            r#"{"html":"<script>test</script>"}"#,
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["tagId"], "99");
    assert_eq!(json["name"], "Test Tag");
}

#[tokio::test]
async fn test_mock_tags_delete_requires_force() {
    let server = setup_server().await;
    // Without --force, should print warning and exit successfully (no API call)
    gtm_with_server(&server)
        .args([
            "tags",
            "delete",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--tag-id",
            "42",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("--force"));
}

#[tokio::test]
async fn test_mock_tags_delete_with_force() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workspace": [{"workspaceId": "1"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/accounts/123456/containers/789/workspaces/1/tags/42"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    gtm_with_server(&server)
        .args([
            "tags",
            "delete",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--tag-id",
            "42",
            "--force",
        ])
        .assert()
        .success();
}

// ─── Triggers ───

#[tokio::test]
async fn test_mock_triggers_list() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workspace": [{"workspaceId": "1"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/triggers",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "trigger": [
                {
                    "triggerId": "5",
                    "name": "All Pages",
                    "type": "pageview"
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "triggers",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json[0]["name"], "All Pages");
}

// ─── Variables ───

#[tokio::test]
async fn test_mock_variables_list() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workspace": [{"workspaceId": "1"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/variables",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "variable": [
                {
                    "variableId": "10",
                    "name": "Page URL",
                    "type": "u"
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "variables",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json[0]["name"], "Page URL");
}

// ─── Versions ───

#[tokio::test]
async fn test_mock_versions_list() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "containerVersion": [
                {
                    "containerVersionId": "1",
                    "name": "v1.0",
                    "path": "accounts/123456/containers/789/versions/1"
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "versions",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array());
}

#[tokio::test]
async fn test_mock_versions_live() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/versions:live"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "containerVersionId": "3",
            "name": "Production",
            "path": "accounts/123456/containers/789/versions/3"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "versions",
            "live",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["containerVersionId"], "3");
}

// ─── Environments ───

#[tokio::test]
async fn test_mock_environments_list() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/environments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "environment": [
                {
                    "environmentId": "1",
                    "name": "Live",
                    "type": "live",
                    "path": "accounts/123456/containers/789/environments/1"
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "environments",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array(), "environments should be unwrapped array");
}

// ─── Permissions ───

#[tokio::test]
async fn test_mock_permissions_list() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/user_permissions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "userPermission": [
                {
                    "accountId": "123456",
                    "emailAddress": "user@example.com",
                    "accountAccess": {"permission": "admin"}
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args(["permissions", "list", "--account-id", "123456"])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array(), "permissions should be unwrapped array");
}

// ─── Pagination ───

#[tokio::test]
async fn test_mock_pagination() {
    let server = setup_server().await;

    // Page 1
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "account": [
                {"accountId": "1", "name": "Account 1"}
            ],
            "nextPageToken": "page2"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Page 2
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .and(query_param("pageToken", "page2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "account": [
                {"accountId": "2", "name": "Account 2"}
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args(["accounts", "list"])
        .assert()
        .success();
    let json = parse_json(&assert);
    let accounts = json.as_array().unwrap();
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0]["accountId"], "1");
    assert_eq!(accounts[1]["accountId"], "2");
}

// ─── Dry Run ───

#[tokio::test]
async fn test_mock_dry_run() {
    let server = setup_server().await;
    // No mock needed — dry-run should NOT call the API

    gtm_with_server(&server)
        .args([
            "--dry-run",
            "tags",
            "create",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--workspace-id",
            "1",
            "--name",
            "Dry Run Tag",
            "--type",
            "html",
            "--firing-trigger-id",
            "1",
            "--params",
            r#"{"html":"test"}"#,
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("[dry-run]"));
}

// ─── Error Handling ───

#[tokio::test]
async fn test_mock_api_error_404() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/999999"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": {
                "code": 404,
                "message": "Account not found"
            }
        })))
        .mount(&server)
        .await;

    gtm_with_server(&server)
        .args(["accounts", "get", "--account-id", "999999"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Account not found"));
}

#[tokio::test]
async fn test_mock_api_error_403() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "error": {
                "code": 403,
                "message": "The caller does not have permission"
            }
        })))
        .mount(&server)
        .await;

    gtm_with_server(&server)
        .args(["accounts", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("permission"));
}

// ─── Compact Format ───

#[tokio::test]
async fn test_mock_compact_format() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "account": [
                {"accountId": "123", "name": "Test"}
            ]
        })))
        .mount(&server)
        .await;

    let mut cmd = Command::cargo_bin("gtm").expect("binary exists");
    cmd.env("GTM_API_BASE", server.uri());
    cmd.env("GTM_QUIET", "1");
    cmd.args(["--format", "compact", "accounts", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("123"))
        .stdout(predicate::str::contains("Test"));
}

// ─── Accounts Update ───

#[tokio::test]
async fn test_mock_accounts_update() {
    let server = setup_server().await;
    // Update does GET first to fetch existing, then PUT with changes
    Mock::given(method("GET"))
        .and(path("/accounts/123456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "accountId": "123456",
            "name": "Old Name"
        })))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/accounts/123456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "accountId": "123456",
            "name": "Updated Account"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "accounts",
            "update",
            "--account-id",
            "123456",
            "--name",
            "Updated Account",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["name"], "Updated Account");
}

// ─── Containers Get / Update / Delete ───

#[tokio::test]
async fn test_mock_containers_get() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "containerId": "789",
            "name": "My Container",
            "publicId": "GTM-XXXX"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "containers",
            "get",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["publicId"], "GTM-XXXX");
}

#[tokio::test]
async fn test_mock_containers_delete_requires_force() {
    let server = setup_server().await;
    gtm_with_server(&server)
        .args([
            "containers",
            "delete",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("--force"));
}

#[tokio::test]
async fn test_mock_containers_delete_with_force() {
    let server = setup_server().await;
    Mock::given(method("DELETE"))
        .and(path("/accounts/123456/containers/789"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    gtm_with_server(&server)
        .args([
            "containers",
            "delete",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--force",
        ])
        .assert()
        .success();
}

#[tokio::test]
async fn test_mock_containers_combine() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers/789:combine"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "container": {"containerId": "789", "name": "Combined"}
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "containers",
            "combine",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json["container"].is_object());
}

#[tokio::test]
async fn test_mock_containers_move_tag_id() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers/789:move_tag_id"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "container": {"containerId": "789"}
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "containers",
            "move-tag-id",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--tag-id",
            "UA-12345",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json["container"].is_object());
}

// ─── Workspaces Create / Get / Delete / CreateVersion / ResolveConflict ───

#[tokio::test]
async fn test_mock_workspaces_create() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers/789/workspaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workspaceId": "5",
            "name": "Feature Branch",
            "description": "Testing workspace"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "workspaces",
            "create",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--name",
            "Feature Branch",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["name"], "Feature Branch");
}

#[tokio::test]
async fn test_mock_workspaces_create_version() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1:create_version",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "containerVersion": {
                "containerVersionId": "10",
                "name": "v2.0"
            }
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "workspaces",
            "create-version",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--workspace-id",
            "1",
            "--name",
            "v2.0",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["containerVersion"]["name"], "v2.0");
}

#[tokio::test]
async fn test_mock_workspaces_resolve_conflict() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1:resolve_conflict",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;

    gtm_with_server(&server)
        .args([
            "workspaces",
            "resolve-conflict",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--workspace-id",
            "1",
            "--entity",
            r#"{"tag":{"name":"Keep This"}}"#,
        ])
        .assert()
        .success();
}

// ─── Tags Get / Update / Revert ───

#[tokio::test]
async fn test_mock_tags_get() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces/1/tags/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tagId": "42",
            "name": "GA4 Config",
            "type": "gaawc"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "tags",
            "get",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--tag-id",
            "42",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["tagId"], "42");
}

#[tokio::test]
async fn test_mock_tags_update() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    // Update does GET first, then PUT
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces/1/tags/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tagId": "42",
            "name": "Old Tag",
            "type": "gaawc"
        })))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/accounts/123456/containers/789/workspaces/1/tags/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tagId": "42",
            "name": "Updated Tag",
            "type": "gaawc"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "tags",
            "update",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--tag-id",
            "42",
            "--name",
            "Updated Tag",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["name"], "Updated Tag");
}

#[tokio::test]
async fn test_mock_tags_revert() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("POST"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/tags/42:revert",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tag": {"tagId": "42", "name": "Reverted Tag"}
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "tags",
            "revert",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--tag-id",
            "42",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json["tag"].is_object());
}

// ─── Triggers Create / Delete ───

#[tokio::test]
async fn test_mock_triggers_create() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("POST"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/triggers",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "triggerId": "10",
            "name": "Button Click",
            "type": "customEvent"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "triggers",
            "create",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--name",
            "Button Click",
            "--type",
            "customEvent",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["name"], "Button Click");
}

#[tokio::test]
async fn test_mock_triggers_delete_with_force() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("DELETE"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/triggers/10",
        ))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    gtm_with_server(&server)
        .args([
            "triggers",
            "delete",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--trigger-id",
            "10",
            "--force",
        ])
        .assert()
        .success();
}

// ─── Variables Create / Delete ───

#[tokio::test]
async fn test_mock_variables_create() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("POST"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/variables",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "variableId": "20",
            "name": "User ID",
            "type": "v"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "variables",
            "create",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--name",
            "User ID",
            "--type",
            "v",
            "--params",
            r#"{"name":"userId"}"#,
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["name"], "User ID");
}

#[tokio::test]
async fn test_mock_variables_delete_with_force() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("DELETE"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/variables/20",
        ))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    gtm_with_server(&server)
        .args([
            "variables",
            "delete",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--variable-id",
            "20",
            "--force",
        ])
        .assert()
        .success();
}

// ─── Versions Create / Publish ───

#[tokio::test]
async fn test_mock_versions_create() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1:create_version",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "containerVersion": {
                "containerVersionId": "5",
                "name": "Release v1.0"
            }
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "versions",
            "create",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--workspace-id",
            "1",
            "--name",
            "Release v1.0",
            "--notes",
            "Initial release",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["containerVersion"]["name"], "Release v1.0");
}

#[tokio::test]
async fn test_mock_versions_publish() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers/789/versions/5:publish"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "containerVersion": {
                "containerVersionId": "5",
                "name": "Release v1.0"
            }
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "versions",
            "publish",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--version-id",
            "5",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["containerVersion"]["containerVersionId"], "5");
}

#[tokio::test]
async fn test_mock_versions_get() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/versions/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "containerVersionId": "5",
            "name": "v1.0",
            "tag": [{"tagId": "1", "name": "GA4"}]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "versions",
            "get",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--version-id",
            "5",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["containerVersionId"], "5");
}

// ─── Version Headers ───

#[tokio::test]
async fn test_mock_version_headers_list() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/version_headers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "containerVersionHeader": [
                {
                    "containerVersionId": "1",
                    "name": "v1.0",
                    "numTags": "5"
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "version-headers",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array(), "versionHeaders should be unwrapped array");
}

#[tokio::test]
async fn test_mock_version_headers_latest() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path(
            "/accounts/123456/containers/789/version_headers:latest",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "containerVersionId": "3",
            "name": "Latest"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "version-headers",
            "latest",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["containerVersionId"], "3");
}

// ─── Environments Create / Get / Delete ───

#[tokio::test]
async fn test_mock_environments_create() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers/789/environments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "environmentId": "5",
            "name": "Staging",
            "type": "user"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "environments",
            "create",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--name",
            "Staging",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["name"], "Staging");
}

#[tokio::test]
async fn test_mock_environments_delete_with_force() {
    let server = setup_server().await;
    Mock::given(method("DELETE"))
        .and(path("/accounts/123456/containers/789/environments/5"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    gtm_with_server(&server)
        .args([
            "environments",
            "delete",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--environment-id",
            "5",
            "--force",
        ])
        .assert()
        .success();
}

// ─── Permissions Create / Delete ───

#[tokio::test]
async fn test_mock_permissions_create() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/user_permissions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "accountId": "123456",
            "emailAddress": "new@example.com",
            "accountAccess": {"permission": "user"}
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "permissions",
            "create",
            "--account-id",
            "123456",
            "--email",
            "new@example.com",
            "--account-access",
            "user",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["emailAddress"], "new@example.com");
}

// ─── Folders List / Create / Delete ───

#[tokio::test]
async fn test_mock_folders_list() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces/1/folders"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "folder": [
                {"folderId": "3", "name": "GA4 Tags"}
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "folders",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json[0]["name"], "GA4 Tags");
}

#[tokio::test]
async fn test_mock_folders_create() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers/789/workspaces/1/folders"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "folderId": "10",
            "name": "New Folder"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "folders",
            "create",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--name",
            "New Folder",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["name"], "New Folder");
}

#[tokio::test]
async fn test_mock_folders_entities() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("GET"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/folders/3:entities",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tag": [{"tagId": "1", "name": "GA4"}],
            "trigger": [],
            "variable": []
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "folders",
            "entities",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--folder-id",
            "3",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json["tag"].is_array());
}

// ─── Built-in Variables ───

#[tokio::test]
async fn test_mock_builtin_variables_list() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("GET"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/built_in_variables",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "builtInVariable": [
                {"name": "Page URL", "type": "pageUrl"},
                {"name": "Click URL", "type": "clickUrl"}
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "builtin-variables",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array());
    assert_eq!(json[0]["type"], "pageUrl");
}

// ─── Destinations ───

#[tokio::test]
async fn test_mock_destinations_list() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/destinations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "destination": [
                {
                    "destinationId": "d1",
                    "name": "Analytics",
                    "path": "accounts/123456/containers/789/destinations/d1"
                }
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "destinations",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array(), "destinations should be unwrapped array");
}

#[tokio::test]
async fn test_mock_destinations_get() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/destinations/d1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "destinationId": "d1",
            "name": "Analytics"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "destinations",
            "get",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--destination-id",
            "d1",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["destinationId"], "d1");
}

// ─── Clients (server-side) ───

#[tokio::test]
async fn test_mock_clients_list() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces/1/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "client": [
                {"clientId": "1", "name": "GA4 Client", "type": "gaaw_client"}
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "clients",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array());
    assert_eq!(json[0]["name"], "GA4 Client");
}

#[tokio::test]
async fn test_mock_clients_create() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers/789/workspaces/1/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "clientId": "5",
            "name": "New Client",
            "type": "gaaw_client"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "clients",
            "create",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--name",
            "New Client",
            "--type",
            "gaaw_client",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["name"], "New Client");
}

// ─── Google Tag Configs ───

#[tokio::test]
async fn test_mock_gtag_configs_list() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("GET"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/gtag_config",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "gtagConfig": [
                {"gtagConfigId": "1", "type": "googtag"}
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "gtag-configs",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array(), "gtagConfigs should be unwrapped array");
}

// ─── Templates ───

#[tokio::test]
async fn test_mock_templates_list() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("GET"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/templates",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "template": [
                {"templateId": "1", "name": "Custom Template"}
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "templates",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array(), "templates should be unwrapped array");
}

// ─── Transformations (server-side) ───

#[tokio::test]
async fn test_mock_transformations_list() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("GET"))
        .and(path(
            "/accounts/123456/containers/789/workspaces/1/transformations",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "transformation": [
                {"transformationId": "1", "name": "Event Filter"}
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "transformations",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array(), "transformations should be unwrapped array");
}

// ─── Zones (server-side) ───

#[tokio::test]
async fn test_mock_zones_list() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces/1/zones"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "zone": [
                {"zoneId": "1", "name": "Secure Zone"}
            ]
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "zones",
            "list",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json.is_array(), "zones should be unwrapped array");
}

#[tokio::test]
async fn test_mock_zones_create() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers/789/workspaces/1/zones"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "zoneId": "5",
            "name": "Test Zone"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "zones",
            "create",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--name",
            "Test Zone",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert_eq!(json["name"], "Test Zone");
}

#[tokio::test]
async fn test_mock_zones_delete_requires_force() {
    let server = setup_server().await;
    gtm_with_server(&server)
        .args([
            "zones",
            "delete",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--zone-id",
            "5",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("--force"));
}

// ─── Workspaces Status / Sync ───

#[tokio::test]
async fn test_mock_workspaces_status() {
    let server = setup_server().await;
    mount_workspace_mock(&server).await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789/workspaces/1/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workspaceChange": [
                {"tag": {"tagId": "1", "name": "Modified Tag"}}
            ],
            "mergeConflict": []
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "workspaces",
            "status",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--workspace-id",
            "1",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json["workspaceChange"].is_array());
}

#[tokio::test]
async fn test_mock_workspaces_sync() {
    let server = setup_server().await;
    Mock::given(method("POST"))
        .and(path("/accounts/123456/containers/789/workspaces/1:sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "syncStatus": {
                "synced": true
            }
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "workspaces",
            "sync",
            "--account-id",
            "123456",
            "--container-id",
            "789",
            "--workspace-id",
            "1",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json["syncStatus"].is_object());
}

// ─── Error: Rate Limiting (429) ───

#[tokio::test]
async fn test_mock_api_error_rate_limit() {
    let server = setup_server().await;
    // Always return 429 - the client should eventually fail
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": {
                "code": 429,
                "message": "Rate Limit Exceeded"
            }
        })))
        .mount(&server)
        .await;

    gtm_with_server(&server)
        .args(["accounts", "list"])
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .failure();
}

// ─── Containers Snippet ───

#[tokio::test]
async fn test_mock_containers_snippet() {
    let server = setup_server().await;
    Mock::given(method("GET"))
        .and(path("/accounts/123456/containers/789:snippet"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "snippet": "<!-- GTM snippet -->"
        })))
        .mount(&server)
        .await;

    let assert = gtm_with_server(&server)
        .args([
            "containers",
            "snippet",
            "--account-id",
            "123456",
            "--container-id",
            "789",
        ])
        .assert()
        .success();
    let json = parse_json(&assert);
    assert!(json["snippet"].is_string());
}
