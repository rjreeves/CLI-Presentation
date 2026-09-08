//! Renderer trait — see docs/renderer-architecture.md §5.

pub mod chart;
pub mod dashboard;
pub mod table;

use serde_json::Value;
use std::error::Error;

pub struct RenderOptions {
    pub sort: Option<String>,
    pub filter: Option<(String, String)>,
    pub width: Option<usize>,
    pub color: bool,
}

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
