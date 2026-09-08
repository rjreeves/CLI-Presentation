//! PPS envelope — see docs/presentation-protocol.md §3.

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Envelope {
    #[serde(default)]
    pub schema: Option<String>,
    pub data: Vec<Value>,
}

pub fn parse(input: &str) -> Result<Envelope, serde_json::Error> {
    serde_json::from_str(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_optional() {
        let envelope = parse(r#"{"data": [{"a": 1}]}"#).unwrap();
        assert_eq!(envelope.schema, None);
        assert_eq!(envelope.data.len(), 1);
    }

    #[test]
    fn schema_is_captured_when_present() {
        let envelope = parse(r#"{"schema": "network.ip", "data": []}"#).unwrap();
        assert_eq!(envelope.schema.as_deref(), Some("network.ip"));
    }

    #[test]
    fn missing_data_field_is_a_parse_error() {
        assert!(parse(r#"{"schema": "x"}"#).is_err());
    }

    #[test]
    fn invalid_json_is_a_parse_error() {
        assert!(parse("not json").is_err());
    }

    #[test]
    fn unrecognized_top_level_fields_are_ignored() {
        // pps_version/source/timestamp/meta from the full envelope
        // (presentation-protocol.md §3.2) aren't modeled yet — they
        // should be tolerated, not rejected.
        let envelope = parse(r#"{"pps_version": "1.0", "source": "x", "data": []}"#).unwrap();
        assert_eq!(envelope.data.len(), 0);
    }
}
