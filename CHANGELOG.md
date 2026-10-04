# Changelog

## 1.1.1 — 2026-10-04

Linux AppImage compatibility fix.

- Fixed AppImage startup failures on newer Mesa/EGL environments, including current Arch Linux.
- Updated the Tauri CLI packaging toolchain to 2.12.1.
- Added a shared AppImage display-stack library policy.
- Added deterministic post-build AppImage sanitization for conflicting bundled Wayland/X11 support libraries.
- Added a release gate that rejects AppImages if known-problematic display-stack libraries are still bundled.
- Pinned and SHA-256 verified the AppImage repackaging tool and runtime used by the release pipeline.
- Preserved DEB and RPM packaging behavior.
- Verified the final AppImage through GitHub Actions smoke testing and a real Arch Linux launch test.
- No changes to engagement data, vault encryption, scope enforcement, IPC permissions, or the core security model.

## 1.1.0 — 2026-10-03

Linux support release.

- Added official Linux x86_64 support.
- Added AppImage packaging for portable Linux use, including Arch Linux.
- Added native `.deb` packages for Debian and Ubuntu.
- Added native `.rpm` packages for Fedora and RHEL-compatible distributions.
- Added platform-specific Tauri configuration for Windows and Linux.
- Added a dedicated Linux release builder with deterministic artifact staging and SHA-256 checksums.
- Added GitHub Actions builds on Ubuntu 22.04.
- Added automated AppImage launch smoke testing before Linux artifacts are accepted.
- Preserved the existing Windows x64 NSIS and portable release workflow.
- No changes to engagement data, vault encryption, scope enforcement, IPC permissions, or the core security model.

## 1.0.0 — 2026-09-12

Initial public NEXUS release baseline.

- Local-first engagement workspaces with explicit include/exclude scope rules and validated active-runner boundaries.
- Normalized assets, services, identities, provenance, Cytoscape graph, inspector, search and confirmed-access paths.
- Argon2id + XChaCha20-Poly1305 credential vault, credential/service access matrix, sessions, pivots and recursive reachability.
- Reviewed Nmap, httpx, Nuclei, NetExec, Nessus v2 and Burp XML imports with raw-source retention, conflict review, deduplication and transactional commit.
- Assessment planning with 52 original checks across ten domains, custom checks, owners, deadlines, outcomes, evidence and finding links.
- Coverage and deterministic explainable opportunities derived only from recorded state.
- Findings, evidence integrity verification, Markdown/client HTML reporting, timeline, snapshots and ReconDelta comparisons.
- Complete workspace bundle export/verification/recovery, automatic backups, schema migrations and pre-upgrade recovery copies.
- Static NEXUS authorship/file-format identity, deterministic local fingerprint tooling and release fingerprint support.
- Controlled Windows release build path that remaps local Rust source/Cargo paths and rejects leaked local build paths in the staged executable.
- Browser, frontend, Rust, security-boundary, workflow, parser and scale tests; dependency-audit evidence is documented separately.

- First public source-available release under the NEXUS Source-Available License 1.0.
