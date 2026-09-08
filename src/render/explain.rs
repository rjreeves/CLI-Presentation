//! `explain` mode — see docs/presentation-command.md §3 and §4
//! (`--explain-level`).
//!
//! This is a deterministic, rule-based generator over the data itself —
//! not a call to an LLM. renderer-architecture.md §3.3/§4 name Azure
//! OpenAI as the intended backend for `explain`/`ai-summary`, but
//! presentation-command.md §11 lists "whether explain/ai-summary calls a
//! local model or a hosted API by default, and how that's configured" as
//! an open question. Wiring a real hosted call needs an API key and a
//! config story neither doc resolves, and would make this build silently
//! network-dependent on an unconfigured external service. This gives the
//! mode real, useful behavior now without pretending to be an LLM.

use super::insights::{Insights, analyze, record_label};
use super::{RenderOptions, Renderer, cell_text, sorted_filtered};
use clap::ValueEnum;
use serde_json::Value;
use std::error::Error;

#[derive(Copy, Clone, ValueEnum)]
pub enum ExplainLevel {
    Brief,
    Detailed,
}

pub struct ExplainRenderer {
    pub level: ExplainLevel,
}

impl Renderer for ExplainRenderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>> {
        let rows = sorted_filtered(data, options);
        if rows.is_empty() {
            return Ok("No data to explain.\n".to_string());
        }
        let insights = analyze(&rows);
        Ok(match self.level {
            ExplainLevel::Brief => brief(&insights),
            ExplainLevel::Detailed => detailed(&rows, &insights),
        })
    }
}

fn brief(insights: &Insights) -> String {
    let mut out = format!("This dataset has {} record(s).\n", insights.total);
    if let Some(status) = &insights.status {
        if status.flagged.is_empty() {
            out.push_str(&format!(
                "All records look healthy on `{}`.\n",
                status.field
            ));
        } else {
            out.push_str(&format!(
                "{} record(s) are flagged on `{}`: {}.\n",
                status.flagged.len(),
                status.field,
                status.flagged.join(", ")
            ));
        }
    }
    for n in &insights.numeric {
        out.push_str(&format!(
            "`{}` ranges from {:.1} to {:.1} (average {:.1}).\n",
            n.field, n.min, n.max, n.avg
        ));
    }
    out
}

fn detailed(rows: &[&Value], insights: &Insights) -> String {
    let mut out = brief(insights);
    out.push('\n');
    for (i, row) in rows.iter().enumerate() {
        let label = record_label(row, i, insights.label_field.as_deref());
        let mut notes = Vec::new();

        if let Some(status) = &insights.status {
            let value = cell_text(row, &status.field);
            if !value.is_empty() {
                notes.push(format!("{} is {value}", status.field));
            }
        }
        for n in &insights.numeric {
            if let Some(v) = row.get(&n.field).and_then(Value::as_f64) {
                notes.push(format!(
                    "{} is {v:.1} ({})",
                    n.field,
                    magnitude(v, n.min, n.max)
                ));
            }
        }

        if notes.is_empty() {
            out.push_str(&format!("- {label}\n"));
        } else {
            out.push_str(&format!("- {label}: {}.\n", notes.join(", ")));
        }
    }
    out
}

fn magnitude(value: f64, min: f64, max: f64) -> &'static str {
    let pos = (value - min) / (max - min).max(f64::EPSILON);
    if pos >= 0.7 {
        "high"
    } else if pos <= 0.3 {
        "low"
    } else {
        "moderate"
    }
}
