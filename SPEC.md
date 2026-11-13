# wirecheck — Design Specification

- Status: M3 (full probe set, starter template, rule-set trust), 2026-11-14;
  M2 (SARIF, dry-run plans, audited apply/rollback), M1 (probes, concurrent
  runner, JSON/HTML reports) and M0 (schema + static validation) dated
  2026-11-13/14
- License: MIT, single copyright holder
- Document purpose: normative description of the design, and a dated record of
  independent authorship.

## 1. Purpose and scope

wirecheck turns a network health check into a versioned, shareable, CI-friendly
configuration instead of an application with hard-coded endpoints.

A rule set declares **what to observe**, **what counts as a failure**, and
**which remediation to suggest**. A runner executes the observations and emits a
structured report containing evidence that can be handed to another person
without leaking local network details.

### In scope

- A declarative rule-set format (`*.rules.toml`).
- A library (`wirecheck-core`) that third-party Rust programs can embed.
- A CLI (`wirecheck`) for humans and for CI.
- Concurrent, dependency-ordered probe execution.
- Structured JSON, SARIF 2.1.0 and single-file HTML reports.
- A closed, allow-listed set of remediation actions with plan / apply /
  rollback semantics.

### Explicitly out of scope

- GUI, tray application, background daemon, continuous polling.
- Built-in addresses belonging to any particular organisation.
- Dynamic plugins, WASM, or user-supplied executables.
- Executing arbitrary shell commands described in a rule set.
- Mutating the host's network configuration from a rule set (no platform
  adapter ships), privilege escalation, and publisher-identity signatures.
  See §15 for the reconciliation of the M2/M3 acceptance wording.
- Telemetry or automatic upload of results. A `Sink` trait is reserved for a
  possible future opt-in service.

## 2. Vocabulary and data contract

| Term | Meaning |
| --- | --- |
| `RuleSet` | Parsed configuration: defaults, redaction policy, variables, probes, remediations, report policy. |
| `Probe` | One observation unit, identified by a stable user-defined `id`. |
| `Observation` | Raw outcome of a probe, before an assertion is applied. |
| `Expect` | Assertion evaluated against an observation to produce an `Outcome`. |
| `Finding` | One row of the report: probe id, outcome, human message, structured evidence, optional remediation reference, method used, elapsed time. |
| `Outcome` | `pass` / `fail` / `warn` / `skipped` / `error`. |
| `Evidence` | Structured key/value data, kept separate from the human-facing message so it can be machine-read and redacted. |
| `Remediation` | A registered, closed action referenced by ID. Discovery is by ID, never by code in the rule set. |
| `Plan` | A description of the changes an action would make, produced without performing them. |
| `Sink` | Destination for a finished `Report`; reserved extension point. |
| `RuleError` / `Diagnostic` | Parse errors / static validation findings. |

Design rule: **human text and machine data never share a field**. This is why a
`Finding` has both `message` and `evidence`, and why remediation is an ID rather
than an inline callable.

## 3. Rule-set format

TOML is the primary format. `serde_yaml` is deliberately not used (archived).
YAML may be added later as an optional front end that parses into the same
types.

Top level: `version` (must equal `SCHEMA_VERSION`), `name`, optional
`description`, `[defaults]`, `[redact]`, `[vars]`, `[reports]`, `[[probes]]`,
`[[remediations]]`.

- `[defaults]`: `timeout` (duration string, units `ms` / `s` / `m` / `h`),
  `retries`, `concurrency`.
- `[redact]`: `ip`, `user`, `mac`, each `none` / `mask` / `hash`.
- `[vars]`: a flat string map. Probes may reference `{{ vars.NAME }}`. Variables
  are substituted from this table only; there is no expression evaluation and
  no command execution.
- `[[probes]]`: `id`, `type` (one of the built-in probe kinds), `depends_on`,
  a `with` table of probe parameters, and an `expect` table of assertions.
- `[[remediations]]`: `for` (probe id), `id`, `action` (must be a known action
  ID), `params`, `risk` (`low` / `medium` / `high`), `needs_confirm`, and an
  optional `rollback` action.

`validate` performs static checks without touching the network: schema version,
identifier uniqueness, known probe kinds, known action IDs, dependency
existence, self-dependency, dependency cycles, undefined variable references,
report formats, and unsafe remediation defaults. All diagnostics are reported
in one pass rather than stopping at the first error.

## 4. Execution model

1. Parse and statically validate.
2. Topologically order probes by `depends_on`; execute each level with bounded
   concurrency.
3. Apply per-probe timeout and retry policy. A probe that fails is `error`; its
   dependents become `skipped`; the run continues.
4. Evaluate `expect` against the observation to produce `outcome`.
5. Collect findings into a `Report`.

Exit codes: `0` all clear, `1` one or more `fail`/`error`, `2` invalid rule set
or runtime configuration error. `validate` uses `0` / `2`.

Cross-platform ICMP has a three-step degradation chain: native raw socket,
system `ping` subprocess, then a TCP-connect reachability approximation. The
`Finding.method` field records which step was used; degradation is never silent.

## 5. Safety model

