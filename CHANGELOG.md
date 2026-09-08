# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0](https://github.com/rjreeves/CLI-Presentation/releases/tag/v0.1.0) - 2026-09-08

### Added

- Presentation Protocol (PPS) envelope parsing — `data` required, `schema` optional
- Nine renderer modes: `table`, `dashboard`, `chart` (`--chart-type bar|line|pie`), `timeline`, `map`, `html`, `md`, `explain`, `ai-summary`
- Schema-based renderer auto-selection: a bare `presentation` picks a mode from the payload's `schema` field, checking a built-in table and the user schema registry
- `presentation register` and `presentation validate` subcommands, backed by a `~/.presentation/registry.json` schema registry (field definitions, type checking, nullable fields)
- Shared `--sort`, `--filter`, `--width`, and `--output` options across every mode
- `--export html|md|svg` — wraps any mode's rendered output in another format; `--export pdf` fails with a clear error rather than producing broken output
- `--live` — redraws on newline-delimited PPS envelopes read from stdin, for `dashboard`/`timeline` modes
- `explain`/`ai-summary` — deterministic, rule-based observations about the data (record counts, flagged status entries, per-field min/max/avg), with `--explain-level brief|detailed`
- 94 unit tests and 34 end-to-end CLI tests, run in CI on every push
- GitHub Actions CI: `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo build`, `cargo test`
- README, MIT license, and an animated demo GIF
- `docs/scope-decisions.md`, documenting every deliberate divergence from or narrowing of the original design docs, and why

### Fixed

- Dashboard panel titles, and the `explain`/`ai-summary`/`chart` label-detection heuristics, didn't recognize `hostname` (only the less common `host`)
- CI's Node.js 20 deprecation warning, by bumping `actions/checkout` to v7
