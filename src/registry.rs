//! Schema registry: the built-in schema → renderer table, plus the
//! user-level file registry behind `presentation register`/`validate` —
//! see docs/schema-registry.md.
//!
//! Only the user-specific location from §8 is implemented
//! (`~/.presentation/registry.json`, or `%USERPROFILE%` on Windows); the
//! system-wide (`/etc`) and application-specific paths, the
//! database-backed option (§12), and custom WASM renderer plugin
//! registration (§6.2) are out of scope here.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;

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

/// Field definitions for the built-in schemas documented in
/// schema-registry.md §3. `docker.stats`/`k8s.pods` aren't given field
/// definitions there, so `validate` skips field-level checks for those.
fn builtin_fields(schema: &str) -> Option<&'static [(&'static str, &'static str)]> {
    match schema {
        "network.ip" => Some(&[
            ("interface", "string"),
            ("ipv4", "string|null"),
            ("ipv6", "string|null"),
            ("gateway", "string|null"),
            ("dns", "array<string>"),
            ("status", "string"),
            ("speed", "string|null"),
        ]),
        "network.wifi" => Some(&[
            ("ssid", "string"),
            ("signal", "number"),
            ("band", "string"),
            ("channel", "number"),
            ("ipv4", "string|null"),
            ("gateway", "string|null"),
        ]),
        "logs.events" => Some(&[
            ("timestamp", "string"),
            ("level", "string"),
            ("message", "string"),
            ("source", "string"),
        ]),
        "system.perf.cpu" => Some(&[
            ("timestamp", "string"),
            ("usage", "number"),
            ("temperature", "number|null"),
        ]),
        "fs.directory" => Some(&[
            ("name", "string"),
            ("size", "number"),
            ("type", "string"),
            ("modified", "string"),
        ]),
        _ => None,
    }
}

/// Combined lookup used at render time: the user registry takes
/// precedence over the built-in table, per schema-registry.md §8's
/// "application > user > system" merge order. Filesystem/parse problems
/// with the user registry are swallowed here (falling back to the
/// built-in table) so a corrupt or missing registry file never breaks
/// normal rendering — `presentation validate` is the tool for surfacing
/// that corruption loudly.
pub fn resolve_renderer(schema: &str) -> Option<String> {
    if let Ok(path) = registry_path()
        && let Ok(registry) = load_registry(&path)
        && let Some(entry) = registry.schemas.get(schema)
    {
        return Some(entry.default_renderer.clone());
    }
    default_renderer(schema).map(str::to_string)
}

#[derive(Serialize, Deserialize, Default)]
struct RegistryFile {
    registry_version: String,
    schemas: BTreeMap<String, SchemaEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct SchemaEntry {
    version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    default_renderer: String,
    #[serde(default)]
    fields: BTreeMap<String, String>,
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn registry_path() -> Result<PathBuf, Box<dyn Error>> {
    home_dir()
        .map(|home| home.join(".presentation").join("registry.json"))
        .ok_or_else(|| "could not determine home directory (HOME/USERPROFILE not set)".into())
}

fn load_registry(path: &PathBuf) -> Result<RegistryFile, Box<dyn Error>> {
    if !path.exists() {
        return Ok(RegistryFile {
            registry_version: "1.0".to_string(),
            schemas: BTreeMap::new(),
        });
    }
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|e| format!("registry file {} is corrupt: {e}", path.display()).into())
}

/// `presentation register --schema <name> --renderer <mode> [--fields <file>]`
/// — see docs/schema-registry.md §4, §6.1, §10.
pub fn register(
    schema: &str,
    renderer: &str,
    fields_path: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let path = registry_path()?;
    let mut registry = load_registry(&path)?;

    let fields = match fields_path {
        Some(p) => parse_fields_file(p)?,
        None => BTreeMap::new(),
    };

    registry.schemas.insert(
        schema.to_string(),
        SchemaEntry {
            version: "1.0".to_string(),
            description: None,
            default_renderer: renderer.to_string(),
            fields: fields.clone(),
        },
    );

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&registry)?)?;

    println!(
        "Registered schema '{schema}' -> {renderer} ({} field(s)) in {}",
        fields.len(),
        path.display()
    );
    Ok(())
}

