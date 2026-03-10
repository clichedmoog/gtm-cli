use serde::{Deserialize, Serialize};
use serde_json::Value;

/// GTM API parameter types, matching the wire format:
/// - `{ "type": "template", "key": "...", "value": "..." }`
/// - `{ "type": "list", "key": "...", "list": [...] }`
/// - `{ "type": "map", "key": "...", "map": [...] }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum GtmParameter {
    Template {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        value: String,
    },
    List {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        list: Vec<GtmParameter>,
    },
    Map {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        map: Vec<GtmParameter>,
    },
}

/// Convert a JSON value to a GTM parameter, mirroring the TypeScript
/// `convertParameterValue` from gtm-client.ts.
pub fn convert_parameter_value(key: Option<&str>, value: &Value) -> GtmParameter {
    let key_owned = key.map(String::from);

    match value {
        Value::String(s) => GtmParameter::Template {
            key: key_owned,
            value: s.clone(),
        },
        Value::Number(n) => GtmParameter::Template {
            key: key_owned,
            value: n.to_string(),
        },
        Value::Bool(b) => GtmParameter::Template {
            key: key_owned,
            value: b.to_string(),
        },
        Value::Array(arr) => {
            let list = arr
                .iter()
                .map(|item| convert_parameter_value(None, item))
                .collect();
            GtmParameter::List {
                key: key_owned,
                list,
            }
        }
        Value::Object(obj) => {
            let map = obj
                .iter()
                .map(|(k, v)| convert_parameter_value(Some(k), v))
                .collect();
            GtmParameter::Map {
                key: key_owned,
                map,
            }
        }
        Value::Null => GtmParameter::Template {
            key: key_owned,
            value: String::new(),
        },
    }
}

/// Convert a top-level JSON object into a Vec of GTM parameters.
pub fn params_from_json(params: &Value) -> Vec<GtmParameter> {
    match params.as_object() {
        Some(obj) => obj
            .iter()
            .map(|(k, v)| convert_parameter_value(Some(k), v))
            .collect(),
        None => vec![],
    }
}

/// Get the correct parameter key for a variable based on its type.
/// Mirrors gtm-client.ts variable type mapping.
pub fn get_variable_parameter_key(variable_type: &str) -> &'static str {
    match variable_type {
        "v" => "name",
        "jsm" => "javascript",
        _ => "value",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_convert_string() {
        let result = convert_parameter_value(Some("measurementId"), &json!("G-XXX"));
        assert_eq!(
            result,
            GtmParameter::Template {
                key: Some("measurementId".into()),
                value: "G-XXX".into(),
            }
        );
    }

    #[test]
    fn test_convert_number() {
        let result = convert_parameter_value(Some("count"), &json!(42));
        assert_eq!(
            result,
            GtmParameter::Template {
                key: Some("count".into()),
                value: "42".into(),
            }
        );
    }

    #[test]
    fn test_convert_boolean() {
        let result = convert_parameter_value(Some("enabled"), &json!(true));
        assert_eq!(
            result,
            GtmParameter::Template {
                key: Some("enabled".into()),
                value: "true".into(),
            }
        );
    }

    #[test]
    fn test_convert_array() {
        let result = convert_parameter_value(Some("items"), &json!(["a", "b"]));
        assert_eq!(
            result,
            GtmParameter::List {
                key: Some("items".into()),
                list: vec![
                    GtmParameter::Template {
                        key: None,
                        value: "a".into(),
                    },
                    GtmParameter::Template {
                        key: None,
                        value: "b".into(),
                    },
                ],
            }
        );
    }

    #[test]
    fn test_convert_object() {
        let result = convert_parameter_value(Some("config"), &json!({"key": "val"}));
        assert_eq!(
            result,
            GtmParameter::Map {
                key: Some("config".into()),
                map: vec![GtmParameter::Template {
                    key: Some("key".into()),
                    value: "val".into(),
                }],
            }
        );
    }

    #[test]
    fn test_convert_nested() {
        let result =
            convert_parameter_value(Some("outer"), &json!({"inner": [1, {"deep": "value"}]}));
        match &result {
            GtmParameter::Map { key, map } => {
                assert_eq!(key.as_deref(), Some("outer"));
                assert_eq!(map.len(), 1);
                match &map[0] {
                    GtmParameter::List { key, list } => {
                        assert_eq!(key.as_deref(), Some("inner"));
                        assert_eq!(list.len(), 2);
                    }
                    _ => panic!("Expected list"),
                }
            }
            _ => panic!("Expected map"),
        }
    }

    #[test]
    fn test_convert_null() {
        let result = convert_parameter_value(Some("empty"), &json!(null));
        assert_eq!(
            result,
            GtmParameter::Template {
                key: Some("empty".into()),
                value: "".into(),
            }
        );
    }

    #[test]
    fn test_params_from_json() {
        let params = json!({"measurementId": "G-XXX", "sendTo": "G-YYY"});
        let result = params_from_json(&params);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_params_from_json_non_object() {
        let result = params_from_json(&json!("not an object"));
        assert!(result.is_empty());
    }

    #[test]
    fn test_variable_parameter_key() {
        assert_eq!(get_variable_parameter_key("v"), "name");
        assert_eq!(get_variable_parameter_key("jsm"), "javascript");
        assert_eq!(get_variable_parameter_key("c"), "value");
        assert_eq!(get_variable_parameter_key("unknown"), "value");
    }

    #[test]
    fn test_serialize_template() {
        let param = GtmParameter::Template {
            key: Some("mid".into()),
            value: "G-XXX".into(),
        };
        let json = serde_json::to_value(&param).unwrap();
        assert_eq!(json["type"], "template");
        assert_eq!(json["key"], "mid");
        assert_eq!(json["value"], "G-XXX");
    }

    #[test]
    fn test_serialize_list() {
        let param = GtmParameter::List {
            key: Some("items".into()),
            list: vec![GtmParameter::Template {
                key: None,
                value: "a".into(),
            }],
        };
        let json = serde_json::to_value(&param).unwrap();
        assert_eq!(json["type"], "list");
        assert!(json["list"].is_array());
        // key: None should be omitted
        assert!(json["list"][0].get("key").is_none());
    }
}
