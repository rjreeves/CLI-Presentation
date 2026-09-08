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
