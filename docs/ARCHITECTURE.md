# Architecture and extension points

## Ownership

`src/app/App.tsx` manages the selected engagement, navigation, refreshes, inspector history, and shared mutation/error feedback. Feature directories contain domain views. `src/lib/schema.ts` provides editor fields, with backend validation remaining authoritative. Cytoscape is loaded lazily to keep ordinary table/overview startup smaller.

`src-tauri/src/core/mod.rs` dispatches explicit operations into a workspace `Store`. `commands/` exposes that core to Tauri. `bin/bridge.rs` exposes the same core over request-ID-tagged stdio for development and browser tests. It has no second data model or browser persistence substitute.

`db/` handles parameterized queries, normalized writes, reference validation, transactions, manual-edit undo, backups, settings, and provenance. `migrations/001_nx_core_engagement.sql` defines the baseline schema; `002_nx_format_identity.sql` adds schema 2 format metadata, migration history and snapshot envelopes. `003_nx_assessment_operations.sql` adds the assessment plan, query indexes and a confirmed-access view. `db/format.rs` performs transactional upgrades and writes workspace metadata. Credential ciphertext is never returned by generic inventory/search APIs. Internal store methods can access it only for vault operations.

`identity.rs` centralizes product, format, schema and build metadata. `domain/` owns typed persisted events and graph relationships, with separate human labels. `errors.rs` defines stable support codes and the structured IPC error envelope. `models/provenance.rs` provides typed observation sources, confidence and normalization records. Public architectural markers and local maintainer scripts are documented in [AUTHORSHIP](AUTHORSHIP.md); the exact database format is in [database-format](architecture/database-format.md).

## Model

The database separates engagement metadata, scope rules, assets and aliases, services, credentials, authentication tests, sessions, pivots, findings, evidence, relationships, imports, field provenance, activity events, methodology definitions/results, snapshots, settings, backups, and undo history. Services are identified by asset/protocol/port. Aliases can correlate additional observed addresses or hostnames with an analyst-confirmed asset. Ambiguous identities produce an import error instead of arbitrary merges.

`services/imports.rs` normalizes parser discoveries. Import previews list additions, observations, warnings, and nonempty field conflicts. Commit writes retained source plus a transactional database update; source hashes and import IDs connect provenance to observations. Empty incoming values cannot erase an existing nonempty value. Failed database commits remove the retained raw file.

## Derived intelligence

`core/intelligence.rs` derives the graph, recursive reachability, directed paths, methodology coverage, opportunities, and snapshot deltas. These views do not claim active checks happened without observations.

- The latest authentication observation controls whether a credential edge is confirmed valid.
- Active sessions establish known operator access. An active pivot extends reachability only when its source session is active and its host is already reachable; iteration supports multiple hops.
- Directed path search uses confirmed traversable edges. Passive network connectivity is not a usable authentication path. A manually pinned sequence is revalidated against those same edges.
- Coverage combines applicable methodology definitions with recorded check states. Recognized imported observations mark only their corresponding checks.
- Opportunities explain missing tests, unknown privileges, and evidence gaps using current rows. They never run commands.
- Snapshots omit secret ciphertext and volatile timestamps. Diffs compare meaningful entity fields and include added, removed, and changed records; removed graph entities can be shown as historical ghosts.

## Rendering and scale

Inventory, references, credential matrices/history, pivots and assessment plans page directly in SQLite. Graph exploration selects 60 assets or a bounded focus neighborhood before loading associated records (600 per collection), then caps the rendered result at 500 nodes and 4,000 edges. Exact focused entities and pinned paths are retained. The graph does not first materialize the full engagement. Inspector previews remain bounded; full inventory and reference search remain available.

Confirmed-access paths use reverse breadth-first traversal of an indexed SQLite edge view, with a 50,000-entity exploration limit and 100-step output limit. Manual descriptive edges and routes alone do not grant access. IPv4 reachability parses networks once and expands active pivots over sorted address ranges. It requires a live session on the pivot's source host.

The small 100-record frontend cache supplies cosmetic overview data only. Database-backed reference controls, matrix queries and labels are independent of it. Inventory service/evidence counts are computed for visible rows. Opportunities are bounded per category and do not compute every credential/service pair in memory. Full snapshots, reports and workspace bundles necessarily scale with their content. Scale tests measure queries, not simultaneous rendering of every edge.

core/assessment.rs supplies local original methodology templates and validated outcome tracking. core/delivery.rs produces escaped, offline client HTML reports and complete active workspace bundles with manifests. Bundle recovery checks bytes again after copying, restores to a new directory, and retains encrypted vault data unchanged.

## Adding a parser

Implement the adapter trait in `parsers/`, map observations to `models::DiscoveryRecord`, register the adapter name, add realistic successful and malformed fixtures, and test transactional merge/provenance behavior. Adapters must never execute imported content, assume authentication from connectivity, or promote a scan finding directly to analyst-confirmed status.

For schema changes, introduce an ordered migration and increase the version gate. Do not silently reinterpret old vault ciphertext or modify authenticated entity IDs. Keep test fixtures fictional.

## Workspace lifecycle

Each open workspace owns one SQLite connection and an in-memory vault. The Tauri command mutex serializes mutations. A child Nmap job belongs to the open workspace and cannot cross an engagement switch. The UI polls status while a scan runs and refreshes summary/backups every 30 seconds. Recent-workspace registry replacement uses a temporary file and rename. Recovery opens and validates a copied database before making it active.
