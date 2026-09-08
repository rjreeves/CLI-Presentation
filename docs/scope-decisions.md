# Scope Decisions and Open Questions

**Status:** Living document — reflects the implementation as of the current build, not the original design docs.

This tracks where the `presentation` implementation deliberately diverges from, narrows, or takes a stance on something the original four design docs ([presentation-command.md](./presentation-command.md), [presentation-protocol.md](./presentation-protocol.md), [schema-registry.md](./schema-registry.md), [renderer-architecture.md](./renderer-architecture.md)) left open, aspirational, or ambiguous. Each entry says what the docs describe, what was actually built, and why. It exists so a decision made for a good reason during implementation isn't mistaken later for an oversight.

## Scope decisions

### `explain` / `ai-summary` are rule-based, not an LLM call

renderer-architecture.md §3.3/§4 names Azure OpenAI as the intended backend. presentation-command.md §11 lists "whether explain/ai-summary calls a local model or a hosted API by default, and how that's configured" as an open question — the docs never resolve it.

**Built instead:** both modes are deterministic generators over the data's own statistics (record count, flagged status entries, per-field min/max/avg via `render::insights`), phrased in plain English. No network call, no API key, no external dependency.

**Why:** wiring a real hosted call would mean inventing a config/credentials story the docs don't specify, and would make the build silently network-dependent on an unconfigured external service. The rule-based version is real, testable, useful today, and doesn't pretend to be an LLM.

### `chart`/dashboard gauges are ASCII-only — no Sixel/Kitty graphics

renderer-architecture.md §7 describes "adaptive rendering": render a real chart via Sixel/Kitty when the terminal supports it, fall back to ASCII otherwise.

**Built instead:** ASCII/Unicode block characters only (bars, sparklines, gauge bars), unconditionally.

**Why:** terminal-capability detection and a graphics-protocol renderer are a separate, sizable feature; ASCII rendering already works everywhere without it.

### `map` is a fixed three-level topology, not a general graph renderer

presentation-command.md §3 lists two use cases for `map`: "Network interfaces, dependency graphs." Only the network-interface shape (`network.ip`) is implemented — root → interface → gateway → DNS leaves.

**Why:** a general dependency-graph renderer needs an actual node/edge data model; nothing in the current schema registry describes dependency-graph-shaped data, so building it now would be speculative.

### `html`/`md` modes skip embedded diagrams and AI recommendations

presentation-command.md §9's fuller worked example for `win net ip` describes `html`/`md` output with "embedded topology diagram, summary and recommendations sections."

**Built instead:** a summary line (record count + flagged count) and a data table. No embedded diagram, no recommendations.

**Why:** the diagram needs Graphviz (renderer-architecture.md §4) and recommendations need the AI module (§3.3) — neither exists in this codebase, and neither is a small addition once `explain`'s no-LLM decision (above) stands.

### Schema registry: user-file location only

schema-registry.md §8 describes three merged locations (`/etc/...` system-wide, `~/.presentation/...` user, `<app>/presentation-schema.json` application-specific) merged application > user > system, plus a database-backed option (§12).

**Built instead:** only the user-level file (`~/.presentation/registry.json`, `%USERPROFILE%` on Windows), consulted before the built-in table (so effectively user > built-in-as-"system"). No `/etc` path, no app-local file, no database backend.

**Why:** `/etc` isn't a meaningful concept on Windows (this project's primary dev environment); an app-local file has no clear "current app" concept for a renderer invoked via a shell pipe; the database option is explicitly marked in schema-registry.md §12 as having unspecified DDL/migration/indexing, i.e. not ready to implement against.

### Built-in field definitions exist for 5 of 7 documented schemas

schema-registry.md §3's canonical JSON sample gives full field definitions for `network.ip`, `network.wifi`, `logs.events`, `system.perf.cpu`, and `fs.directory`. `docker.stats` and `k8s.pods` appear only in the renderer-selection table (§6), never with field definitions.

**Built:** `presentation validate` has field-level checks for the five documented schemas; for `docker.stats`/`k8s.pods` it confirms the schema is recognized (and its renderer) but skips field validation, since there's nothing in the docs to validate against.

### No WASM renderer plugins

presentation-protocol.md §6.2 and renderer-architecture.md §5 describe third-party renderers as sandboxed WASM modules registered via `presentation register --renderer myapp-renderer.wasm`.

**Not built.** `presentation register` only registers *schemas* (schema-registry.md §4/§6.1), never custom renderer plugins.

**Why:** this needs an embedded WASM runtime, a `Renderer` trait ABI across the WASM boundary, and a sandboxing story — a substantial subsystem, not an incremental addition to the current renderer set.

### `--export pdf` errors instead of producing a file

presentation-command.md §4 lists `pdf` as an `--export` target alongside `html`/`md`/`svg`.

**Built:** `html`/`md`/`svg` export wrap any mode's rendered text in that container format. `--export pdf` returns a clear error ("PDF generation needs a dedicated dependency this build doesn't include") rather than writing something broken with a `.pdf` name.

**Why:** real PDF generation needs either a dedicated crate (a real addition to the dependency tree — fonts, layout, binary structure) or hand-rolling the PDF byte format by hand. Either is a bigger, more consequential call than this scaffold should make without checking first.

### `--live` commits to one interpretation of an explicitly open protocol

presentation-command.md §11 lists "Exact live-refresh protocol for `--live` dashboards (poll vs. push from producer)" as an open question.

**Built:** push-from-producer over newline-delimited PPS envelopes on stdin — each line is a complete envelope, redrawn (clearing the screen when stdout is a real terminal) as it arrives. Mode is resolved once from the first line and held fixed for the session. Restricted to `dashboard`/`timeline`, per the docs' own scoping ("Enable live refresh for dashboard/timeline modes") — any other mode warns and renders the first frame once.

**Why this and not polling:** polling implies a re-invokable data source, but this tool's whole design (every other mode included) is "read once from stdin, produced by an upstream pipe." NDJSON-over-stdin is the interpretation consistent with that existing architecture, and it's directly implementable without inventing a source-command/interval config the docs never describe.

## Open questions the docs left unresolved that are still unresolved

These aren't scope calls made during implementation — they're open questions the source docs name explicitly, and nothing above resolves them fully. Revisit if the project's direction changes.

- **AI backend for `explain`/`ai-summary`** (presentation-command.md §11): local model vs. hosted API, and its configuration story. Currently sidestepped entirely by not calling an LLM (see above) — if real LLM-backed output is ever wanted, this decision still has to be made.
- **SVG/PDF export fidelity** (presentation-command.md §11): "SVG/PDF export fidelity requirements for `--export`" — SVG is implemented as plain monospace text-as-image (functional, not polished); PDF is unimplemented. Fidelity expectations for either were never specified.
- **Database-backed registry DDL/migration/indexing** (schema-registry.md §12, "Open item"): explicitly called out in the source doc as unspecified. Not attempted here.

## Not implemented at all

For completeness — things named in the docs with no corresponding code, beyond what's covered above:

- Third-party renderer plugin registration (`presentation register --renderer *.wasm`)
- System-wide and application-specific registry file locations
- `--changes` (diff mode) and `--compact` from the `win net ip` worked example (presentation-command.md §9) — these describe `win.exe`-specific behavior for a producer this project doesn't include, not a `presentation` renderer feature
