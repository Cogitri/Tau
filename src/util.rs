use serde_json;
use serde_json::Value;

pub fn dict_get_u64(dict: &serde_json::Map<String, Value>, key: &str) -> Option<u64> {
    dict.get(key).and_then(Value::as_u64)
}

pub fn dict_get_string<'a>(dict: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a str> {
    dict.get(key).and_then(Value::as_str)
}
