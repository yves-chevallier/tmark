//! The document as JSON across a boundary (design 09 §Versioning): the
//! root carries `"tmark": "<version>"`, a consumer refuses a major
//! mismatch, and `schema_hash` lets a generated consumer detect drift.

use serde_json::Value;
use tmark_ir::Document;

/// The version of this workspace, what the JSON root carries.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The `Document` as JSON with `"tmark": VERSION` as its first key.
pub fn to_json(doc: &Document) -> Value {
    let mut value = serde_json::to_value(doc).expect("the IR serialises");
    if let Value::Object(map) = &mut value {
        map.shift_insert(0, "tmark".to_string(), Value::String(VERSION.to_string()));
    }
    value
}

/// A `Document` back from its JSON. The `"tmark"` key, when present, must
/// be compatible with [`VERSION`] (same major; before 1.0, same minor);
/// keys the IR does not know are ignored, so `parse`'s `"diagnostics"`
/// travels along harmlessly.
pub fn from_json(value: Value) -> Result<Document, String> {
    if let Some(Value::String(version)) = value.get("tmark") {
        if !compatible(version, VERSION) {
            return Err(format!(
                "document produced by tmark {version}, this is tmark {VERSION}"
            ));
        }
    }
    serde_json::from_value(value).map_err(|e| format!("not a tmark document: {e}"))
}

/// Semantic-version compatibility: same major, and same minor while the
/// major is 0.
fn compatible(theirs: &str, ours: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.split(['.', '-', '+'])
            .take(2)
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parts(theirs), parts(ours));
    match (a.first(), b.first()) {
        (Some(0), Some(0)) => a.get(1) == b.get(1),
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// A stable hash of the IR JSON schema, as a 16-digit hexadecimal string:
/// a consumer that generated models from `schema("ir")` compares it with
/// the one it recorded and refuses to run on drift. FNV-1a over the
/// compact JSON, so that the value is the same on every platform and
/// needs no dependency.
pub fn schema_hash() -> String {
    let schema = tmark_ir::schema("ir").expect("the IR schema exists");
    let text = serde_json::to_string(&schema).expect("the schema serialises");
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmark_ir::FileId;

    #[test]
    fn root_carries_the_version_and_round_trips() {
        let parsed = tmark_syntax::parse("Hello *world*\n", FileId(0));
        let json = to_json(&parsed.document);
        let keys: Vec<_> = json.as_object().unwrap().keys().collect();
        assert_eq!(keys[0], "tmark");
        assert_eq!(json["tmark"], VERSION);
        let back = from_json(json).unwrap();
        assert_eq!(
            tmark_ir::structural_json(&back),
            tmark_ir::structural_json(&parsed.document)
        );
    }

    #[test]
    fn refuses_a_major_mismatch() {
        let mut json = to_json(&Document::default());
        json["tmark"] = Value::String("99.0.0".to_string());
        let error = from_json(json).unwrap_err();
        assert!(error.contains("99.0.0"), "{error}");
        assert!(compatible("1.2.3", "1.9.0"));
        assert!(!compatible("2.0.0", "1.9.0"));
        assert!(compatible("0.3.1", "0.3.0"));
        assert!(!compatible("0.4.0", "0.3.0"));
    }

    #[test]
    fn schema_hash_is_stable() {
        let hash = schema_hash();
        assert_eq!(hash.len(), 16);
        assert_eq!(hash, schema_hash());
    }
}
