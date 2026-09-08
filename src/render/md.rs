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
    fn pipe_and_newline_in_a_value_do_not_corrupt_the_table() {
        let data = vec![json!({"interface": "Weird | Name\nWithNewline"})];
        let out = MarkdownRenderer.render(&data, &opts()).unwrap();
        let table_line = out.lines().find(|l| l.starts_with("| Weird")).unwrap();
        // One column -> exactly 3 pipes total: opening, the escaped '|'
        // inside the value, and closing. An unescaped '|' or a literal
        // embedded newline would add a phantom column or row instead.
        assert_eq!(table_line.matches('|').count(), 3);
        assert_eq!(table_line, r"| Weird \| Name WithNewline |");
    }

    #[test]
    fn down_status_is_bolded_up_status_is_not() {
        let data = vec![json!({"status": "up"}), json!({"status": "down"})];
        let out = MarkdownRenderer.render(&data, &opts()).unwrap();
        assert!(out.contains("**down**"));
        assert!(!out.contains("**up**"));
    }

    #[test]
    fn empty_data_shows_no_data_message() {
        let out = MarkdownRenderer.render(&[], &opts()).unwrap();
        assert!(out.contains("_No data._"));
        assert!(!out.contains('|'));
    }

    #[test]
    fn escape_handles_backslash_pipe_and_newline() {
        assert_eq!(escape(r"back\slash"), r"back\\slash");
        assert_eq!(escape("a|b"), r"a\|b");
        assert_eq!(escape("a\nb"), "a b");
    }

    #[test]
    fn is_flagged_only_for_status_like_columns_with_down_values() {
        assert!(is_flagged("status", "down"));
        assert!(!is_flagged("status", "up"));
        assert!(!is_flagged("other", "down"));
    }
}
