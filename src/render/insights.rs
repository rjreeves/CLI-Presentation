//! Shared, deterministic heuristics for `explain` and `ai-summary` — see
//! render::explain for why these modes are rule-based rather than an
//! actual LLM call.

use super::{DOWN_VALUES, STATUS_FIELDS, cell_text, columns};
use serde_json::Value;

const LABEL_FIELDS: &[&str] = &[
    "name",
    "interface",
    "label",
    "id",
    "ssid",
    "host",
    "hostname",
];

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numeric_insight_computes_min_max_avg() {
        let rows = [
            json!({"cpu": 10.0}),
            json!({"cpu": 20.0}),
            json!({"cpu": 30.0}),
        ];
        let refs: Vec<&Value> = rows.iter().collect();
        let insights = analyze(&refs);
        let cpu = insights.numeric.iter().find(|n| n.field == "cpu").unwrap();
        assert_eq!(cpu.min, 10.0);
        assert_eq!(cpu.max, 30.0);
        assert_eq!(cpu.avg, 20.0);
    }

    #[test]
    fn numeric_insight_skipped_when_field_is_not_numeric_on_every_row() {
        let rows = [json!({"speed": "1Gbps"}), json!({"speed": null})];
        let refs: Vec<&Value> = rows.iter().collect();
        let insights = analyze(&refs);
        assert!(insights.numeric.iter().all(|n| n.field != "speed"));
    }

    #[test]
    fn status_insight_lists_flagged_records_by_label() {
        let rows = [
            json!({"name": "web", "status": "up"}),
            json!({"name": "db", "status": "down"}),
        ];
        let refs: Vec<&Value> = rows.iter().collect();
        let insights = analyze(&refs);
        let status = insights.status.unwrap();
        assert_eq!(status.field, "status");
        assert_eq!(status.flagged, vec!["db".to_string()]);
    }

    #[test]
    fn hostname_is_recognized_as_a_label_field() {
        // Regression: "hostname" (distinct from "host") was missing from
        // LABEL_FIELDS, so flagged records fell back to "#2" instead of
        // the actual hostname.
        let rows = [
            json!({"hostname": "web-01", "status": "up"}),
            json!({"hostname": "web-02", "status": "down"}),
        ];
        let refs: Vec<&Value> = rows.iter().collect();
        let insights = analyze(&refs);
        assert_eq!(insights.status.unwrap().flagged, vec!["web-02".to_string()]);
    }

    #[test]
    fn record_label_falls_back_to_index_when_label_is_empty() {
        let row = json!({"name": ""});
        assert_eq!(record_label(&row, 0, Some("name")), "#1");
    }

    #[test]
    fn no_status_field_means_no_status_insight() {
        let rows = [json!({"a": 1})];
        let refs: Vec<&Value> = rows.iter().collect();
        assert!(analyze(&refs).status.is_none());
    }
}
