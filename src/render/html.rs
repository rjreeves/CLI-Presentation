//! `html` mode — see docs/presentation-command.md §3 and §9.
//!
//! Renders a self-contained, styled HTML report: a summary line plus a
//! data table, with status/level-like columns highlighted. All record
//! values are HTML-escaped before interpolation — payloads may come from
//! untrusted producers, and docs/presentation-protocol.md §11 requires
//! "HTML output is sanitized before being written or displayed".
//!
//! The richer report described in presentation-command.md §9 (embedded
//! topology diagram, AI-generated recommendations) needs the Graphviz and
//! AI modules from renderer-architecture.md §3.3/§4, neither of which
//! exist in this codebase yet — out of scope here.

use super::{RenderOptions, Renderer, cell_text, columns, sorted_filtered};
use serde_json::Value;
use std::error::Error;

const STATUS_FIELDS: &[&str] = &["status", "state", "level"];
const UP_VALUES: &[&str] = &["up", "active", "ok", "online", "info"];
const DOWN_VALUES: &[&str] = &["down", "error", "offline", "inactive", "critical", "fatal"];

pub struct HtmlRenderer;

impl Renderer for HtmlRenderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>> {
        let rows = sorted_filtered(data, options);
        let columns = columns(&rows);

        let mut body = String::new();
        body.push_str("<h1>Presentation Report</h1>\n");
        body.push_str(&summary(&rows, &columns));

        if columns.is_empty() {
            body.push_str("<p class=\"empty\">No data.</p>\n");
        } else {
            body.push_str(&table(&rows, &columns));
        }

        Ok(document(&body))
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
        Some(0) | None => format!("<p class=\"summary\">{} record(s).</p>\n", rows.len()),
        Some(n) => format!(
            "<p class=\"summary\">{} record(s), <span class=\"flag-down\">{n} flagged</span>.</p>\n",
            rows.len()
        ),
    }
}

fn table(rows: &[&Value], columns: &[String]) -> String {
    let mut out = String::from("<table>\n<thead><tr>");
    for c in columns {
        out.push_str(&format!("<th>{}</th>", escape(c)));
    }
    out.push_str("</tr></thead>\n<tbody>\n");
    for row in rows {
        out.push_str("<tr>");
        for c in columns {
            let text = cell_text(row, c);
            let class = status_class(c, &text);
            out.push_str(&format!("<td class=\"{class}\">{}</td>", escape(&text)));
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</tbody>\n</table>\n");
    out
}

fn status_class(column: &str, value: &str) -> &'static str {
    if !STATUS_FIELDS.contains(&column.to_ascii_lowercase().as_str()) {
        return "";
    }
    let value = value.to_ascii_lowercase();
    if UP_VALUES.contains(&value.as_str()) {
        "flag-up"
    } else if DOWN_VALUES.contains(&value.as_str()) {
        "flag-down"
    } else {
        ""
    }
}

fn escape(s: &str) -> String {
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

fn document(body: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Presentation Report</title>
<style>
  body {{ font-family: system-ui, sans-serif; margin: 2rem; color: #1a1a1a; background: #fff; }}
  h1 {{ font-size: 1.4rem; }}
  table {{ border-collapse: collapse; width: 100%; margin-top: 1rem; }}
  th, td {{ border: 1px solid #ddd; padding: 0.4rem 0.6rem; text-align: left; font-size: 0.9rem; }}
  th {{ background: #f5f5f5; }}
  .summary {{ color: #555; }}
  .empty {{ color: #888; font-style: italic; }}
  .flag-up {{ color: #1a7f37; font-weight: 600; }}
  .flag-down {{ color: #b3261e; font-weight: 600; }}
</style>
</head>
<body>
{body}</body>
</html>
"#
    )
}
