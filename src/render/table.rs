//! `table` mode — see docs/presentation-command.md §3.

use super::{RenderOptions, Renderer};
use crossterm::style::{Color, Stylize};
use serde_json::Value;
use std::error::Error;

pub struct TableRenderer;

impl Renderer for TableRenderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>> {
        let mut rows: Vec<&Value> = data.iter().filter(|v| passes_filter(v, options)).collect();

        if let Some(field) = &options.sort {
            rows.sort_by(|a, b| compare_field(a, field, b, field));
        }

        let columns = columns(&rows);
        if columns.is_empty() {
            return Ok(String::new());
        }

        let col_width = options.width.map(|w| (w / columns.len()).max(4));
        let widths: Vec<usize> = columns
            .iter()
            .map(|c| {
                let natural = rows
                    .iter()
                    .map(|r| cell_text(r, c).len())
                    .chain(std::iter::once(c.len()))
                    .max()
                    .unwrap_or(c.len());
                col_width.map_or(natural, |cap| natural.min(cap))
            })
            .collect();

        let mut out = String::new();
        out.push_str(&format_row(&columns, &widths));
        out.push('\n');
        out.push_str(
            &"-".repeat(widths.iter().sum::<usize>() + (widths.len().saturating_sub(1)) * 3),
        );
        out.push('\n');
        for row in &rows {
            let cells: Vec<String> = columns
                .iter()
                .zip(&widths)
                .map(|(c, w)| truncate(&cell_text(row, c), *w))
                .collect();
            out.push_str(&format_colored_row(
                &columns,
                &cells,
                &widths,
                options.color,
            ));
            out.push('\n');
        }

        Ok(out)
    }
}

fn columns(rows: &[&Value]) -> Vec<String> {
    rows.first()
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

fn cell_text(row: &Value, field: &str) -> String {
    match row.get(field) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string(),
    }
}

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_string()
    } else if width == 0 {
        String::new()
    } else {
        let head: String = s.chars().take(width - 1).collect();
        format!("{head}…")
    }
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

fn format_row(columns: &[String], widths: &[usize]) -> String {
    columns
        .iter()
        .zip(widths)
        .map(|(c, w)| format!("{:<width$}", c, width = w))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn format_colored_row(
    columns: &[String],
    cells: &[String],
    widths: &[usize],
    color: bool,
) -> String {
    columns
        .iter()
        .zip(cells)
        .zip(widths)
        .map(|((col, cell), w)| {
            let padded = format!("{:<width$}", cell, width = w);
            if color {
                colorize(col, cell, padded)
            } else {
                padded
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn colorize(column: &str, value: &str, padded: String) -> String {
    if !matches!(column.to_ascii_lowercase().as_str(), "status" | "state") {
        return padded;
    }
    match value.to_ascii_lowercase().as_str() {
        "up" | "active" | "ok" | "online" => padded.with(Color::Green).to_string(),
        "down" | "error" | "offline" | "inactive" => padded.with(Color::Red).to_string(),
        _ => padded,
    }
}
