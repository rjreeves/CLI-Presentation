//! Shared, deterministic heuristics for `explain` and `ai-summary` — see
//! render::explain for why these modes are rule-based rather than an
//! actual LLM call.

use super::{DOWN_VALUES, STATUS_FIELDS, cell_text, columns};
use serde_json::Value;

const LABEL_FIELDS: &[&str] = &["name", "interface", "label", "id", "ssid", "host"];

pub(super) struct Insights {
    pub total: usize,
    pub label_field: Option<String>,
    pub status: Option<StatusInsight>,
    pub numeric: Vec<NumericInsight>,
}

pub(super) struct StatusInsight {
    pub field: String,
    pub flagged: Vec<String>,
}

pub(super) struct NumericInsight {
    pub field: String,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
}

pub(super) fn analyze(rows: &[&Value]) -> Insights {
    let cols = columns(rows);
    let label_field = LABEL_FIELDS
        .iter()
        .find(|f| cols.iter().any(|c| c == *f))
        .map(|f| f.to_string());

    let status = cols
        .iter()
        .find(|c| STATUS_FIELDS.contains(&c.to_ascii_lowercase().as_str()))
        .map(|field| status_insight(rows, field, label_field.as_deref()));

    let numeric = cols
        .iter()
        .filter_map(|c| numeric_insight(rows, c))
        .collect();

    Insights {
        total: rows.len(),
        label_field,
        status,
        numeric,
    }
}

fn status_insight(rows: &[&Value], field: &str, label_field: Option<&str>) -> StatusInsight {
    let flagged = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            DOWN_VALUES.contains(&cell_text(row, field).to_ascii_lowercase().as_str())
        })
        .map(|(i, row)| record_label(row, i, label_field))
        .collect();
    StatusInsight {
        field: field.to_string(),
        flagged,
    }
}

/// Only treats a field as "numeric" for summary purposes when every row
/// has a number there — a sparsely-numeric field is more likely a mixed
/// bag (e.g. `speed: "1Gbps" | null`) than a metric worth averaging.
fn numeric_insight(rows: &[&Value], field: &str) -> Option<NumericInsight> {
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|row| row.get(field).and_then(Value::as_f64))
        .collect();
    if values.is_empty() || values.len() != rows.len() {
        return None;
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let avg = values.iter().sum::<f64>() / values.len() as f64;
    Some(NumericInsight {
        field: field.to_string(),
        min,
        max,
        avg,
    })
}

pub(super) fn record_label(row: &Value, index: usize, label_field: Option<&str>) -> String {
    label_field
        .map(|f| cell_text(row, f))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("#{}", index + 1))
}
