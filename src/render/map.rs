//! `map` mode — see docs/presentation-command.md §3 and §9.
//!
//! Renders a fixed three-level topology: a "PC" root, one branch per
//! record (its interface/node name), then that node's gateway, then the
//! gateway's DNS/downstream targets — covering the `network.ip` worked
//! example. This is not a general graph renderer; arbitrary dependency
//! graphs (docs/presentation-command.md §3's other stated use case) would
//! need a real node/edge model, which is future work.

use super::{RenderOptions, Renderer, cell_text, pick_field, sorted_filtered, value_text};
use crossterm::style::{Color, Stylize};
use serde_json::Value;
use std::error::Error;

const NODE_FIELDS: &[&str] = &["interface", "name", "id"];
const GATEWAY_FIELDS: &[&str] = &["gateway", "parent", "target"];
const DNS_FIELDS: &[&str] = &["dns", "children", "targets"];
const STATUS_FIELDS: &[&str] = &["status", "state"];

pub struct MapRenderer;

impl Renderer for MapRenderer {
    fn render(&self, data: &[Value], options: &RenderOptions) -> Result<String, Box<dyn Error>> {
        let rows = sorted_filtered(data, options);
        let Some(first) = rows.first().and_then(|r| r.as_object()) else {
            return Ok(String::new());
        };

        let node_field = pick_field(first, NODE_FIELDS);
        let gateway_field = pick_field(first, GATEWAY_FIELDS);
        let dns_field = pick_field(first, DNS_FIELDS);
        let status_field = pick_field(first, STATUS_FIELDS);

        let mut out = String::from("PC\n");
        for (i, row) in rows.iter().enumerate() {
            let last_row = i + 1 == rows.len();
            let node_label = node_field
                .as_deref()
                .map(|f| cell_text(row, f))
                .unwrap_or_else(|| format!("#{}", i + 1));
            let status = status_field.as_deref().map(|f| cell_text(row, f));
            let node_display = style_node(&node_label, status.as_deref(), options.color);

            out.push_str(&format!(
                "{} {node_display}\n",
                if last_row { "└─" } else { "├─" }
            ));
            let child_prefix = if last_row { "   " } else { "│  " };

            let gateway = gateway_field
                .as_deref()
                .map(|f| cell_text(row, f))
                .filter(|v| !v.is_empty());
            let dns_list: Vec<String> = dns_field
                .as_deref()
                .and_then(|f| row.get(f))
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .map(value_text)
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            if let Some(gateway) = &gateway {
                let has_dns = !dns_list.is_empty();
                out.push_str(&format!(
                    "{child_prefix}{} {gateway}\n",
                    if has_dns { "├─" } else { "└─" }
                ));
                let leaf_prefix = format!("{child_prefix}{}", if has_dns { "│  " } else { "   " });
                push_leaves(&mut out, &leaf_prefix, &dns_list);
            } else {
                push_leaves(&mut out, child_prefix, &dns_list);
            }
        }

        Ok(out)
    }
}

fn push_leaves(out: &mut String, prefix: &str, leaves: &[String]) {
    for (i, leaf) in leaves.iter().enumerate() {
        let connector = if i + 1 == leaves.len() {
            "└─"
        } else {
            "├─"
        };
        out.push_str(&format!("{prefix}{connector} {leaf}\n"));
    }
}

fn style_node(label: &str, status: Option<&str>, color: bool) -> String {
    if !color {
        return label.to_string();
    }
    match status.map(str::to_ascii_lowercase).as_deref() {
        Some("up" | "active" | "ok" | "online") => label.to_string().with(Color::Green).to_string(),
        Some("down" | "disabled" | "inactive" | "offline") => {
            label.to_string().with(Color::DarkGrey).to_string()
        }
        _ => label.to_string(),
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
    fn builds_full_three_level_tree() {
        let data = vec![json!({
            "interface": "Ethernet",
            "gateway": "192.168.1.1",
            "dns": ["8.8.8.8", "1.1.1.1"],
            "status": "Up",
        })];
        let out = MapRenderer.render(&data, &opts()).unwrap();
        let expected = [
            "PC",
            "└─ Ethernet",
            "   ├─ 192.168.1.1",
            "   │  ├─ 8.8.8.8",
            "   │  └─ 1.1.1.1",
            "",
        ]
        .join("\n");
        assert_eq!(out, expected);
    }

    #[test]
    fn node_without_gateway_or_dns_is_a_bare_leaf() {
        let data = vec![json!({"interface": "vEthernet"})];
        let out = MapRenderer.render(&data, &opts()).unwrap();
        assert_eq!(out, "PC\n└─ vEthernet\n");
    }

    #[test]
    fn missing_node_name_falls_back_to_index_label() {
        let data = vec![json!({"gateway": "1.1.1.1"})];
        let out = MapRenderer.render(&data, &opts()).unwrap();
        assert!(out.contains("└─ #1"));
    }

    #[test]
    fn empty_dns_array_is_treated_like_no_dns() {
        let data = vec![json!({"interface": "eth0", "gateway": "1.1.1.1", "dns": []})];
        let out = MapRenderer.render(&data, &opts()).unwrap();
        // No DNS entries -> gateway is the last child, using the corner
        // connector rather than a branch connector.
        assert!(out.contains("└─ 1.1.1.1"));
        assert!(!out.contains("├─ 1.1.1.1"));
    }

    #[test]
    fn empty_data_renders_empty_string() {
        assert_eq!(MapRenderer.render(&[], &opts()).unwrap(), "");
    }

    #[test]
    fn style_node_without_color_returns_plain_label() {
        assert_eq!(style_node("eth0", Some("down"), false), "eth0");
    }
}
