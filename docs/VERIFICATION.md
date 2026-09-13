# Release verification

NEXUS source baseline: **1.0.0**, engagement schema **3**.

This source package intentionally contains no stale native release binary. The feature-complete baseline was exercised before version normalization, including Rust, frontend, browser workflow, malformed-input/security-boundary and generated-scale tests. The final public 1.0.0 artifact must be rebuilt and re-verified from this exact source; old executable hashes or screenshots must not be reused as release evidence.

## Required final verification

Run on the Windows release machine from a clean checkout/extraction:

```powershell
npm ci
npm run release:source-check
npm run format:check
npm run lint
npm run typecheck
npm test
npm run fingerprint:test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --features desktop --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:e2e
npm run build
npm audit
```

With a trusted Nmap available on PATH, additionally run the explicitly ignored localhost-only test:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test nmap_loopback -- --ignored --nocapture
```

After the final NEXUS license is added:

```powershell
npm run release:check
npm run release:build
```

The controlled build must finish with the binary path-leak/product-marker gate passing. Smoke-test the staged `release/windows-x64/NEXUS.exe` from a clean portable directory, verify demo creation, vault lock/unlock, an import preview/commit, graph/inspector interaction, an assessment write, snapshot comparison and restart persistence.

## Scale/regression expectations

The included generated-scale test covers 10,000 assets, 50,000 services and 100,000 relationships using bounded graph retrieval and paged inventories. Regression tests cover schema migration, static authorship/format identity, encrypted credentials, malformed import rollback, scope blocking, evidence integrity, path validation, snapshot behavior, reporting escaping and recovery copies. These tests are evidence of the implemented behavior, not a certification for every environment.

## Release evidence to record

For the actual 1.0.0 GitHub release, record:

- Node/npm/Rust versions used;
- exact test counts and results;
- npm/RustSec audit status;
- native smoke-test result;
- whether Authenticode signing is present;
- final `NEXUS.exe` SHA-256;
- final release ZIP SHA-256;
- confirmation that the path-leak gate returned no local repository/home/Cargo/Rustup paths.

Do not publish a verification document that claims checks were executed on a binary that was not actually built and tested.
