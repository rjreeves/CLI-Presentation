# Presentation Protocol Specification (PPS)

**Version:** 1.0
**Status:** Draft — cross-application data contract
**Component:** Presentation Protocol

## 1. Purpose

The Presentation Protocol defines how any CLI tool emits structured data so that the [`presentation`](./presentation-command.md) command (or any compliant renderer) can turn it into tables, dashboards, diagrams, reports, or AI summaries.

It is tool-agnostic, platform-agnostic, and shell-agnostic. Any producer can participate:

```
myapp --json | presentation table
docker stats --format json | presentation dashboard
kubectl get pods -o json | presentation chart
win net ip --json | presentation map
```

## 2. Core Principles

1. **Universal** — any app can emit PPS-compatible JSON/YAML/CSV; the renderer does not care who produced it.
2. **Minimal** — only one field is required: `data`.
3. **Schema is optional but recommended** — a `schema` hint enables auto-selection of the best renderer.
4. **Non-intrusive** — producers need no knowledge of `presentation` internals; they just emit structured data.
5. **Extensible** — third parties register their own schemas and renderers without modifying the core.

## 3. Data Format

### 3.1 Required

```json
{ "data": [...] }
```

### 3.2 Full envelope

```json
{
  "pps_version": "1.0",
  "schema": "network.ip",
  "source": "win net ip",
  "timestamp": "2026-08-27T10:15:00Z",
  "meta": {
    "host": "DESKTOP-123",
    "user": "robert"
  },
  "data": [...]
}
```

| Field | Required | Purpose |
|---|---|---|
| `data` | yes | The structured content — an array of records |
| `pps_version` | no | Protocol version this payload conforms to (see [§12](#12-protocol-versioning)) |
| `schema` | no | Identifies data type for renderer auto-selection |
| `source` | no | Name of the upstream command that produced this payload |
| `timestamp` | no | When the data was generated |
| `meta` | no | Arbitrary producer-defined metadata |

## 4. Schema Naming Convention

Schemas use a dotted hierarchical pattern:

```
<domain>.<category>[.<subtype>]
```

Examples: `network.ip`, `network.wifi`, `system.process`, `system.perf.cpu`, `logs.events`, `fs.directory`, `docker.stats`, `k8s.pods`.

Full definitions of built-in schemas live in the [Schema Registry](./schema-registry.md).

## 5. Renderer Selection Logic

### 5.1 Explicit mode

The user names the renderer directly:

```
presentation table
presentation map
presentation dashboard
```

### 5.2 Schema-based auto-selection

If mode is omitted, the renderer is chosen from the payload's `schema` field:

```
win net ip --json | presentation
```

| Schema | Default renderer |
|---|---|
| `network.ip` | `map` |
| `network.wifi` | `dashboard` |
| `logs.events` | `timeline` |
| `system.perf.*` | `chart` |
| `fs.directory` | `table` |
| `docker.stats` | `dashboard` |
| `k8s.pods` | `table` |

### 5.3 Fallback

If no schema is present, or the schema is unrecognized: default to `table`.

## 6. Third-Party Integration

### 6.1 Schema registration

```
presentation register --schema myapp.metrics --renderer chart
```

### 6.2 Custom renderer plugins

```
presentation register --renderer myapp-renderer.wasm
```

Renderers are sandboxed WASM modules implementing the `Renderer` trait — see [Renderer Architecture](./renderer-architecture.md#5-plugin-system).

### 6.3 Validation

```
myapp --json | presentation validate
```

Checks a payload against the registry: presence of `data`, and if `schema` is set, that fields match the registered definition.

## 7. Pipeline Contract

**Input** must be valid JSON/YAML/CSV, must contain `data`, and may contain `schema`, `source`, `timestamp`, `meta`.

**Output**, depending on mode: ANSI terminal text, TUI dashboard, ASCII diagram, Sixel/Kitty graphics, HTML/SVG, or AI-generated text.

## 8. Worked Examples

**Third-party tool** (`myapp metrics --json`):

```json
{
  "schema": "myapp.metrics",
  "data": [
    { "name": "requests", "value": 1200 },
    { "name": "errors", "value": 3 }
  ]
}
```

```
myapp metrics --json | presentation chart
myapp metrics --json | presentation        # auto-selects chart once registered
```

**Kubernetes:**

```
kubectl get pods -o json | presentation table
kubectl get pods -o json | presentation        # auto-selects table (schema = k8s.pods)
```

**Docker:**

```
docker stats --format json | presentation dashboard
docker stats --format json | presentation        # auto-selects dashboard
```

**win.exe:**

```
win net ip --json | presentation map
win logs --json | presentation timeline
win system perf --json | presentation chart cpu
```

## 9. Design Principle

> `presentation` is a renderer for any structured CLI output, not just `win.exe`.

`win.exe` leads adoption, but the protocol is meant to work identically from PowerShell, Bash, Ion, or any shell on Windows, Linux, or macOS.

## 10. Protocol Versioning

```json
"pps_version": "1.0"
```

Rules:

- Backwards compatible within a major version
- New optional fields may be added without a version bump
- Schema registry entries evolve independently (see [Schema Registry §7](./schema-registry.md#7-versioning))

## 11. Security Considerations

- No code execution from input payloads
- WASM renderers run sandboxed
- AI summary/explain modes do not execute or evaluate data as code
- HTML output is sanitized before being written or displayed

## 12. Ion Pipeline Fit

Ion's structured, typed pipeline model is a natural fit for PPS producers:

```
win system info --json
  | json query ".cpu"
  | presentation --chart
```

```
win logs --json
  | json filter level="error"
  | sort timestamp
  | presentation --timeline
```

Ion's clean stdout/stderr separation suits renderers that need unpolluted structured input, and its native plugin support means `presentation` itself could ship as an Ion-loadable WASM plugin.
