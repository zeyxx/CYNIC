use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&canonicalize(serde_json::to_value(value)?))
}

pub fn digest_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = canonical_json_bytes(value)?;
    Ok(format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}

fn canonicalize(value: Value) -> Value {
    match value {
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, canonicalize(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(canonicalize).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_key_order_does_not_change_digest() {
        let left = json!({"z": 1, "nested": {"b": 2, "a": 1}});
        let right = json!({"nested": {"a": 1, "b": 2}, "z": 1});
        assert_eq!(digest_json(&left).unwrap(), digest_json(&right).unwrap());
    }

    #[test]
    fn array_order_remains_significant() {
        assert_ne!(
            digest_json(&json!([1, 2])).unwrap(),
            digest_json(&json!([2, 1])).unwrap()
        );
    }
}
