# Contributing to wirecheck

## Current policy (v0.x): issues, not code

To keep a single copyright holder, this repository **does not accept external
code pull requests during v0.x**. What is welcome:

- bug reports and reproduction steps,
- feature requests,
- suggestions for example rule sets,
- documentation corrections (open an issue with the proposed text).

If contributions are opened in a later release, a **Copyright Transfer
Agreement or an exclusive-license CLA** will be introduced first. A DCO is not
sufficient for that goal: a DCO only certifies that a contributor has the right
to submit the work, it does not consolidate ownership.

Rule sets contributed by others must be explicitly offered under CC0 or another
clear grant stated in the PR description.

## For the maintainer

- Every source file starts with:

  ```rust
  // SPDX-License-Identifier: MIT
  // Copyright (c) 2026 h1s97x
  ```

  Use `# SPDX-License-Identifier: MIT` for TOML, YAML and shell files.

- Before opening the first external contribution path, update `LICENSE`,
  `NOTICE` and this file together.
- Commit messages follow `type(scope): summary`, e.g.
  `feat(core): add cycle detection`.
- Run the same checks as CI before releasing:

  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cargo deny check licenses bans sources
  cargo run -p wirecheck-cli -- validate --rules rules/home.rules.toml
  ```

- A release archive always includes `LICENSE` and `NOTICE`.

## Safety rules for anyone adding a probe or action

- A new action ID must be added to the compiled allow-list and implemented in
  `wirecheck-builtins`. Rule sets must never be able to name an executable or
  shell command.
- No probe may perform telemetry or upload results by default.
- New example rule sets must use `example.com` and RFC 5737 documentation
  addresses, never addresses belonging to a specific organisation.