/// A `--fields <file>` argument accepts either a bare field-name -> type
/// map, or the fuller schema-entry shape from schema-registry.md §10
/// (which nests that map under a `"fields"` key) — accept both.
fn parse_fields_file(path: &str) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let text = std::fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&text)?;
    let obj = value
        .as_object()
        .ok_or("fields file must contain a JSON object")?;
    let fields_obj = obj.get("fields").and_then(Value::as_object).unwrap_or(obj);
    Ok(fields_obj
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|t| (k.clone(), t.to_string())))
        .collect())
}

/// `presentation validate` — see docs/schema-registry.md §5 and
/// presentation-protocol.md §6.3.
pub fn validate(input: &str) -> Result<(), Box<dyn Error>> {
    let envelope = crate::protocol::parse(input)?;
    let rows = &envelope.data;

    let Some(schema) = envelope.schema.as_deref() else {
        println!("OK: {} record(s). No schema specified.", rows.len());
        return Ok(());
    };

    // Propagated via `?` rather than swallowed: a corrupt registry file is a
    // real problem `validate` exists to surface, not something to silently
    // treat as "schema not registered" (unlike resolve_renderer, which must
    // stay robust for the render path).
    let path = registry_path()?;
    let user_entry = load_registry(&path)?.schemas.get(schema).cloned();

    let (renderer, fields, source) = match user_entry {
        Some(entry) => (
            Some(entry.default_renderer),
            Some(entry.fields),
            "user-registered",
        ),
        None => (
            default_renderer(schema).map(str::to_string),
            builtin_fields(schema).map(|f| {
                f.iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect()
            }),
            "built-in",
        ),
    };

    let Some(renderer) = renderer else {
        return Err(format!(
            "[SRD_SCHEMA_NOT_FOUND] schema '{schema}' is not in the built-in or user registry"
        )
        .into());
    };

    let Some(fields) = fields.filter(|f: &BTreeMap<String, String>| !f.is_empty()) else {
        println!(
            "OK: {} record(s), schema '{schema}' recognized ({source}, renderer: {renderer}). No field definitions to validate against.",
            rows.len()
        );
        return Ok(());
    };

    let warnings = field_warnings(rows, &fields);
    if warnings.is_empty() {
        println!(
            "OK: {} record(s) match schema '{schema}' ({source}, renderer: {renderer}).",
            rows.len()
        );
    } else {
        println!(
            "{} record(s) checked against schema '{schema}' ({source}, renderer: {renderer}): {} warning(s).",
            rows.len(),
            warnings.len()
        );
        for w in &warnings {
            println!("  WARN: {w}");
        }
    }
    Ok(())
}

/// "Fields must match the registry's field definitions... Unknown fields
/// are allowed but ignored. Missing required fields produce warnings,
/// not hard failures." — schema-registry.md §5.
fn field_warnings(rows: &[Value], fields: &BTreeMap<String, String>) -> Vec<String> {
    let mut warnings = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        for (field, ty) in fields {
            let nullable = ty.contains("|null");
            let base = ty.split('|').next().unwrap_or(ty);
            match row.get(field) {
                None => {
                    if !nullable {
                        warnings.push(format!(
                            "record {}: missing required field '{field}'",
                            i + 1
                        ));
                    }
                }
                Some(Value::Null) => {
                    if !nullable {
                        warnings.push(format!(
                            "record {}: field '{field}' is null but is not nullable",
                            i + 1
                        ));
                    }
                }
                Some(v) if !type_matches(base, v) => {
                    warnings.push(format!(
                        "record {}: field '{field}' expected {base} but got {}",
                        i + 1,
                        json_type_name(v)
                    ));
                }
                _ => {}
            }
        }
    }
    warnings
}