A rule set is **data** and may be downloaded from an untrusted source. Therefore:

- Remediation actions form a closed enum compiled into `wirecheck-builtins`.
  The rule set can only name an action and supply parameters; it cannot
  introduce new behaviour.
- `--dry-run` / plan generation is the default. `--apply` is explicit;
  `needs_confirm = true` requires an interactive TTY.
- Actions that need privileges are declared `requires_elevation`; elevation is
  handled centrally by the CLI.
- Every apply writes an audit record including the plan, result and rollback
  result.
- M0 implements **no apply path at all** — only validation and planning.

## 6. Reports and redaction

- JSON: stable, carries `schema_version`, consumed by programs and CI.
- SARIF 2.1.0: integrates with code-scanning surfaces.
- HTML: single self-contained file with inline CSS, no external resources,
  suitable for attaching to a support request.

Redaction is applied in the report layer, never in the core runner, so local
debugging keeps full values while exported reports do not. Defaults are
`mask` for IP/MAC and `hash` (SHA-256) for user identifiers.

## 7. Compatibility

`SCHEMA_VERSION` and `REPORT_SCHEMA_VERSION` are independent. Rule-set schema
changes bump `SCHEMA_VERSION` and come with a migration note. Report consumers
may rely on `schema_version` being stable within a major release.

## 8. Milestones and acceptance

| Milestone | Deliverable | Acceptance |
| --- | --- | --- |
| M0 | Governance files, schema types, static validator, CLI skeleton, CI | `cargo test` green; `wirecheck validate` reports every invalid action ID in one pass; `cargo deny check` passes. |
| M1 / v0.1 | dns / http / icmp probes, concurrent runner, JSON + HTML reports, redaction, exit codes | Three-platform CI green; a shareable single-file HTML report is produced; still read-only. |
| M2 / v0.2 | dependency ordering, variable substitution, SARIF, plan, closed action registry | Dry-run plan, complete audit records and rollback orchestration with an injectable executor; see §15. |
| M3 / v0.3 | full probe set, starter template, local rule-set trust, elevation guard | `wirecheck init` produces a rule set that passes `validate`; see §15. |
| v1.0 | Frozen report schema, semver commitment, crates.io release | usable as a library dependency. |

## 9. Record of independent authorship

This project is a new, independent design. Its interfaces, vocabulary,
directory layout, wording, error codes and example data were defined here from
first principles. No source code, comments, UI copy, string literals,
configuration files, assets or directory structure were copied or translated
from any existing network-diagnosis application. Example addresses use
`example.com` and the RFC 5737 documentation range `192.0.2.0/24`.

Dated design decisions:

| Date | Decision | Reason |
| --- | --- | --- |
| 2026-11-13 | Async `Probe` returning an `Observation`, with assertions held in `expect`. | Keeps observation and judgement separate; allows the same probe to be reused with different thresholds. |
| 2026-11-13 | `Finding` carries structured `Evidence` plus a separate `message`. | Machine-readable reports, reliable redaction, CI integration. |
| 2026-11-13 | Remediations are IDs into a closed registry, with `plan` / `apply` / `rollback`. | Prevents untrusted rule sets from executing arbitrary code; makes dry-run and rollback first-class. |
| 2026-11-13 | TOML as primary format; `serde_yaml` excluded. | The latter crate is archived and unmaintained. |
| 2026-11-13 | M0 ships no `apply` and no network access. | Removes the highest-risk surface from the first release. |

## 10. Known M0 simplifications

- `expect` is an untyped key/value table; per-probe assertion typing arrives in M1.
- `Report.generated_at` is a plain string; a real clock is wired in M1.
- `RedactPolicy` classification of evidence fields uses field-name hints; M1
  replaces it with explicit sensitivity tags in the schema.
- `ProbeDescriptor.implemented` is `false` for every probe kind in M0; the
  registry is metadata only.

## 11. M1 implementation notes (2026-11-14)

- Implemented probe kinds: `dns`, `http`, `icmp`, `tcp`. The other four remain
  planned and report an explicit error when run.
- Concurrency uses standard-library scoped threads with bounded fan-out per
  dependency level. `ProbeExecutor` is synchronous rather than async, to avoid
  adding a runtime dependency; observation and judgement remain separate.
- HTTP is plaintext only. An `https` URL is rejected with an explicit error
  until a TLS stack is added.
- ICMP starts at the system `ping` fallback and degrades to a TCP-connect
  estimate. Native raw sockets need platform-specific code. The step actually
  used is always recorded in `Finding.method`; degradation is never silent.
- `Report.generated_at` is now a real UTC RFC3339 clock.
- Variable substitution was brought forward from M2. There is still no
  expression evaluation and no command execution.
- SARIF, plan/apply/rollback and rule-set signing remain M2/M3. M1 is still
  read-only: no `apply` path and no remediation execution.

## 12. M2 implementation notes (2026-11-14)

- SARIF 2.1.0 is rendered in the report layer with the same redaction policy
  as JSON and HTML.
- `run` builds a dry-run plan from unhealthy findings that reference a
  registered remediation. `--apply` requires `--yes`; a remediation with
  `needs_confirm = true` additionally requires an interactive TTY.
