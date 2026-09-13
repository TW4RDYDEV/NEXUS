# Contributing to NEXUS

Thanks for helping improve NEXUS.

NEXUS is source-available software. Contributions to the official project are welcome, but public derivative distributions are not permitted outside the limited contribution-fork exception in the project license.

## Before you start

Please keep changes focused, reviewable, and aligned with the product direction:

- local-first;
- operator-focused;
- evidence-based;
- no telemetry;
- no cloud account requirement;
- no opaque AI recommendations;
- no fake or decorative security functionality;
- active execution must remain explicit, scope-validated, and auditable.

## Contribution fork exception

The [NEXUS Source-Available License 1.0](LICENSE) permits a public source fork solely for preparing and submitting good-faith contributions to the official NEXUS repository.

Such a fork must retain the project license and attribution, remain clearly unofficial, and must not publish compiled releases or market itself as an alternative NEXUS distribution.

## Development setup

```powershell
npm ci
npm run desktop
```

The frontend uses React + TypeScript. Domain rules and persisted engagement state live in the Rust core.

## Engineering expectations

Keep domain rules in Rust and keep UI state distinct from persisted assessment facts.

Add or update tests when changing:

- imports or normalization;
- references or entity resolution;
- credentials or authentication records;
- sessions, pivots, reachability, or paths;
- snapshots / ReconDelta;
- evidence or recovery;
- security boundaries;
- database schema or migrations.

Use fictional names, private/documentation IP space, and synthetic data only in fixtures and screenshots.

## Required checks

Before submitting a pull request, run the relevant checks:

```powershell
npm run format:check
npm run lint
npm run typecheck
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run fingerprint:test
npm run authorship:check
npm run build
```

For UI changes, also verify:

- 1440×920;
- 1366×768;
- keyboard focus;
- inspector/dialog overflow;
- reduced-motion behavior where relevant.

Native Tauri/config/filesystem changes should be tested in a native desktop build.

## Security boundaries

Do not submit changes that:

- execute imported content;
- render imported text as raw HTML;
- bypass scope validation;
- expand confirmed access from mere host visibility;
- interpolate untrusted values into a shell command;
- add telemetry, tracking, remote kill switches, or hidden callbacks;
- weaken vault handling or leak secrets into logs;
- silently accept ambiguous parser output as confirmed state.

## Dependencies

Do not copy code from incompatible or proprietary sources.

New dependencies should be justified and should use a license compatible with NEXUS distribution. Prefer mature permissive dependencies where practical. Document dependency changes and rerun the relevant audits.

## Contribution rights

By intentionally submitting a contribution to the official NEXUS project, you certify that you have the right to submit it and you grant the NEXUS copyright holder the rights described in Section 8 of the project [LICENSE](LICENSE), including the right to incorporate, modify, distribute, relicense, and commercialize the contribution as part of NEXUS or related products.

You retain copyright in your original contribution unless separately agreed otherwise.

## Pull requests

A strong pull request should include:

- a clear explanation of the problem;
- the proposed behavior;
- tests for meaningful logic changes;
- screenshots for visual changes;
- migration notes for schema changes;
- security implications when relevant.

Prefer small, explainable changes over large unrelated refactors.
