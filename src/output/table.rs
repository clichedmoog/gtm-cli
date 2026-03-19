use comfy_table::{presets::UTF8_FULL_CONDENSED, ContentArrangement, Table};
use serde_json::Value;

/// Column spec: (json_key, display_header)
type ColumnSpec = &'static [(&'static str, &'static str)];

/// Render a GTM API response as a table.
/// Handles both list responses (with a wrapper key) and single objects.
pub fn render(value: &Value, resource_hint: &str) {
    let (columns, items) = match detect_resource(value, resource_hint) {
        Some(r) => r,
        None => {
            // Fallback: just print JSON
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_default()
            );
            return;
        }
    };

    if items.is_empty() {
        println!("No items found.");
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic);

    // Header row
    let headers: Vec<&str> = columns.iter().map(|(_, h)| *h).collect();
    table.set_header(headers);

    // Data rows
    for item in &items {
        let row: Vec<String> = columns
            .iter()
            .map(|(key, _)| extract_value(item, key))
            .collect();
        table.add_row(row);
    }

    println!("{table}");
}

fn detect_resource<'a>(value: &'a Value, hint: &str) -> Option<(ColumnSpec, Vec<&'a Value>)> {
    let columns = columns_for(hint);

    // Try to find the list wrapper key
    if let Some(obj) = value.as_object() {
        // GTM API wraps lists in a key like "account", "container", "tag", etc.
        for (_key, val) in obj {
            if let Some(arr) = val.as_array() {
                return Some((columns, arr.iter().collect()));
            }
        }
        // Single object (e.g., get response)
        if !obj.is_empty() {
            return Some((columns, vec![value]));
        }
    }

    // Empty response
    Some((columns, vec![]))
}

fn columns_for(hint: &str) -> ColumnSpec {
    match hint {
        "accounts" | "account" => &[("accountId", "ID"), ("name", "Name"), ("path", "Path")],
        "containers" | "container" => &[
            ("containerId", "ID"),
            ("name", "Name"),
            ("publicId", "Public ID"),
            ("usageContext", "Context"),
        ],
        "workspaces" | "workspace" => &[
            ("workspaceId", "ID"),
            ("name", "Name"),
            ("description", "Description"),
        ],
        "tags" | "tag" => &[
            ("tagId", "ID"),
            ("name", "Name"),
            ("type", "Type"),
            ("parentFolderId", "Folder"),
        ],
        "triggers" | "trigger" => &[("triggerId", "ID"), ("name", "Name"), ("type", "Type")],
        "variables" | "variable" => &[
            ("variableId", "ID"),
            ("name", "Name"),
            ("type", "Type"),
            ("parentFolderId", "Folder"),
        ],
        "folders" | "folder" => &[("folderId", "ID"), ("name", "Name"), ("notes", "Notes")],
        "templates" | "template" => &[("templateId", "ID"), ("name", "Name"), ("type", "Type")],
        "versions" | "version" | "containerVersion" => &[
            ("containerVersionId", "ID"),
            ("name", "Name"),
            ("description", "Description"),
            ("fingerprint", "Fingerprint"),
        ],
        "version_headers" | "containerVersionHeader" => &[
            ("containerVersionId", "ID"),
            ("name", "Name"),
            ("numTags", "Tags"),
            ("numTriggers", "Triggers"),
            ("numVariables", "Variables"),
        ],
        "environments" | "environment" => &[
            ("environmentId", "ID"),
            ("name", "Name"),
            ("type", "Type"),
            ("enableDebug", "Debug"),
        ],
        "permissions" | "permission" | "user_permissions" => &[
            ("permissionId", "ID"),
            ("emailAddress", "Email"),
            ("accountAccess", "Access"),
        ],
        "clients" | "client" => &[("clientId", "ID"), ("name", "Name"), ("type", "Type")],
        "gtag_config" | "gtag_configs" => &[
            ("gtagConfigId", "ID"),
            ("measurementId", "Measurement ID"),
            ("type", "Type"),
        ],
        "transformations" | "transformation" => &[
            ("transformationId", "ID"),
            ("name", "Name"),
            ("type", "Type"),
        ],
        "zones" | "zone" => &[("zoneId", "ID"), ("name", "Name")],
        "destinations" | "destination" => &[
            ("destinationId", "ID"),
            ("name", "Name"),
            ("destinationLinkId", "Link ID"),
        ],
        "built_in_variables" | "builtInVariable" => &[("name", "Name"), ("type", "Type")],
        _ => &[("name", "Name"), ("type", "Type"), ("path", "Path")],
    }
}

fn extract_value(item: &Value, key: &str) -> String {
    match item.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        Some(Value::Object(obj)) => {
            // For nested objects like accountAccess, try to get "permission"
            if let Some(perm) = obj.get("permission").and_then(|p| p.as_str()) {
                return perm.to_string();
            }
            serde_json::to_string(&Value::Object(obj.clone())).unwrap_or_default()
        }
        Some(Value::Null) | None => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_string() {
        let item = json!({"name": "Test Tag"});
        assert_eq!(extract_value(&item, "name"), "Test Tag");
    }

    #[test]
    fn test_extract_missing() {
        let item = json!({"name": "Test"});
        assert_eq!(extract_value(&item, "missing"), "-");
    }

    #[test]
    fn test_extract_array() {
        let item = json!({"usageContext": ["web", "android"]});
        assert_eq!(extract_value(&item, "usageContext"), "web, android");
    }

    #[test]
    fn test_extract_nested_permission() {
        let item = json!({"accountAccess": {"permission": "admin"}});
        assert_eq!(extract_value(&item, "accountAccess"), "admin");
    }

    #[test]
    fn test_detect_resource_list() {
        let value = json!({"tag": [{"tagId": "1", "name": "A"}, {"tagId": "2", "name": "B"}]});
        let (cols, items) = detect_resource(&value, "tags").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(cols[0].1, "ID");
    }

    #[test]
    fn test_detect_resource_single() {
        let value = json!({"tagId": "1", "name": "A", "type": "html"});
        let (_, items) = detect_resource(&value, "tag").unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_detect_resource_empty() {
        let value = json!({});
        let (_, items) = detect_resource(&value, "tags").unwrap();
        assert!(items.is_empty());
    }
}
