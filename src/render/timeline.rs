//! `timeline` mode — see docs/presentation-command.md §3.
//!
//! Renders records as a chronological event list. Timestamp/level/message/
//! source fields are guessed by name (matching the `logs.events` schema in
//! schema-registry.md, but not requiring it), and — unless the caller passed
//! an explicit `--sort` — rows default-sort by the timestamp field, since
//! "chronological" is the point of this mode.

use super::{RenderOptions, Renderer, cell_text, pick_field, sorted_filtered, truncate};
use crossterm::style::{Color, Stylize};
use serde_json::Value;
use std::error::Error;

const TIME_FIELDS: &[&str] = &["timestamp", "time", "date"];
const LEVEL_FIELDS: &[&str] = &["level", "severity"];
const MESSAGE_FIELDS: &[&str] = &["message", "event", "description", "summary"];
const SOURCE_FIELDS: &[&str] = &["source", "host", "origin"];

pub struct TimelineRenderer;

impl Renderer for TimelineRenderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>> {
        let mut rows = sorted_filtered(data, options);
        let Some(first) = rows.first().and_then(|r| r.as_object()) else {
            return Ok(String::new());
        };

        let time_field = pick_field(first, TIME_FIELDS);
        let level_field = pick_field(first, LEVEL_FIELDS);
        let message_field = pick_field(first, MESSAGE_FIELDS);
        let source_field = pick_field(first, SOURCE_FIELDS);

        if options.sort.is_none()
            && let Some(field) = &time_field
        {
            rows.sort_by_key(|row| cell_text(row, field));
        }

        let used = [&time_field, &level_field, &message_field, &source_field];
        let entries: Vec<Entry> = rows
            .iter()
            .map(|row| Entry {
                time: time_field
                    .as_deref()
                    .map(|f| cell_text(row, f))
                    .unwrap_or_default(),
                level: level_field.as_deref().map(|f| cell_text(row, f)),
                message: message_field
                    .as_deref()
                    .map(|f| cell_text(row, f))
                    .unwrap_or_else(|| fallback_message(row, &used)),
                source: source_field.as_deref().map(|f| cell_text(row, f)),
            })
            .collect();

        let time_width = entries
            .iter()
            .map(|e| e.time.chars().count())
            .max()
            .unwrap_or(0);
        let level_width = entries
            .iter()
            .map(|e| e.level.as_deref().map_or(0, |l| l.chars().count() + 2))
            .max()
            .unwrap_or(0);
        let message_width = options
            .width
            .map(|w| w.saturating_sub(time_width + level_width + 10).max(10));

        let mut out = String::new();
        if !entries.is_empty() {
            out.push_str("│\n");
        }
        for (i, entry) in entries.iter().enumerate() {
            let connector = if i + 1 == entries.len() {
                "└─"
            } else {
                "├─"
            };

            let badge_text = entry
                .level
                .as_deref()
                .map_or(String::new(), |l| format!("[{}]", l.to_ascii_uppercase()));
            let badge_padded = format!("{badge_text:<level_width$}");
            let badge = if options.color {
                colorize_badge(&badge_padded, entry.level.as_deref())
            } else {
                badge_padded
            };

            let message = message_width
                .map_or_else(|| entry.message.clone(), |w| truncate(&entry.message, w));
            let source_suffix = entry
                .source
                .as_deref()
                .map(|s| format!("  ({s})"))
                .unwrap_or_default();

            out.push_str(&format!(
                "{connector} {:<time_width$}  {badge}  {message}{source_suffix}\n",
                entry.time,
            ));
        }

        Ok(out)
    }
}

struct Entry {
    time: String,
    level: Option<String>,
    message: String,
    source: Option<String>,
}

fn fallback_message(row: &Value, used: &[&Option<String>]) -> String {
    let Some(obj) = row.as_object() else {
        return String::new();
    };
    obj.iter()
        .filter(|(k, _)| !used.iter().any(|u| u.as_deref() == Some(k.as_str())))
        .map(|(k, v)| format!("{k}={}", super::value_text(v)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Colors an already width-padded badge. Padding must happen first: ANSI
/// escape codes count as characters for `{:<width$}`, so coloring before
/// padding would under-pad the visible text.
fn colorize_badge(padded: &str, level: Option<&str>) -> String {
    let Some(level) = level else {
        return padded.to_string();
    };
    match level.to_ascii_lowercase().as_str() {
        "error" | "fatal" | "critical" => padded.to_string().with(Color::Red).to_string(),
        "warn" | "warning" => padded.to_string().with(Color::Yellow).to_string(),
        "info" => padded.to_string().with(Color::Cyan).to_string(),
        "debug" | "trace" => padded.to_string().with(Color::DarkGrey).to_string(),
        _ => padded.to_string(),
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
    fn defaults_to_chronological_order_when_sort_is_unset() {
        let data = vec![
            json!({"timestamp": "10:05", "message": "later"}),
            json!({"timestamp": "10:00", "message": "earlier"}),
        ];
        let out = TimelineRenderer.render(&data, &opts()).unwrap();
        let earlier_pos = out.find("earlier").unwrap();
        let later_pos = out.find("later").unwrap();
        assert!(earlier_pos < later_pos);
    }

    #[test]
    fn explicit_sort_overrides_chronological_default() {
        let data = vec![
            json!({"timestamp": "10:00", "source": "b"}),
            json!({"timestamp": "10:05", "source": "a"}),
        ];
        let mut options = opts();
        options.sort = Some("source".to_string());
        let out = TimelineRenderer.render(&data, &options).unwrap();
        let a_pos = out.find("(a)").unwrap();
        let b_pos = out.find("(b)").unwrap();
        assert!(a_pos < b_pos);
    }

    #[test]
    fn missing_message_field_falls_back_to_key_value_join() {
        let data = vec![json!({"timestamp": "t1", "action": "deploy", "user": "alice"})];
        let out = TimelineRenderer.render(&data, &opts()).unwrap();
        assert!(out.contains("action=deploy"));
        assert!(out.contains("user=alice"));
    }

    #[test]
    fn last_entry_uses_corner_connector() {
        let data = vec![json!({"timestamp": "t1", "message": "only"})];
        let out = TimelineRenderer.render(&data, &opts()).unwrap();
        assert!(out.contains("└─"));
        assert!(!out.contains("├─"));
    }

    #[test]
    fn empty_data_renders_empty_string() {
        assert_eq!(TimelineRenderer.render(&[], &opts()).unwrap(), "");
    }

    #[test]
    fn colorize_badge_leaves_padding_intact() {
        // Color must wrap the already-padded text verbatim, not re-derive
        // it, otherwise trailing padding spaces could be lost.
        let padded = "[INFO]   ";
        let colored = colorize_badge(padded, Some("info"));
        assert!(colored.contains(padded));
    }
}
