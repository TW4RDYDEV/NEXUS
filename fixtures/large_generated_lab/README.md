Run `npm run fixture:large` to generate 10,000 fictional hosts and 50,000 service observations. The generated XML is intentionally excluded from the source archive to avoid redundant bulk. All hostnames use the reserved `.test` namespace; no live systems are contacted.

The Rust `large_engagement` integration test constructs the same scale directly in SQLite, adds 100,000 relationships, and exercises paginated queries, coverage, search, and bounded graph output. This test runs as part of `cargo test`.
