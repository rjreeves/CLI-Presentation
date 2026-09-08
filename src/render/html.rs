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

use super::{
    DOWN_VALUES, RenderOptions, Renderer, STATUS_FIELDS, UP_VALUES, cell_text, columns,
    escape_html as escape, sorted_filtered,
};
use serde_json::Value;
use std::error::Error;

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
    fn script_tag_in_a_value_is_escaped() {
        let data = [json!({"interface": "<script>alert(1)</script>"})];
        let out = HtmlRenderer.render(&data, &opts()).unwrap();
        assert!(!out.contains("<script>alert(1)</script>"));
        assert!(out.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    }

    #[test]
    fn empty_data_shows_no_data_message() {
        let out = HtmlRenderer.render(&[], &opts()).unwrap();
        assert!(out.contains("No data."));
        assert!(!out.contains("<table>"));
    }

    #[test]
    fn flagged_count_appears_only_when_a_status_is_down() {
        let healthy = [json!({"status": "up"})];
        let out = HtmlRenderer.render(&healthy, &opts()).unwrap();
        assert!(!out.contains("flagged"));

        let unhealthy = [json!({"status": "up"}), json!({"status": "down"})];
        let out = HtmlRenderer.render(&unhealthy, &opts()).unwrap();
        assert!(out.contains("1 flagged"));
    }

    #[test]
    fn status_class_maps_up_and_down_values() {
        assert_eq!(status_class("status", "Up"), "flag-up");
        assert_eq!(status_class("status", "Down"), "flag-down");
        assert_eq!(status_class("status", "unknown"), "");
        assert_eq!(status_class("other", "down"), "");
    }
}
