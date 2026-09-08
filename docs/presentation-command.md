# Presentation Command Specification (PCS)

**Version:** 1.0
**Status:** Draft — authoritative command-level specification
**Component:** `presentation` CLI renderer

## 1. Overview

`presentation` is a universal renderer for structured CLI output. It consumes JSON, YAML, or CSV/TSV from an upstream command and produces a human-readable artifact: a table, a dashboard, a chart, a diagram, a report, or an AI-generated summary.

It is a general-purpose pipeline sink, not a subsystem of any single tool. `win.exe` is its first and primary producer, but any CLI that emits [Presentation Protocol (PPS)](./presentation-protocol.md)-compatible data can use it — see that document for the cross-application contract.

> Presentation is the missing layer between raw CLI output and human comprehension.

## 2. Command Syntax

```
presentation [mode] [options]
```

`mode` is optional. When omitted, the renderer chooses a default based on the input's `schema` field (see [Renderer Auto-Selection](./presentation-protocol.md#5-renderer-selection-logic) in the protocol spec), falling back to `table`.

### Examples

```
win net ip --json | presentation table
win logs error --json | presentation timeline
win system perf --json | presentation chart cpu
win events query --json | presentation explain
win fs ls --json | presentation html
docker stats --format json | presentation dashboard
kubectl get pods -o json | presentation table
```

## 3. Modes

| Mode | Description | Typical use |
|---|---|---|
| `table` | Tabular rendering (sortable, color-coded status) | System info, IP lists, directory listings |
| `dashboard` | Live or static panel of metrics and gauges | Performance, health, container/cluster status |
| `chart` | Bar, line, or pie charts | CPU, memory, network usage over time |
| `timeline` | Chronological event visualization | Logs, system events, commit history |
| `map` | Topology or relationship diagrams | Network interfaces, dependency graphs |
| `html` | HTML report export | Documentation, sharing, support tickets |
| `md` | Markdown export | Reports, git-friendly documentation |
| `explain` | AI-generated natural-language explanation | Diagnostics, teaching, onboarding |
| `ai-summary` | Condensed AI narrative | Quick-glance insight from large output |

## 4. Options

| Flag | Description |
|---|---|
| `--theme <name>` | Color theme: `dark`, `light`, `solarized` |
| `--output <file>` | Write rendered output to a file |
| `--live` | Enable live refresh for dashboard/timeline modes |
| `--sort <field>` | Sort table output by field |
| `--filter <expr>` | Filter data before rendering |
| `--chart-type <bar\|line\|pie>` | Chart style, for `chart` mode |
| `--explain-level <brief\|detailed>` | AI verbosity, for `explain`/`ai-summary` |
| `--width <cols>` | Terminal width for layout |
| `--export <format>` | Export rendered output: `html`, `md`, `svg`, `pdf` |

## 5. Input Requirements

Accepted formats: JSON (preferred), YAML, CSV/TSV, Ion structured output.

Required field: `data` (an array of records).