- `apply_plan` writes a complete `AuditRecord`: the plan, every result and
  every rollback result. Any non-applied action triggers rollback of all
  actions that did succeed.
- No host-mutating adapter ships in M2. `DisabledExecutor` refuses every
  action and records the refusal; `RecordingExecutor` exercises the full
  apply/rollback/audit path in tests. Supplying a platform-specific
  `ActionExecutor` is the remaining step before real remediation.
- Elevation, rule-set signing / `--trust` and the full probe set remain M3.

## 13. M3 implementation notes (2026-11-14)

- All eight registered probe kinds are implemented. `proxy` reads only
  environment variables and never records values (they may contain
  credentials). `interface` uses a connected UDP socket to discover the local
  address without sending a packet. `traceroute` delegates to the system
  `traceroute` / `tracert` and reports an explicit error when unavailable.
  `captive_portal` reuses the plaintext HTTP probe and reports redirect or
  login-page heuristics; it never fabricates a detection when the request
  fails.
- `wirecheck init` writes a commented starter rule set that passes
  `validate`; this is the "configure within ten minutes" acceptance path.
- `wirecheck trust` registers a rule set's SHA-256 fingerprint in
  `.wirecheck/trusted.txt` (gitignored); `run --trust` refuses an unknown or
  modified rule set. This is a local allow-list, **not** asymmetric signing:
  publisher identity verification still needs a key-management design.
- Elevation is handled centrally in the CLI: a planned action marked
  `requires_elevation` is refused with an explicit error rather than silently
  attempting a privileged system change. `DisabledExecutor` remains the
  default, so M3 still ships no host-mutating adapter.
- Remaining v1.0 work: frozen report schema, semver commitment, crates.io
  release and a real signing scheme if publisher verification is required.

## 14. Report UI note (2026-11-14)

- The HTML report was upgraded from a static table to a single-file
  interactive dashboard: summary cards, SVG pass-rate ring, outcome filters,
  free-text search, expandable evidence, dark-mode support and accessible
  labels.
- It remains a **report artifact**, not a product front end: inline CSS/JS
  only, no external resources, no network requests, evidence redacted at
  render time. §1 still excludes a GUI / tray application.
- If a real front end is required, that is a scope change: either a separate
  web application consuming `report.json`, or a desktop UI. Both need a new
  design decision before implementation.

## 15. Scope reconciliation (2026-11-14)

The M2/M3 rows in §8 originally described full remediation execution,
privilege elevation and publisher signing. The shipped implementation does
**not** do those things, and that is now the accepted scope, not a pending
bug:

- `--apply --yes` produces a complete `AuditRecord` (the plan, every result
  and every rollback result). The default `DisabledExecutor` refuses every
  host change; `RecordingExecutor` verifies apply/rollback in tests. A real
  platform adapter is out of scope for v0.x.
- Elevation is a central guard that refuses privileged actions with a clear
  error. No privilege escalation is performed.
- `--trust` is a local SHA-256 allow-list of reviewed rule-set fingerprints.
  It detects modification, not publisher identity; no asymmetric signing or
  key management ships.
- §1 is amended accordingly. v1.0 therefore does **not** promise real
  remediation. Changing this scope requires a dated decision here first.

## 16. v1.0 release preparation (2026-11-14)

- Report schema: `SCHEMA_VERSION` and `REPORT_SCHEMA_VERSION` are independent.
  Within a major release, `Report.schema_version` and all `Finding` field
  names and types are stable. Adding a field is a minor change; renaming,
  removing or retyping one bumps `REPORT_SCHEMA_VERSION` and requires a
  migration note in `CHANGELOG.md`. Rule-set schema changes bump
  `SCHEMA_VERSION` and require a migration note too.
- Semver commitment: from v1.0, the public API is the set of `pub` items in
  `wirecheck-core` (`model`, `rules`, `observation`, `runner`, `plan`,
  `validate`), `wirecheck-builtins` (registry and `ActionExecutor`
  implementations), `wirecheck-report` (`render` and redaction helpers), and
  the CLI's documented subcommands and exit codes. Breaking changes require a
  major version bump.
- Library usability is verified by
  `cargo run -p wirecheck-core --example embed`, which parses and validates a
  rule set without the CLI or any network access.
- Release artifacts: `CHANGELOG.md`, package metadata (`repository`,
  `readme`, `keywords`, `categories`), `cargo package --list` and
  `cargo publish --dry-run`. Publishing itself, version tags and crates.io
  credentials are maintainer actions at release time.
- The declared MSRV was corrected from 1.75 to **1.85** after inspecting the
  dependency chain (`clap` / `indexmap` / `hashbrown` declare 1.85). It has
  not yet been compiled on a 1.85 toolchain (this machine runs 1.100);
  re-check with `cargo +1.85.0 test --workspace` before tagging v1.0.
- The `1.0.0`–`1.0.5` tags belong to the legacy C# project, not this
  codebase. They must not be read as wirecheck releases; decide whether to
  delete or rename them before publishing.
