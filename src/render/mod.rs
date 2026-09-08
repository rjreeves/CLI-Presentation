//! Renderer trait — see docs/renderer-architecture.md §5.

pub mod ai_summary;
pub mod chart;
pub mod dashboard;
pub mod explain;
pub mod html;
mod insights;
pub mod map;
pub mod md;
pub mod table;
pub mod timeline;

use serde_json::Value;
use std::error::Error;

pub struct RenderOptions {
    pub sort: Option<String>,
    pub filter: Option<(String, String)>,
    pub width: Option<usize>,
    pub color: bool,
}

/// Column names that plausibly hold a status/health/severity value, shared
/// by every renderer that highlights or reasons about them.
pub(crate) const STATUS_FIELDS: &[&str] = &["status", "state", "level"];
pub(crate) const UP_VALUES: &[&str] = &["up", "active", "ok", "online", "info"];
pub(crate) const DOWN_VALUES: &[&str] =
    &["down", "error", "offline", "inactive", "critical", "fatal"];

pub trait Renderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>>;
}

/// Renders a JSON value as display text; `null` renders as empty.
pub(crate) fn value_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        v => v.to_string(),
    }
}

/// Renders a field's value as display text; `null`/missing render as empty.
pub(crate) fn cell_text(row: &Value, field: &str) -> String {
    row.get(field).map(value_text).unwrap_or_default()
}

fn passes_filter(row: &Value, options: &RenderOptions) -> bool {
    match &options.filter {
        None => true,
        Some((field, expected)) => cell_text(row, field) == *expected,
    }
}

fn compare_field(a: &Value, fa: &str, b: &Value, fb: &str) -> std::cmp::Ordering {
    let (av, bv) = (a.get(fa), b.get(fb));
    match (av.and_then(Value::as_f64), bv.and_then(Value::as_f64)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        _ => cell_text(a, fa).cmp(&cell_text(b, fb)),
    }
}

/// Applies `--filter` and `--sort` to `data`, shared by every renderer.
pub(crate) fn sorted_filtered<'a>(data: &'a [Value], options: &RenderOptions) -> Vec<&'a Value> {
    let mut rows: Vec<&Value> = data.iter().filter(|v| passes_filter(v, options)).collect();
    if let Some(field) = &options.sort {
        rows.sort_by(|a, b| compare_field(a, field, b, field));
    }
    rows
}

/// Truncates `s` to at most `width` characters, marking truncation with `…`.
pub(crate) fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_string()
    } else if width == 0 {
        String::new()
    } else {
        let head: String = s.chars().take(width - 1).collect();
        format!("{head}…")
    }
}

/// Column names for a set of records, taken from the first record's keys.
pub(crate) fn columns(rows: &[&Value]) -> Vec<String> {
    rows.first()
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

/// Finds the first of `candidates` that exists as a key in `obj`.
pub(crate) fn pick_field(
    obj: &serde_json::Map<String, Value>,
    candidates: &[&str],
) -> Option<String> {
    candidates
        .iter()
        .find(|c| obj.contains_key(**c))
        .map(|c| c.to_string())
}

/// Escapes the five characters that are significant in both HTML and XML
/// (SVG) text content — shared by html mode and `--export html`/`svg`.
pub(crate) fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn value_text_renders_null_as_empty() {
        assert_eq!(value_text(&json!(null)), "");
        assert_eq!(value_text(&json!("hi")), "hi");
        assert_eq!(value_text(&json!(42)), "42");
    }

    #[test]
    fn cell_text_missing_field_is_empty() {
        let row = json!({"a": 1});
        assert_eq!(cell_text(&row, "a"), "1");
        assert_eq!(cell_text(&row, "missing"), "");
    }

    #[test]
    fn truncate_short_string_is_unchanged() {
        assert_eq!(truncate("hi", 10), "hi");
    }

    #[test]
    fn truncate_long_string_gets_ellipsis() {
        assert_eq!(truncate("hello world", 5), "hell…");
    }

    #[test]
    fn truncate_zero_width_is_empty() {
        assert_eq!(truncate("hello", 0), "");
    }

    #[test]
    fn truncate_is_char_safe_not_byte_safe() {
        // "café" truncated to 3 chars must not panic on the multi-byte 'é'.
        assert_eq!(truncate("café", 3), "ca…");
    }

    #[test]
    fn columns_from_first_row() {
        let rows = [json!({"b": 1, "a": 2})];
        let refs: Vec<&Value> = rows.iter().collect();
        let mut cols = columns(&refs);
        cols.sort();
        assert_eq!(cols, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn columns_of_empty_rows_is_empty() {
        let refs: Vec<&Value> = vec![];
        assert!(columns(&refs).is_empty());
    }

    #[test]
    fn pick_field_prefers_first_candidate_present() {
        let obj = json!({"time": "t", "timestamp": "ts"})
            .as_object()
            .unwrap()
            .clone();
        assert_eq!(
            pick_field(&obj, &["timestamp", "time"]),
            Some("timestamp".to_string())
        );
        assert_eq!(
            pick_field(&obj, &["date", "time"]),
            Some("time".to_string())
        );
        assert_eq!(pick_field(&obj, &["date"]), None);
    }

    #[test]
    fn escape_html_covers_all_five_entities() {
        assert_eq!(
            escape_html(r#"<script>alert('x') & "y"</script>"#),
            "&lt;script&gt;alert(&#39;x&#39;) &amp; &quot;y&quot;&lt;/script&gt;"
        );
    }

    fn opts(sort: Option<&str>, filter: Option<(&str, &str)>) -> RenderOptions {
        RenderOptions {
            sort: sort.map(str::to_string),
            filter: filter.map(|(f, v)| (f.to_string(), v.to_string())),
            width: None,
            color: false,
        }
    }

    #[test]
    fn sorted_filtered_sorts_numerically_when_both_sides_parse() {
        let data = vec![json!({"v": 3}), json!({"v": 1}), json!({"v": 2})];
        let rows = sorted_filtered(&data, &opts(Some("v"), None));
        let values: Vec<i64> = rows.iter().map(|r| r["v"].as_i64().unwrap()).collect();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn sorted_filtered_applies_equality_filter() {
        let data = vec![json!({"status": "up"}), json!({"status": "down"})];
        let rows = sorted_filtered(&data, &opts(None, Some(("status", "down"))));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["status"], "down");
    }
}
