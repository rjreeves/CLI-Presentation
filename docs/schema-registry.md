# Schema Registry Document (SRD)

**Version:** 1.0
**Status:** Draft
**Component:** Presentation Protocol (PPS) schema registry

## 1. Overview

The Schema Registry is the central catalog of known data schemas used by [`presentation`](./presentation-command.md) to validate incoming data, auto-select a renderer, supply visualization metadata, and let third-party tools register new schemas. It is open, extensible, and cross-application — see the [Presentation Protocol](./presentation-protocol.md) for how producers reference these schemas.

## 2. Schema Naming Convention

```
<domain>.<category>[.<subtype>]
```

Examples: `network.ip`, `network.wifi`, `system.perf.cpu`, `system.process`, `logs.events`, `fs.directory`, `docker.stats`, `k8s.pods`, `myapp.metrics`.

## 3. Registry Structure

Stored as a structured document (JSON canonical form shown; YAML/TOML equivalent acceptable for hand-authored registries).

```json
{
  "registry_version": "1.0",
  "schemas": {
    "network.ip": {
      "version": "1.0",
      "description": "IP configuration for network interfaces",
      "default_renderer": "map",
      "fields": {
        "interface": "string",
        "ipv4": "string|null",
        "ipv6": "string|null",
        "gateway": "string|null",
        "dns": "array<string>",
        "status": "string",
        "speed": "string|null"
      }
    },

    "network.wifi": {
      "version": "1.0",
      "description": "Wi-Fi interface and signal metrics",
      "default_renderer": "dashboard",
      "fields": {
        "ssid": "string",
        "signal": "number",
        "band": "string",
        "channel": "number",
        "ipv4": "string|null",
        "gateway": "string|null"
      }
    },

    "logs.events": {
      "version": "1.0",
      "description": "Chronological system or application events",
      "default_renderer": "timeline",
      "fields": {
        "timestamp": "string",
        "level": "string",
        "message": "string",
        "source": "string"
      }
    },

    "system.perf.cpu": {
      "version": "1.0",
      "description": "CPU performance metrics",
      "default_renderer": "chart",
      "fields": {
        "timestamp": "string",
        "usage": "number",
        "temperature": "number|null"
      }
    },

    "fs.directory": {
      "version": "1.0",
      "description": "Directory listing",
      "default_renderer": "table",
      "fields": {
        "name": "string",
        "size": "number",
        "type": "string",
        "modified": "string"
      }
    }
  }
}
```

This is the canonical format for both the built-in registry and any registry a code-generation tool produces or extends.

## 4. Schema Registration

Third-party tools register schemas dynamically:

```
presentation register --schema <name> --renderer <mode> --fields <file>
```

```
presentation register --schema myapp.metrics --renderer chart --fields metrics.json
```

The registry then stores: schema name, version, default renderer, field definitions, and any optional metadata supplied.

## 5. Schema Validation

Rules applied by `presentation validate` (and implicitly on every render):

- `data` must exist
- If `schema` is set, it must be either a built-in schema or a registered one
- Fields must match the registry's field definitions
- Unknown fields are allowed but ignored
- Missing required fields produce warnings, not hard failures
- Renderer auto-selection uses the schema's `default_renderer`

```
myapp --json | presentation validate
```

## 6. Renderer Auto-Selection

If the user does not specify a mode, the registry resolves it from `schema`:

| Schema | Default renderer |
|---|---|
| `network.ip` | `map` |
| `network.wifi` | `dashboard` |
| `logs.events` | `timeline` |
| `system.perf.*` | `chart` |
| `fs.directory` | `table` |
| `docker.stats` | `dashboard` |
| `k8s.pods` | `table` |

Fallback: `table`. Full selection logic lives in [Presentation Protocol §5](./presentation-protocol.md#5-renderer-selection-logic).

## 7. Versioning

Each schema entry carries its own `version`; the registry file as a whole carries `registry_version`.

- Backwards compatible within a version
- New fields may be added freely
- Fields may not be removed without a major version bump
- A schema's `default_renderer` may change only in a minor version bump (not silently)

## 8. Registry Storage Locations

```
/etc/presentation/registry.json        # system-wide
~/.presentation/registry.json          # user-specific
myapp/presentation-schema.json         # application-specific
```

Merge precedence, highest first: **application > user > system**.

## 9. Schema Discovery at Render Time

Given a payload:

```json
{ "schema": "network.ip", "data": [...] }
```

the renderer: confirms the schema exists in the merged registry → loads its renderer metadata → normalizes and passes the data to that renderer. If the schema is missing, it warns and falls back to `table`.

## 10. Worked Example — Third-Party Schema

`myapp.metrics.json`:

```json
{
  "version": "1.0",
  "description": "Application performance metrics",
  "default_renderer": "chart",
  "fields": {
    "name": "string",
    "value": "number",
    "timestamp": "string"
  }
}
```

```
presentation register --schema myapp.metrics --renderer chart --fields myapp.metrics.json
```

## 11. Error Codes

| Code | Meaning |
|---|---|
| `SRD_SCHEMA_NOT_FOUND` | Schema missing from the merged registry |
| `SRD_SCHEMA_INVALID` | Schema file malformed |
| `SRD_FIELD_MISMATCH` | Data fields do not match the schema's field definitions |
| `SRD_VERSION_CONFLICT` | Payload/schema version incompatible with registry entry |
| `SRD_REGISTRY_CORRUPT` | Registry file unreadable |

## 12. Appendix — Database-Backed Registry

The registry above is a flat-file design (§8). For centralized, multi-machine, or multi-team use, it can instead live in a relational database and be loaded at startup via a connection URL — this is a deployment option, not a change to the schema format in §3.

**Table shape (Postgres example):**

```sql
CREATE TABLE pps_schema_registry (
    schema_name TEXT PRIMARY KEY,
    version TEXT,
    default_renderer TEXT,
    fields JSONB
);
```

**Config:**

```
schema_registry_url = "postgres://user@host:5432/presentation?currentSchema=registry"
```

**Startup sequence:** connect using the URL → query `pps_schema_registry` → load rows into the in-memory registry → merge with local registry files per the precedence rule in §8 (treat the database as an additional, lowest- or highest-precedence source — decide explicitly per deployment).

Schema-selection syntax is engine-specific (this is unrelated to PPS schema naming in §2 — it selects a *database* namespace, not a PPS schema):

| Engine | Schema-in-URL support | Method |
|---|---|---|
| PostgreSQL | Yes | `?currentSchema=analytics` or `?search_path=public` |
| MySQL / MariaDB | Yes (schema == database) | database name in the path |
| SQL Server | Yes | `?database=master&mode=schema` |
| Oracle | Yes (implicit) | schema == connecting user/service |
| CockroachDB | Yes (Postgres-compatible) | `?search_path=public` |
| SQLite | No | no schema concept |

**Benefits over flat files:** centralized management across machines, versioning via DB migrations, multi-tool interoperability, remote registry access without file sync.

**Open item:** DDL, migration strategy, and indexing for `pps_schema_registry` are not yet specified — needed before this becomes the default rather than an option.
