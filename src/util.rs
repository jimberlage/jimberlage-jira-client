use serde_json::Value;

/// Gets a string out of a json object at a given path.
///
/// This is motivated by issue fields returned by the JIRA API being scoped as just a JSON object.
/// For example, to get the statusCategory ("To Do", "In Progress", "Done") from an issue returned by JIRA search:
///
/// ### Example
///
/// ```
/// use serde_json::Value;
/// use jimberlage_jira_client::util;
///
/// let data = r#"{
///   "statusCategory": {
///     "name": "Done"
///   }
/// }"#;
/// let value: Value = serde_json::from_str(data).unwrap();
/// let path = vec!["statusCategory", "name"];
///
/// assert_eq!(util::get_string_in_json(&value, &path), Some("Done".to_owned()));
/// ```
pub fn get_string_in_json<'a>(value: &Value, path: &Vec<&'a str>) -> Option<String> {
    if path.is_empty() {
        return None;
    }

    let mut current_value = value;

    for i in 0..(path.len() - 1) {
        if let Value::Object(m) = current_value {
            if let Some(inner) = m.get(path[i]) {
                current_value = inner;
            }
        }
    }

    if let Value::Object(m) = current_value {
        if let Some(inner) = m.get(path[path.len() - 1]) {
            if let Value::String(s) = inner {
                return Some(s.clone());
            }
        }
    }

    None
}
