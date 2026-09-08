//! `chart` mode — see docs/presentation-command.md §3 and §4 (`--chart-type`).
//!
//! Terminal-native ASCII rendering only; the Sixel/Kitty graphics path
//! from docs/renderer-architecture.md §7 ("adaptive rendering") is not
//! implemented yet.

use super::{RenderOptions, Renderer, sorted_filtered};
use clap::ValueEnum;
use crossterm::style::{Color, Stylize};
use serde_json::{Map, Value};
use std::error::Error;

const LABEL_FIELDS: &[&str] = &["name", "label", "interface", "timestamp"];
const VALUE_FIELDS: &[&str] = &["value", "usage", "count", "temperature"];
const SPARK: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const PALETTE: [Color; 6] = [
    Color::Cyan,
    Color::Magenta,
    Color::Yellow,
    Color::Green,
    Color::Blue,
    Color::Red,
];
/// Distinct fill glyphs so pie segments stay legible without color (piped
/// output, --output files, non-color terminals).
const GLYPHS: [char; 6] = ['█', '▓', '▒', '░', '▞', '▚'];

#[derive(Copy, Clone, ValueEnum)]
pub enum ChartType {
    Bar,
    Line,
    Pie,
}

pub struct ChartRenderer {
    pub chart_type: ChartType,
}

impl Renderer for ChartRenderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>> {
        let rows = sorted_filtered(data, options);
        let series = extract_series(&rows);
        if series.is_empty() {
            return Ok(String::new());
        }
        let width = options.width.unwrap_or(50).max(20);
        Ok(match self.chart_type {
            ChartType::Bar => bar_chart(&series, width, options.color),
            ChartType::Line => line_chart(&series, options.color),
            ChartType::Pie => pie_chart(&series, width, options.color),
        })
    }
}

fn pick_field(
    obj: &Map<String, Value>,
    preferred: &[&str],
    matches: impl Fn(&Value) -> bool,
) -> Option<String> {
    preferred
        .iter()
        .find(|f| obj.get(**f).is_some_and(&matches))
        .map(|f| f.to_string())
        .or_else(|| obj.iter().find(|(_, v)| matches(v)).map(|(k, _)| k.clone()))
}

/// Reduces each record to a `(label, value)` point by guessing the label and
/// value fields from the first row, preferring common names (§`LABEL_FIELDS`,
/// `VALUE_FIELDS`) and falling back to the first string/numeric field.
fn extract_series(rows: &[&Value]) -> Vec<(String, f64)> {
    let Some(first) = rows.first().and_then(|r| r.as_object()) else {
        return Vec::new();
    };
    let Some(value_field) = pick_field(first, VALUE_FIELDS, |v| v.as_f64().is_some()) else {
        return Vec::new();
    };
    let label_field = pick_field(first, LABEL_FIELDS, |v| v.is_string());

    rows.iter()
        .enumerate()
        .filter_map(|(i, row)| {
            let value = row.get(&value_field).and_then(Value::as_f64)?;
            let label = label_field
                .as_ref()
                .and_then(|f| row.get(f))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("#{}", i + 1));
            Some((label, value))
        })
        .collect()
}

fn bar_chart(series: &[(String, f64)], width: usize, color: bool) -> String {
    let max = series
        .iter()
        .map(|(_, v)| v.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let label_width = series
        .iter()
        .map(|(l, _)| l.chars().count())
        .max()
        .unwrap_or(0);
    let bar_width = width.saturating_sub(label_width + 12).max(10);

    let mut out = String::new();
    for (label, value) in series {
        let filled = ((value.abs() / max) * bar_width as f64).round() as usize;
        let bar = "█".repeat(filled);
        let bar = if color {
            bar.with(Color::Cyan).to_string()
        } else {
            bar
        };
        out.push_str(&format!(
            "{label:<label_width$} | {bar}{} {value:>8.2}\n",
            "░".repeat(bar_width - filled),
        ));
    }
    out
}

fn line_chart(series: &[(String, f64)], color: bool) -> String {
    let values: Vec<f64> = series.iter().map(|(_, v)| *v).collect();
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(f64::EPSILON);

    let spark: String = values
        .iter()
        .map(|v| {
            let idx = (((v - min) / range) * (SPARK.len() - 1) as f64).round() as usize;
            SPARK[idx.min(SPARK.len() - 1)]
        })
        .collect();
    let spark = if color {
        spark.with(Color::Cyan).to_string()
    } else {
        spark
    };

    format!(
        "{spark}\n{:<20} min {min:.2}   max {max:.2}   {:>20}\n",
        series.first().map(|(l, _)| l.as_str()).unwrap_or(""),
        series.last().map(|(l, _)| l.as_str()).unwrap_or(""),
    )
}

fn pie_chart(series: &[(String, f64)], width: usize, color: bool) -> String {
    let total = series
        .iter()
        .map(|(_, v)| v.abs())
        .sum::<f64>()
        .max(f64::EPSILON);
    let bar_width = width.min(60);

    let mut bar = String::new();
    let mut legend = String::new();
    let mut used = 0usize;
    for (i, (label, value)) in series.iter().enumerate() {
        let pct = value.abs() / total * 100.0;
        let seg = ((pct / 100.0 * bar_width as f64).round() as usize).min(bar_width - used);
        used += seg;
        let glyph = GLYPHS[i % GLYPHS.len()];
        let swatch = PALETTE[i % PALETTE.len()];
        let block = glyph.to_string().repeat(seg);
        bar.push_str(&if color {
            block.with(swatch).to_string()
        } else {
            block
        });
        let marker = if color {
            glyph.to_string().with(swatch).to_string()
        } else {
            glyph.to_string()
        };
        legend.push_str(&format!("{marker} {label}: {pct:.1}%\n"));
    }
    bar.push_str(&" ".repeat(bar_width.saturating_sub(used)));

    format!("[{bar}]\n{legend}")
}
