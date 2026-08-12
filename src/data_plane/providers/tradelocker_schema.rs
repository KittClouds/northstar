//! Cold, value-free diagnostics for provider schema drift.

use serde_json::Value;

#[cold]
pub(super) fn json_shape(value: Option<&Value>) -> String {
    match value {
        None => "missing".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::Bool(_)) => "bool".into(),
        Some(Value::Number(_)) => "number".into(),
        Some(Value::String(_)) => "string".into(),
        Some(Value::Object(object)) => {
            let mut fields = object
                .iter()
                .map(|(key, value)| format!("{key}:{}", json_kind(value)))
                .collect::<Vec<_>>();
            fields.sort_unstable();
            format!("object{{{}}}", fields.join(","))
        }
        Some(Value::Array(values)) => {
            let entries = values
                .iter()
                .take(4)
                .map(|value| json_shape(Some(value)))
                .collect::<Vec<_>>();
            format!("array[len={};{}]", values.len(), entries.join("|"))
        }
    }
}

fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(number) => match number.as_f64() {
            Some(value) if value > 0.0 => "number-positive",
            Some(0.0) => "number-zero",
            Some(_) => "number-negative",
            None => "number-invalid",
        },
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
