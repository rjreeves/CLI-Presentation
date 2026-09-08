# Renderer Architecture Blueprint (RAB)

**Version:** 1.0
**Status:** Draft — internal developer architecture
**Component:** `presentation` renderer engine

## 1. Overview

This document defines the internal architecture of the `presentation` renderer: how a [PPS-compliant](./presentation-protocol.md) payload moves from stdin to a rendered artifact. It is the implementation counterpart to the [Presentation Command Specification](./presentation-command.md) and the [Schema Registry](./schema-registry.md).

## 2. High-Level Diagram

```
┌──────────────────────────┐
│      Data Sources        │
│ win.exe, docker, kubectl │
└──────────────┬───────────┘
               │ JSON/YAML
┌──────────────▼───────────┐
│       Input Parser        │
│ Normalizes schemas        │
└──────────────┬───────────┘
               │
┌──────────────▼───────────┐
│      Rendering Engine     │
│ TUI, Graphics, Web, AI    │
└──────────────┬───────────┘
               │
┌──────────────▼───────────┐
│         Output            │
│ Terminal, HTML, SVG, AI   │
└───────────────────────────┘
```

## 3. Modules

### 3.1 Parser Module

- Detects input format: JSON, YAML, or CSV
- Validates PPS compliance (presence of `data`; well-formed `schema`/`source`/`timestamp`/`meta` if present)
- Extracts `schema` and looks it up in the [Schema Registry](./schema-registry.md)
- Normalizes records into the internal data model consumed by renderers

### 3.2 Renderer Module

Implements one concrete renderer per mode, all behind a common trait (§5):

- `TableRenderer`
- `DashboardRenderer`
- `ChartRenderer`
- `MapRenderer`
- `TimelineRenderer`
- `HtmlRenderer`
- `ExplainRenderer`

### 3.3 AI Module

- LLM API integration (see [§4](#4-technology-stack) for default provider)
- Summary generation (`ai-summary` mode)
- Explanation generation (`explain` mode), with `--explain-level brief|detailed`

### 3.4 Output Module

- Terminal: ANSI escape codes, Unicode box-drawing
- Graphics: Sixel / Kitty Graphics Protocol inline images
- Web: HTML/SVG file or stream
- File export: `--output`/`--export` targets (html, md, svg, pdf)

## 4. Technology Stack

Chosen for portability, terminal-graphics support, and a single-binary distribution story consistent with the rest of the `win.exe`/Ion toolchain.

| Layer | Technology | Notes |
|---|---|---|
| Core | Rust | Safety, no GC pauses, predictable latency — consistent with the [desktopdb](../desktopdb.md) engine choice |
| TUI | `ratatui` (+ `crossterm`) | Terminal dashboards, tables, live panels |
| Terminal graphics | `sixel-rs`, `plotters` | Inline charts/diagrams where the terminal supports Sixel/Kitty |
| Web | `askama` (templating), `warp`, Graphviz | HTML/SVG report generation; Graphviz for topology diagrams |
| AI | Azure OpenAI API (default), pluggable | `explain`/`ai-summary` modes |
| Config/serialization | `serde`, `serde_json`, `toml` | Payload parsing, registry files |

### 4.1 Stack by capability (reference)

If a capability needs to be implemented independently of the unified stack above, these are the alternatives considered:

- **Terminal-native (tables/dashboards/gauges):** Rust `ratatui`/`crossterm`; Go `tview`/`bubbletea`; C# `Spectre.Console`; Python `rich`/`textual`
- **Graphics-capable terminal (inline charts/diagrams):** Sixel, Kitty Graphics Protocol, iTerm2 inline images, Windows Terminal GPU rendering; Rust `sixel-rs`/`image`/`plotters`, Go `go-sixel`/`go-chart`, Python `matplotlib` → Sixel
- **Web-based (HTML/SVG/dashboards):** Rust+Yew (WASM), C#+Blazor (WASM), Node+React, Python Flask/FastAPI; D3.js, Chart.js, Mermaid.js, Graphviz, ECharts

## 5. Plugin System

Renderers implement a common trait so new modes — first-party or third-party — plug into the same pipeline:

```rust
trait Renderer {
    fn render(&self, data: Value, options: RenderOptions) -> Result<RenderOutput>;
}
```

- Third-party renderers are registered via `presentation register --renderer myapp-renderer.wasm` (see [Presentation Protocol §6.2](./presentation-protocol.md#62-custom-renderer-plugins))
- Renderers compile to WASM for sandboxing and portability across Windows/Linux/macOS
- The registry (see [Schema Registry](./schema-registry.md)) maps a schema's `default_renderer` to a concrete `Renderer` implementation at dispatch time

## 6. Error Model

| Code | Meaning |
|---|---|
| `ERR_PARSE_FAILED` | Input could not be parsed as JSON/YAML/CSV |
| `ERR_SCHEMA_UNKNOWN` | `schema` field present but not found in any registry |
| `ERR_RENDERER_NOT_FOUND` | Requested/selected renderer is not registered |
| `ERR_OUTPUT_FAILED` | Rendering succeeded but writing output (file, terminal, network) failed |

Schema-validation-specific error codes (`SRD_*`) are defined in the [Schema Registry §11](./schema-registry.md#11-error-codes).

## 7. Non-Obvious Design Opportunities

Captured from early design discussion — not committed, but worth carrying into implementation planning:

- **Self-describing pipelines:** producers embed enough metadata that the renderer picks the best visualization without an explicit mode flag (formalized as schema auto-selection, §5 of the protocol spec).
- **Adaptive rendering:** if the terminal supports Sixel/Kitty graphics, render a chart; otherwise fall back to ASCII art.
- **Presentation as a debugging tool:** pipe logs into a timeline viewer, perf counters into a live dashboard, registry/config diffs into a tree view.
- **Presentation as a teaching tool:** `explain` mode annotates command output and pipelines for newcomers, not just end results.

## 8. Future Extensions

- `presentation --slides` — auto-generate HTML slide decks from structured output
- `presentation --report` — PDF export
- `presentation --interactive` — clickable/expandable dashboards, filterable tables
- `presentation --teach` — deeper AI walkthroughs of what a command's output means
