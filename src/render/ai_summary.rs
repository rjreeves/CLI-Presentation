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
    fn empty_data_short_circuits_with_a_plain_message() {
        let out = AiSummaryRenderer {
            level: ExplainLevel::Brief,
        }
        .render(&[], &opts())
        .unwrap();
        assert_eq!(out, "No data to summarize.\n");
    }

    #[test]
    fn stays_a_single_line_even_at_detailed_level() {
        let data = vec![
            json!({"name": "web", "cpu": 10.0}),
            json!({"name": "db", "cpu": 90.0}),
        ];
        let out = AiSummaryRenderer {
            level: ExplainLevel::Detailed,
        }
        .render(&data, &opts())
        .unwrap();
        assert_eq!(out.lines().count(), 1);
    }

    #[test]
    fn detailed_adds_average_brief_does_not() {
        let data = vec![json!({"cpu": 10.0}), json!({"cpu": 90.0})];
        let brief = AiSummaryRenderer {
            level: ExplainLevel::Brief,
        }
        .render(&data, &opts())
        .unwrap();
        let detailed = AiSummaryRenderer {
            level: ExplainLevel::Detailed,
        }
        .render(&data, &opts())
        .unwrap();
        assert!(!brief.contains("avg"));
        assert!(detailed.contains("avg 50.0"));
    }

    #[test]
    fn omits_flagged_clause_when_nothing_is_down() {
        let data = vec![json!({"status": "up"})];
        let out = AiSummaryRenderer {
            level: ExplainLevel::Brief,
        }
        .render(&data, &opts())
        .unwrap();
        assert!(!out.contains("flagged"));
    }
}