Optional fields: `schema`, `source`, `timestamp`, `meta` — see the [Presentation Protocol](./presentation-protocol.md#3-data-format) for the full envelope definition.

```json
{
  "source": "win net ip",
  "timestamp": "2026-08-27T10:15:00Z",
  "data": [
    {
      "interface": "Ethernet",
      "ipv4": "192.168.1.10",
      "gateway": "192.168.1.1",
      "status": "Up",
      "speed": "1Gbps"
    }
  ]
}
```

## 6. Output Targets

- **Terminal:** ANSI color codes, Unicode box-drawing, optional Sixel/Kitty inline graphics
- **Web:** HTML5 + CSS + SVG, optional JS interactivity
- **AI:** natural-language summary, optionally Markdown-formatted

## 7. Rendering Pipeline

```
┌────────────┐
│ Input Data │  ← JSON/YAML/CSV
└──────┬─────┘
       │
┌──────▼─────┐
│ Parser     │  → Normalizes schema
└──────┬─────┘
       │
┌──────▼─────┐
│ Renderer   │  → TUI / Graphics / Web / AI
└──────┬─────┘
       │
┌──────▼─────┐
│ Output     │  → Terminal / HTML / File / AI text
└────────────┘
```

Full module breakdown, plugin trait, and technology stack live in [Renderer Architecture Blueprint](./renderer-architecture.md).

## 8. Extensibility

Third-party tools may:

- Register schemas so their output auto-selects a sensible renderer (see [Schema Registry](./schema-registry.md#4-schema-registration))
- Provide custom renderer plugins, compiled as WASM modules, implementing the `Renderer` trait
- Validate their output against the schema registry before shipping: `myapp --json | presentation validate`

## 9. Worked Example — `win net ip`

`win net ip` is the canonical demonstration case: interface name, IPv4/IPv6, subnet, gateway, DNS, DHCP status, link speed, MAC, state, metric, and (for Wi-Fi) SSID/signal — structured, predictable, and rich enough to exercise every mode.

| Mode | What it shows for `win net ip` |
|---|---|
| `table` | One row per interface: IPv4, IPv6, gateway, DNS, DHCP, speed, color-coded Up/Down, misconfiguration highlights |
| `map` | PC at center, branching to each interface → gateway → DNS/internet; virtual adapters grouped, disabled interfaces greyed out |
| `dashboard` (Wi-Fi) | SSID, signal-strength bar, channel/band, IP, gateway, DNS, security type, link speed |
| `dashboard` (health) | Interface status, DHCP lease age, DNS latency, gateway reachability, IPv6 default-route presence, packet error rate |
| `explain` | Plain-language read: "Your Ethernet adapter is active with a stable IPv4 lease," "IPv6 is configured but no default route is present" |
| `md` / `html` | Full interface detail, highlighted issues, embedded topology diagram, summary and recommendations sections |
| `--changes` (diff) | Compares two snapshots: IP/DNS/gateway changes, interface up/down transitions, DHCP renewals, new virtual adapters |
| `timeline` | IP changes, DHCP renewals, up/down events, DNS changes over stored history |
| `--compact` | One line per interface — IPv4 + gateway, IPv6 + gateway, DNS summary, status indicator — for logs/scripting |

## 10. Prior Art

No existing tool combines table/diagram/dashboard/AI-summary rendering into one unified CLI sink. Adjacent tools worth studying while implementing:

| Tool | Platform | Relevant capability |
|---|---|---|
| `nmap`/`zenmap` | Linux/Windows | Zenmap GUI visualizes topology from CLI scan data |
| `ip` (iproute2) | Linux | Structured interface/routing text, scriptable into visualizers |
| `traceroute`/`mtr` | All | ASCII path visualization; `mtr` adds live latency charts |
| `bmon`/`iftop`/`nload` | Linux | Terminal dashboards with live bandwidth graphs |
| `netdata` | Linux/macOS | Real-time web dashboard fed by a CLI agent |
| `glances` | Cross-platform | Terminal dashboard with network panels |
| `tshark` (Wireshark) | All | Structured packet data via CLI; GUI visualizes flows |
| `iptraf-ng` | Linux | Interactive text-based traffic dashboard |
| PowerShell `Get-NetIPAddress \| Out-GridView` | Windows | Grid-view table, scriptable into HTML reports |

Emerging patterns worth borrowing: Python `rich`/`textual` for custom terminal dashboards, Node `blessed`/`ink` for text-UI network maps, and the Netdata/Prometheus/Grafana stack for CLI-agent-to-web-dashboard pipelines.

## 11. Open Questions

- Exact live-refresh protocol for `--live` dashboards (poll vs. push from producer)
- Whether `explain`/`ai-summary` calls a local model or a hosted API by default, and how that's configured
- SVG/PDF export fidelity requirements for `--export`
