use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CanonicalError {
    #[error("floating-point values are forbidden in authority artifacts")]
    FloatForbidden,
    #[error("JSON serialization failed: {0}")]
    Serialization(String),
}

pub fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, CanonicalError> {
    let mut out = Vec::with_capacity(256);
    write_value(value, &mut out)?;
    out.push(b'\n');
    Ok(out)
}

fn write_value(value: &Value, out: &mut Vec<u8>) -> Result<(), CanonicalError> {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(value) => out.extend_from_slice(if *value { b"true" } else { b"false" }),
        Value::Number(number) => {
            if !number.is_i64() && !number.is_u64() {
                return Err(CanonicalError::FloatForbidden);
            }
            out.extend_from_slice(number.to_string().as_bytes());
        }
        Value::String(text) => serde_json::to_writer(out, text)
            .map_err(|error| CanonicalError::Serialization(error.to_string()))?,
        Value::Array(values) => {
            out.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                write_value(value, out)?;
            }
            out.push(b']');
        }
        Value::Object(values) => {
            out.push(b'{');
            let mut keys: Vec<_> = values.keys().collect();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                serde_json::to_writer(&mut *out, key)
                    .map_err(|error| CanonicalError::Serialization(error.to_string()))?;
                out.push(b':');
                write_value(&values[key], out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}