fn type_matches(base: &str, value: &Value) -> bool {
    match base {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        t if t.starts_with("array<") => value.is_array(),
        _ => true,
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_renderer_covers_the_documented_table() {
        assert_eq!(default_renderer("network.ip"), Some("map"));
        assert_eq!(default_renderer("network.wifi"), Some("dashboard"));
        assert_eq!(default_renderer("logs.events"), Some("timeline"));
        assert_eq!(default_renderer("fs.directory"), Some("table"));
        assert_eq!(default_renderer("docker.stats"), Some("dashboard"));
        assert_eq!(default_renderer("k8s.pods"), Some("table"));
        assert_eq!(default_renderer("system.perf.cpu"), Some("chart"));
        assert_eq!(default_renderer("system.perf.memory"), Some("chart"));
        assert_eq!(default_renderer("unknown.schema"), None);
    }

    #[test]
    fn docker_stats_and_k8s_pods_have_no_documented_fields() {
        assert!(builtin_fields("docker.stats").is_none());
        assert!(builtin_fields("k8s.pods").is_none());
        assert!(builtin_fields("network.ip").is_some());
    }

    #[test]
    fn type_matches_checks_the_base_type_only() {
        assert!(type_matches("string", &json!("x")));
        assert!(!type_matches("string", &json!(1)));
        assert!(type_matches("number", &json!(1.5)));
        assert!(type_matches("boolean", &json!(true)));
        assert!(type_matches("array<string>", &json!(["a"])));
        assert!(!type_matches("array<string>", &json!("not an array")));
    }

    #[test]
    fn field_warnings_null_on_a_nullable_field_is_not_a_warning() {
        // Regression: an explicit `null` on a field declared "string|null"
        // was once wrongly flagged as a type mismatch.
        let rows = vec![json!({"ipv6": null})];
        let mut fields = BTreeMap::new();
        fields.insert("ipv6".to_string(), "string|null".to_string());
        assert!(field_warnings(&rows, &fields).is_empty());
    }

    #[test]
    fn field_warnings_null_on_a_required_field_is_a_warning() {
        let rows = vec![json!({"status": null})];
        let mut fields = BTreeMap::new();
        fields.insert("status".to_string(), "string".to_string());
        let warnings = field_warnings(&rows, &fields);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("not nullable"));
    }

    #[test]
    fn field_warnings_missing_required_field() {
        let rows = vec![json!({})];
        let mut fields = BTreeMap::new();
        fields.insert("status".to_string(), "string".to_string());
        let warnings = field_warnings(&rows, &fields);
        assert_eq!(
            warnings,
            vec!["record 1: missing required field 'status'".to_string()]
        );
    }

    #[test]
    fn field_warnings_missing_nullable_field_is_fine() {
        let rows = vec![json!({})];
        let mut fields = BTreeMap::new();
        fields.insert("gateway".to_string(), "string|null".to_string());
        assert!(field_warnings(&rows, &fields).is_empty());
    }

    #[test]
    fn field_warnings_unknown_extra_field_is_ignored() {
        let rows = vec![json!({"status": "up", "extra": "whatever"})];
        let mut fields = BTreeMap::new();
        fields.insert("status".to_string(), "string".to_string());
        assert!(field_warnings(&rows, &fields).is_empty());
    }

    #[test]
    fn field_warnings_type_mismatch() {
        let rows = vec![json!({"ipv4": 123})];
        let mut fields = BTreeMap::new();
        fields.insert("ipv4".to_string(), "string".to_string());
        let warnings = field_warnings(&rows, &fields);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("expected string but got number"));
    }

    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "presentation-test-{name}-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn parse_fields_file_accepts_a_flat_field_map() {
        let path = temp_file("flat", r#"{"a": "string", "b": "number"}"#);
        let fields = parse_fields_file(path.to_str().unwrap()).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(fields.get("a"), Some(&"string".to_string()));
        assert_eq!(fields.get("b"), Some(&"number".to_string()));
    }

    #[test]
    fn parse_fields_file_accepts_the_nested_schema_entry_shape() {
        let path = temp_file(
            "nested",
            r#"{"version": "1.0", "default_renderer": "chart", "fields": {"name": "string"}}"#,
        );
        let fields = parse_fields_file(path.to_str().unwrap()).unwrap();
        std::fs::remove_file(&path).ok();

        // Only the nested "fields" map is extracted, not "version"/"default_renderer".
        assert_eq!(fields.len(), 1);
        assert_eq!(fields.get("name"), Some(&"string".to_string()));
    }
}
