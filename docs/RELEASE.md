# Release procedure

Create public NEXUS artifacts only from a clean source workspace and a supported Windows Rust toolchain. Release binaries are staged locally under the gitignored `release/` directory and should be attached to GitHub Releases rather than committed to source.

1. Confirm version metadata is correct and run `npm run release:source-check`.
2. Confirm the project `LICENSE` is present, then run `npm run release:check`. The gate intentionally refuses a missing license and the old MIT project template.
3. Run `npm ci`, formatting, ESLint, TypeScript, frontend tests, authorship/fingerprint tests and the production frontend build.
4. Run Rustfmt, Clippy with `--features desktop --all-targets -- -D warnings`, and all ordinary Rust tests.
5. With a trusted Nmap on PATH, run the explicit localhost-only `nmap_loopback` test. It binds a temporary test-owned 127.0.0.1 service and never scans another machine.
6. Build `nexus-bridge` and run the Playwright workflows against the real Rust/SQLite bridge.
7. Refresh npm/RustSec audits. Do not suppress warnings without documenting the dependency path and platform relevance.
8. Run `npm run release:build`. **Do not replace this with a raw `tauri build` for the public Windows artifact.** The controlled script applies Rust path remapping, stages the portable build, validates NEXUS binary markers and rejects local repository/home/Cargo/Rustup path leakage.
9. Smoke-test `release/windows-x64/NEXUS.exe` from a fresh writable folder. Confirm version 1.0.0, fictional demo, vault workflow, import/graph/assessment/snapshot writes and restart persistence.
10. If a publisher certificate is available, sign the staged EXE with `scripts/sign-windows.ps1 -CertificateThumbprint <thumbprint> -TimestampUrl <provider HTTPS timestamp URL>` and verify the Authenticode signature. Otherwise clearly identify the release as unsigned.
11. Generate `npm run authorship:report` and `npm run fingerprint:release` for private maintainer evidence. Those outputs are gitignored and must not be put in the public ZIP.
12. Regenerate public screenshots from the final 1.0.0 native build; never reuse screenshots that show an internal/pre-release version.
13. Record the exact final checks and hashes in `docs/VERIFICATION.md`, create the GitHub release archive, verify its contents, and preserve the archive SHA-256 separately.

The controlled build uses `CARGO_ENCODED_RUSTFLAGS` with `--remap-path-prefix` for the source repository, Cargo home, Rustup home and current user home. This protects release binaries from exposing local build-path details while keeping normal Rust diagnostics useful during development.

A successful automated run does not substitute for independent security review, a trusted publisher chain or field testing on every target environment.
