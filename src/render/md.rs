//! `md` mode — see docs/presentation-command.md §3.
//!
//! Renders a Markdown report: a heading, a summary line, and a table.
//! Cell values are escaped for Markdown table syntax (`|` and `\`) and
//! newlines are flattened to spaces, since either would corrupt the
//! table's column structure.
//!
//! Same scope limits as html mode (see render::html): no embedded
//! topology diagram or AI-generated recommendations — those need the
//! Graphviz and AI modules from renderer-architecture.md §3.3/§4, which
//! don't exist in this codebase yet.

use super::{
    DOWN_VALUES, RenderOptions, Renderer, STATUS_FIELDS, cell_text, columns, sorted_filtered,
};
use serde_json::Value;
use std::error::Error;

pub struct MarkdownRenderer;

impl Renderer for MarkdownRenderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>> {
        let rows = sorted_filtered(data, options);
        let columns = columns(&rows);

        let mut out = String::from("# Presentation Report\n\n");
        out.push_str(&summary(&rows, &columns));

        if columns.is_empty() {
            out.push_str("\n_No data._\n");
        } else {
            out.push('\n');
            out.push_str(&table(&rows, &columns));
        }

        Ok(out)
    }
}

fn summary(rows: &[&Value], columns: &[String]) -> String {
    let status_field = columns
        .iter()
        .find(|c| STATUS_FIELDS.contains(&c.to_ascii_lowercase().as_str()));

    let flagged = status_field.map(|field| {
        rows.iter()
            .filter(|row| {
                DOWN_VALUES.contains(&cell_text(row, field).to_ascii_lowercase().as_str())
            })
            .count()
    });

    match flagged {
        Some(0) | None => format!("{} record(s).\n", rows.len()),
        Some(n) => format!("{} record(s), **{n} flagged**.\n", rows.len()),
    }
}

fn table(rows: &[&Value], columns: &[String]) -> String {
    let mut out = String::new();

    out.push_str("| ");
    out.push_str(
        &columns
            .iter()
            .map(|c| escape(c))
            .collect::<Vec<_>>()
            .join(" | "),
    );
    out.push_str(" |\n| ");
    out.push_str(
        &columns
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | "),
    );
    out.push_str(" |\n");

    for row in rows {
        let cells: Vec<String> = columns
            .iter()
            .map(|c| {
                let raw = cell_text(row, c);
                let text = escape(&raw);
                if is_flagged(c, &raw) {
                    format!("**{text}**")
                } else {
                    text
                }
            })
            .collect();
        out.push_str("| ");
        out.push_str(&cells.join(" | "));
        out.push_str(" |\n");
    }

    out
}

fn is_flagged(column: &str, value: &str) -> bool {
    STATUS_FIELDS.contains(&column.to_ascii_lowercase().as_str())
        && DOWN_VALUES.contains(&value.to_ascii_lowercase().as_str())
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '\n' | '\r' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}
