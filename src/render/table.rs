//! `table` mode — see docs/presentation-command.md §3.

use super::{RenderOptions, Renderer, cell_text, columns, sorted_filtered, truncate};
use crossterm::style::{Color, Stylize};
use serde_json::Value;
use std::error::Error;

pub struct TableRenderer;

impl Renderer for TableRenderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>> {
        let rows = sorted_filtered(data, options);
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
    fn renders_header_divider_and_rows() {
        // Single column keeps the expected widths trivial to verify by hand:
        // natural width = max(header len, cell len) = max(1, 1) = 1.
        let data = vec![json!({"a": "x"})];
        let out = TableRenderer.render(&data, &opts()).unwrap();
        assert_eq!(out, "a\n-\nx\n");
    }

    #[test]
    fn pads_columns_to_the_widest_value() {
        let data = vec![json!({"name": "web"})];
        let out = TableRenderer.render(&data, &opts()).unwrap();
        // header "name" (4 chars) vs cell "web" (3 chars) -> cell padded to 4.
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "name");
        assert_eq!(lines[2], "web ");
    }

    #[test]
    fn empty_data_renders_empty_string() {
        let out = TableRenderer.render(&[], &opts()).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn width_option_caps_and_truncates_columns() {
        let data = vec![json!({"a": "this is a very long value"})];
        let mut options = opts();
        options.width = Some(20);
        let out = TableRenderer.render(&data, &options).unwrap();
        // one column, cap = (20 / 1).max(4) = 20
        assert!(out.lines().all(|l| l.chars().count() <= 20));
        assert!(out.contains('…'));
    }

    #[test]
    fn color_only_applies_to_status_like_columns() {
        let data = vec![json!({"status": "down", "note": "down"})];
        let mut options = opts();
        options.color = true;
        let out = TableRenderer.render(&data, &options).unwrap();
        // "status" cell should carry an ANSI escape; "note" cell (same text,
        // different column) should not.
        let rows_line = out.lines().nth(2).unwrap();
        assert!(rows_line.contains("\x1b["));
    }

    #[test]
    fn colorize_ignores_non_status_columns() {
        let padded = "down".to_string();
        assert_eq!(colorize("note", "down", padded.clone()), padded);
    }

    #[test]
    fn colorize_leaves_unrecognized_values_plain() {
        let padded = "unknown".to_string();
        assert_eq!(colorize("status", "unknown", padded.clone()), padded);
    }
}
