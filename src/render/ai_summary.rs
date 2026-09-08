//! `ai-summary` mode — see docs/presentation-command.md §3.
//!
//! Same rule-based approach as `explain` (render::explain) and the same
//! "not actually an LLM" caveat documented there. Where `explain`
//! itemizes per record at `--explain-level detailed`, `ai-summary` always
//! stays a single condensed paragraph — matching "condensed AI
//! narrative... quick-glance insight from large output" from the command
//! spec, as opposed to explain's itemized "diagnostics, teaching,
//! onboarding" framing.

use super::explain::ExplainLevel;
use super::insights::analyze;
use super::{RenderOptions, Renderer, sorted_filtered};
use serde_json::Value;
use std::error::Error;

pub struct AiSummaryRenderer {
    pub level: ExplainLevel,
}

impl Renderer for AiSummaryRenderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>> {
        let rows = sorted_filtered(data, options);
        if rows.is_empty() {
            return Ok("No data to summarize.\n".to_string());
        }
        let insights = analyze(&rows);
        let detailed = matches!(self.level, ExplainLevel::Detailed);

        let mut sentence = format!("{} record(s)", insights.total);
        if let Some(status) = &insights.status
            && !status.flagged.is_empty()
        {
            sentence.push_str(&format!(
                ", {} flagged on {} ({})",
                status.flagged.len(),
                status.field,
                status.flagged.join(", ")
            ));
        }
        for n in &insights.numeric {
            if detailed {
                sentence.push_str(&format!(
                    "; {} avg {:.1} (range {:.1}-{:.1})",
                    n.field, n.avg, n.min, n.max
                ));
            } else {
                sentence.push_str(&format!("; {} {:.1}-{:.1}", n.field, n.min, n.max));
            }
        }
        sentence.push_str(".\n");

        Ok(sentence)
    }
}
