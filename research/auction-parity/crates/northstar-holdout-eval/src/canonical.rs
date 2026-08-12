use std::{fs, path::Path};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{Error, Result};

pub fn sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub fn file_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(sha256(&bytes))
}

pub fn sealed_text_matches(path: &Path, expected: &str) -> Result<bool> {
    Ok(file_sha256(path)? == expected || canonical_text_sha256(path)? == expected)
}

pub fn canonical_text_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut normalized = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
            normalized.push(b'\n');
            index += 2;
        } else {
            normalized.push(bytes[index]);
            index += 1;
        }
    }
    Ok(sha256(&normalized))
}

pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let value = serde_json::to_value(value)?;
    let mut output = Vec::with_capacity(4096);
    write_value(&value, &mut output)?;
    Ok(output)
}

fn write_value(value: &Value, output: &mut Vec<u8>) -> Result<()> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(value) => output.extend_from_slice(if *value { b"true" } else { b"false" }),
        Value::Number(value) => output.extend_from_slice(value.to_string().as_bytes()),
        Value::String(value) => output.extend_from_slice(serde_json::to_string(value)?.as_bytes()),
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                write_value(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            output.push(b'{');
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                output.extend_from_slice(serde_json::to_string(key)?.as_bytes());
                output.push(b':');
                write_value(&values[key], output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

pub fn canonical_hash<T: Serialize>(value: &T) -> Result<String> {
    Ok(sha256(&canonical_json(value)?))
}

pub fn canonical_hash_without<T: Serialize>(value: &T, field: &str) -> Result<String> {
    json_hash_without(&serde_json::to_vec(value)?, field)
}

pub fn json_hash_without(bytes: &[u8], field: &str) -> Result<String> {
    let mut value: Value = serde_json::from_slice(bytes)?;
    value
        .as_object_mut()
        .ok_or_else(|| Error::Contract("expected JSON object".into()))?
        .remove(field);
    canonical_hash(&value)
}

pub fn write_pretty_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 15) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_json_sorts_object_keys() {
        assert_eq!(
            canonical_json(&json!({"z": 1, "a": [true, null]})).unwrap(),
            b"{\"a\":[true,null],\"z\":1}"
        );
    }
}
