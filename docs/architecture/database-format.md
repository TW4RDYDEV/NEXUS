# NEXUS engagement format

NEXUS — TWARDY.exe / TW4RDYDEV

The SQLite `application_id` is permanently assigned within this project as **0x4E585553**, decimal **1314411859** (ASCII `NXUS`). It is a positive signed 32-bit integer, stored in the SQLite header at bytes 68–71 in big-endian order. This is a local project format identifier; no global registration is claimed.

The canonical values live in `src-tauri/src/identity.rs`:

| Field | Value |
| --- | --- |
| product | NEXUS |
| product_id | nexus.tw4rdy.core |
| format | nexus-engagement |
| format_version | 1 |
| schema_family | nx-engagement-graph |
| schema_version / PRAGMA user_version | 3 |

Each engagement has `.nexus/nexus.db` and `.nexus/workspace.json`. The JSON and `nx_format_metadata` table identify the format; they contain no installation, user or device ID. SQL metadata values are text; JSON version values are numbers. Snapshot records contain the same format envelope in `snapshots.format_metadata`. Graph and engagement summary responses include `format_metadata`.

## Migration history

`nx_migrations(version, name, recorded_at)` records these useful schema transitions:

1. `001_nx_core_engagement`: original normalized engagement schema, access inventory, provenance, coverage and ReconDelta tables.
2. `002_nx_format_identity`: format metadata, migration registry, snapshot envelopes and canonical relationship/event identifiers.
3. `003_nx_assessment_operations`: assessment plans, supporting indexes and the confirmed-access edge view.

For an existing schema 1 or 2 engagement, opening it first creates a local `backups/schema-<previous version>-<random filename>.db` recovery copy. That filename is a local backup identity, not a user/device marker. Database upgrades run in one SQLite transaction. The baseline migration's registry timestamp is when its presence was recorded during this upgrade, not a fabricated original creation time. Reopening an upgraded database does not rerun migrations or create duplicate upgrade backups. Older NEXUS builds reject schema 3; keep the pre-upgrade backup if older-version recovery is necessary.

Known legacy relationship tokens gain `NX_` in current relationships, historical snapshot JSON and undo JSON. This prevents false ReconDelta changes and keeps undo compatible. Legacy timeline categories map to honest broad canonical events. In particular, `Authentication` becomes `nx.authentication.recorded`, never automatically `nx.authentication.confirmed`; existing messages, sources, entities and timestamps remain intact. Unknown historical event values are preserved.

NEXUS checks SQLite integrity, compatible format/schema versions, and the expected base tables before claiming an untagged legacy file. Other application IDs and future formats receive `NX-DB-9076`. Author/product attribution is not an anti-tamper gate: compatible private workspace extension fields and product attribution are preserved. Workspace metadata writes use a sibling temporary file and atomic replacement. A failed metadata write can be retried by reopening; the committed database upgrade stays valid.

Backups carry the SQLite application ID and database format metadata. Opening a recovered database recreates missing workspace JSON. Format metadata never contains credentials, client names, network addresses, local filesystem paths or build-host information.
