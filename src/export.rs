//! `--export <format>` — see docs/presentation-command.md §4.
//!
//! Wraps *any* mode's rendered text output in a container format. This is
//! distinct from selecting `html`/`md` as the render mode itself
//! (render::html, render::md), which produce a richer, data-aware report
//! rather than a wrapped dump of another mode's text — e.g.
//! `presentation dashboard --export html` wraps the dashboard's ASCII
//! panels in a `<pre>` block, while `presentation html` builds an actual
//! HTML table from the data.
//!
//! `pdf` is listed in the docs but not implemented: real PDF generation
//! needs either a dedicated crate or hand-rolling the binary format,
//! either of which is a bigger call than this scaffold should make
//! silently. Selecting it errors clearly instead of writing something
//! broken with a `.pdf` name.

use crate::render::escape_html;
use clap::ValueEnum;
use std::error::Error;

#[derive(Copy, Clone, ValueEnum)]
pub enum ExportFormat {
    Html,
    Md,
    Svg,
    Pdf,
}

pub fn apply(format: ExportFormat, rendered: &str) -> Result<String, Box<dyn Error>> {
    match format {
        ExportFormat::Html => Ok(html(rendered)),
        ExportFormat::Md => Ok(md(rendered)),
        ExportFormat::Svg => Ok(svg(rendered)),
        ExportFormat::Pdf => Err(
            "--export pdf is not implemented: PDF generation needs a dedicated dependency this build doesn't include"
                .into(),
        ),
    }
}

fn html(rendered: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Presentation Export</title>
<style>
  body {{ margin: 2rem; background: #fff; color: #1a1a1a; }}
  pre {{ font-family: ui-monospace, Consolas, monospace; font-size: 0.9rem; white-space: pre-wrap; }}
</style>
</head>
<body>
<pre>{}</pre>
</body>
</html>
"#,
        escape_html(rendered)
    )
}

/// Wraps `rendered` in a fenced code block, using a fence one backtick
/// longer than the longest backtick run already present in the content —
/// otherwise embedded backticks (however unlikely in practice) could
/// prematurely close the fence.
fn md(rendered: &str) -> String {
    let longest = rendered
        .lines()
        .map(longest_backtick_run)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat((longest + 1).max(3));
    let mut body = rendered.to_string();
    if !body.ends_with('\n') {
        body.push('\n');
    }
    format!("{fence}text\n{body}{fence}\n")
}

fn longest_backtick_run(line: &str) -> usize {
    let mut max_run = 0;
    let mut current = 0;
    for c in line.chars() {
        if c == '`' {
            current += 1;
            max_run = max_run.max(current);
        } else {
            current = 0;
        }
    }
    max_run
}

fn svg(rendered: &str) -> String {
    const CHAR_WIDTH: f64 = 8.0;
    const LINE_HEIGHT: f64 = 18.0;
    const FONT_SIZE: u32 = 14;
    const PADDING: f64 = 10.0;

    let lines: Vec<&str> = rendered.lines().collect();
    let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let width = (max_len as f64 * CHAR_WIDTH + PADDING * 2.0).max(200.0);
    let height = (lines.len() as f64 * LINE_HEIGHT + PADDING * 2.0).max(40.0);

    let mut text_elements = String::new();
    for (i, line) in lines.iter().enumerate() {
        let y = PADDING + FONT_SIZE as f64 + i as f64 * LINE_HEIGHT;
        text_elements.push_str(&format!(
            "  <text x=\"{PADDING}\" y=\"{y:.1}\" font-family=\"monospace\" font-size=\"{FONT_SIZE}\" fill=\"#1a1a1a\" xml:space=\"preserve\">{}</text>\n",
            escape_html(line)
        ));
    }

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.0}\" height=\"{height:.0}\" viewBox=\"0 0 {width:.0} {height:.0}\">\n  <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n{text_elements}</svg>\n"
    )
}
