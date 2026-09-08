//! `dashboard` mode — see docs/presentation-command.md §3.
//!
//! Renders each record as a boxed panel of `field: value` lines. Numeric
//! fields whose name suggests a 0-100 metric (signal, usage, battery, ...)
//! render as a gauge bar instead of a bare number.

use super::{RenderOptions, Renderer, sorted_filtered, value_text};
use crossterm::style::{Color, Stylize};
use serde_json::Value;
use std::error::Error;

const GAUGE_FIELDS: &[&str] = &[
    "signal", "usage", "percent", "cpu", "memory", "battery", "load",
];
const GAUGE_WIDTH: usize = 20;
const TITLE_FIELDS: &[&str] = &["name", "interface", "ssid", "host", "hostname", "id"];

pub struct DashboardRenderer;

impl Renderer for DashboardRenderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>> {
        let rows = sorted_filtered(data, options);
        let width = options.width.unwrap_or(44).max(GAUGE_WIDTH + 20);

        let mut out = String::new();
        for row in &rows {
            out.push_str(&panel(row, width, options.color));
        }
        Ok(out)
    }
}

fn panel(row: &Value, width: usize, color: bool) -> String {
    let obj = match row.as_object() {
        Some(o) => o,
        None => return String::new(),
    };

    let title = TITLE_FIELDS
        .iter()
        .find_map(|f| obj.get(*f).and_then(Value::as_str))
        .unwrap_or("panel")
        .to_string();

    let mut out = String::new();
    out.push_str(&format!(
        "┌─ {} {}\n",
        title,
        "─".repeat(width.saturating_sub(title.len() + 3))
    ));
    for (field, value) in obj {
        let line = if is_gauge_field(field) {
            value
                .as_f64()
                .map(|n| gauge_line(field, n, color))
                .unwrap_or_else(|| field_line(field, value))
        } else {
            field_line(field, value)
        };
        out.push_str(&format!("│ {line}\n"));
    }
    out.push_str(&format!("└{}\n", "─".repeat(width)));
    out
}

fn is_gauge_field(field: &str) -> bool {
    let lower = field.to_ascii_lowercase();
    GAUGE_FIELDS.iter().any(|g| lower.contains(g))
}

fn field_line(field: &str, value: &Value) -> String {
    format!("{field}: {}", value_text(value))
}

fn gauge_line(field: &str, value: f64, color: bool) -> String {
    let clamped = value.clamp(0.0, 100.0);
    let filled = ((clamped / 100.0) * GAUGE_WIDTH as f64).round() as usize;
    let bar = format!(
        "[{}{}]",
        "█".repeat(filled),
        "░".repeat(GAUGE_WIDTH - filled)
    );
    let bar = if color { colorize(&bar, clamped) } else { bar };
    format!("{field}: {bar} {clamped:.0}%")
}

fn colorize(bar: &str, value: f64) -> String {
    if value >= 70.0 {
        bar.to_string().with(Color::Green).to_string()
    } else if value >= 40.0 {
        bar.to_string().with(Color::Yellow).to_string()
    } else {
        bar.to_string().with(Color::Red).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn opts() -> RenderOptions {
        RenderOptions {
            sort: None,
            filter: None,
            width: None,
            color: false,
        }
    }

    #[test]
    fn is_gauge_field_matches_by_substring_case_insensitively() {
        assert!(is_gauge_field("cpu_percent"));
        assert!(is_gauge_field("CPU_Usage"));
        assert!(is_gauge_field("signal"));
        assert!(!is_gauge_field("hostname"));
    }

    #[test]
    fn gauge_line_clamps_above_100_and_below_0() {
        assert!(gauge_line("x", 150.0, false).ends_with("100%"));
        assert!(gauge_line("x", -20.0, false).ends_with("0%"));
    }

    #[test]
    fn gauge_line_full_bar_at_100_percent() {
        let line = gauge_line("x", 100.0, false);
        assert_eq!(line.matches('█').count(), GAUGE_WIDTH);
        assert_eq!(line.matches('░').count(), 0);
    }

    #[test]
    fn title_prefers_hostname_over_generic_fallback() {
        let data = vec![json!({"hostname": "web-01", "cpu_usage": 10})];
        let out = DashboardRenderer.render(&data, &opts()).unwrap();
        assert!(out.starts_with("┌─ web-01 "));
    }

    #[test]
    fn title_falls_back_to_panel_when_no_recognized_field() {
        let data = vec![json!({"note": "x"})];
        let out = DashboardRenderer.render(&data, &opts()).unwrap();
        assert!(out.starts_with("┌─ panel "));
    }

    #[test]
    fn numeric_gauge_field_renders_as_bar_not_bare_number() {
        let data = vec![json!({"cpu_usage": 50})];
        let out = DashboardRenderer.render(&data, &opts()).unwrap();
        assert!(out.contains("cpu_usage: [") && out.contains('%'));
    }

    #[test]
    fn non_numeric_gauge_named_field_falls_back_to_plain_field_line() {
        // "cpu_usage" matches the gauge-name heuristic but isn't a number,
        // so it should render as a plain "field: value" line, not panic.
        let data = vec![json!({"cpu_usage": "n/a"})];
        let out = DashboardRenderer.render(&data, &opts()).unwrap();
        assert!(out.contains("cpu_usage: n/a"));
    }

    #[test]
    fn empty_data_renders_no_panels() {
        assert_eq!(DashboardRenderer.render(&[], &opts()).unwrap(), "");
    }
}
