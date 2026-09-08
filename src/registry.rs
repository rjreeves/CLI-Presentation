//! Built-in schema → default-renderer table.
//!
//! See docs/schema-registry.md §6 and docs/presentation-protocol.md §5.2.
//! Third-party registration (`presentation register`) and the
//! merged/file-backed registry from schema-registry.md §8 are not
//! implemented — this is the built-in table only.

/// Returns the renderer name PPS associates with a built-in schema, or
/// `None` if the schema is not recognized.
pub fn default_renderer(schema: &str) -> Option<&'static str> {
    match schema {
        "network.ip" => Some("map"),
        "network.wifi" => Some("dashboard"),
        "logs.events" => Some("timeline"),
        "fs.directory" => Some("table"),
        "docker.stats" => Some("dashboard"),
        "k8s.pods" => Some("table"),
        s if s.starts_with("system.perf.") => Some("chart"),
        _ => None,
    }
}
