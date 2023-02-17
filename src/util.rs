use serde_json;

pub fn get_string_in_json<'a>(value: &serde_json::Value, path: &Vec<&'a str>) -> Option<String> {
    if path.is_empty() {
        return None;
    }

    let mut current_value = value;

    for i in 0..(path.len() - 1) {
        if let serde_json::Value::Object(m) = current_value {
            if let Some(inner) = m.get(path[i]) {
                current_value = inner;
            }
        }
    }

    if let serde_json::Value::Object(m) = current_value {
        if let Some(inner) = m.get(path[path.len() - 1]) {
            if let serde_json::Value::String(s) = inner {
                return Some(s.clone());
            }
        }
    }

    None
}
