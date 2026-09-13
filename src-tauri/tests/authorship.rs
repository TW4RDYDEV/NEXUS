use nexus_core::{
    core::Api,
    db::{format::MIGRATIONS, Store},
    domain::{DomainEvent, RelationshipKind},
    errors::ErrorCode,
    identity::*,
    models::s,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{collections::HashSet, fs};

#[test]
fn sqlite_header_workspace_and_migrations_identify_the_format() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(NEXUS_PRODUCT_ID, "nexus.tw4rdy.core");
    assert_eq!(NEXUS_APPLICATION_ID, 0x4E585553);
    assert_eq!(NEXUS_APPLICATION_ID, 1314411859);
    let application_id: i32 = store
        .conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .unwrap();
    assert_eq!(application_id, NEXUS_APPLICATION_ID);
    let metadata = store
        .query("SELECT * FROM nx_format_metadata", &[])
        .unwrap();
    for (key, value) in [
        ("product_id", NEXUS_PRODUCT_ID),
        ("format", NEXUS_FORMAT_ID),
        ("schema_family", NEXUS_SCHEMA_FAMILY),
    ] {
        assert!(metadata
            .iter()
            .any(|r| r["key"] == key && r["value"] == value));
    }
    let workspace: Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("workspace.json")).unwrap())
            .unwrap();
    assert_eq!(workspace, workspace_metadata());
    let migrations = store
        .query("SELECT * FROM nx_migrations ORDER BY version", &[])
        .unwrap();
    assert_eq!(migrations.len(), MIGRATIONS.len());
    assert_eq!(migrations[0]["name"], "001_nx_core_engagement");
    assert_eq!(migrations[1]["name"], "002_nx_format_identity");
    drop(store);
    let bytes = fs::read(dir.path().join("nexus.db")).unwrap();
    assert_eq!(&bytes[68..72], &NEXUS_APPLICATION_ID.to_be_bytes());
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(
        reopened
            .query("SELECT * FROM nx_migrations", &[])
            .unwrap()
            .len(),
        MIGRATIONS.len()
    );
}

#[test]
fn canonical_registries_are_stable_unique_and_round_trip() {
    let expected = [
        "NX_EXPOSES",
        "NX_RESOLVES_TO",
        "NX_AUTHENTICATES_TO",
        "NX_DISCOVERED_FROM",
        "NX_HAS_SESSION",
        "NX_REACHABLE_VIA",
        "NX_AFFECTED_BY",
        "NX_EVIDENCED_BY",
        "NX_CONNECTS_TO",
        "NX_MEMBER_OF",
    ];
    assert_eq!(
        RelationshipKind::ALL
            .iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(RelationshipKind::ReachableVia.label(), "Reachable Via");
    for kind in RelationshipKind::ALL {
        assert_eq!(RelationshipKind::parse(kind.as_str()), Some(*kind));
    }
    let events = DomainEvent::ALL
        .iter()
        .map(|e| e.as_str())
        .collect::<HashSet<_>>();
    assert_eq!(events.len(), DomainEvent::ALL.len());
    for event in [
        "nx.asset.discovered",
        "nx.asset.enriched",
        "nx.service.observed",
        "nx.credential.recorded",
        "nx.authentication.confirmed",
        "nx.session.opened",
        "nx.pivot.created",
        "nx.network.reachable",
        "nx.finding.confirmed",
        "nx.snapshot.created",
        "nx.import.completed",
    ] {
        assert!(events.contains(event));
    }
    for event in DomainEvent::ALL {
        assert!(event.as_str().starts_with("nx."));
        assert_eq!(DomainEvent::parse(event.as_str()), Some(*event));
    }
    assert_eq!(
        ErrorCode::ALL
            .iter()
            .map(|c| c.as_str())
            .collect::<HashSet<_>>()
            .len(),
        ErrorCode::ALL.len()
    );
    assert_eq!(ErrorCode::PivotSourceMismatch.as_str(), "NX-PVT-7314");
}

#[test]
fn legacy_upgrade_preserves_data_undo_and_snapshot_meaning() {
    let dir = tempfile::tempdir().unwrap();
    let conn = Connection::open(dir.path().join("nexus.db")).unwrap();
    conn.execute_batch(include_str!("../migrations/001_nx_core_engagement.sql"))
        .unwrap();
    let legacy =
        json!({"source_id":"operator","target_id":"host","kind":"CONNECTS_TO","notes":"old"});
    conn.execute("INSERT INTO relationships(id,source_id,target_id,kind,notes,created_at) VALUES('rel','operator','host','CONNECTS_TO','old','then')",[]).unwrap();
    conn.execute(
        "INSERT INTO snapshots VALUES('snapshot','Before migration','then')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO snapshot_entities VALUES('snapshot','rel','relationships',?)",
        [legacy.to_string()],
    )
    .unwrap();
    let mut undo = legacy.clone();
    undo["id"] = json!("rel");
    conn.execute("INSERT INTO undo_history(table_name,entity_id,before_data,created_at) VALUES('relationships','rel',?,'then')",[undo.to_string()]).unwrap();
    conn.execute("INSERT INTO timeline_events VALUES('auth','Authentication','host','Invalid authentication recorded','NetExec','then')",[]).unwrap();
    drop(conn);
    let mut store = Store::open(dir.path()).unwrap();
    assert_eq!(
        store.one("relationships", "rel").unwrap()["kind"],
        "NX_CONNECTS_TO"
    );
    assert_eq!(
        store.all("timeline_events").unwrap()[0]["kind"],
        "nx.authentication.recorded"
    );
    assert_eq!(
        store.all("timeline_events").unwrap()[0]["name"],
        "Invalid authentication recorded"
    );
    let snapshot = store.snapshot("After migration").unwrap();
    assert!(store
        .diff("snapshot", s(&snapshot, "id"))
        .unwrap()
        .is_empty());
    store.undo().unwrap();
    assert_eq!(
        store.one("relationships", "rel").unwrap()["kind"],
        "NX_CONNECTS_TO"
    );
    let backups = fs::read_dir(dir.path().join("backups")).unwrap().count();
    assert_eq!(backups, 1);
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(
        store
            .query("SELECT * FROM nx_migrations", &[])
            .unwrap()
            .len(),
        MIGRATIONS.len()
    );
    assert_eq!(fs::read_dir(dir.path().join("backups")).unwrap().count(), 1);
}

#[test]
fn incompatible_formats_do_not_get_claimed_and_private_attribution_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let conn = Connection::open(dir.path().join("nexus.db")).unwrap();
    conn.execute_batch("CREATE TABLE unrelated(value TEXT); INSERT INTO unrelated VALUES('keep');")
        .unwrap();
    drop(conn);
    assert!(Store::open(dir.path())
        .err()
        .unwrap()
        .contains("NX-DB-9076"));
    let conn = Connection::open(dir.path().join("nexus.db")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA application_id", [], |r| r.get::<_, i32>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row("SELECT value FROM unrelated", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    let valid = tempfile::tempdir().unwrap();
    drop(Store::open(valid.path()).unwrap());
    let mut metadata = workspace_metadata();
    metadata["product_id"] = json!("private.fork");
    fs::write(valid.path().join("workspace.json"), metadata.to_string()).unwrap();
    drop(Store::open(valid.path()).unwrap());
    let saved: Value =
        serde_json::from_str(&fs::read_to_string(valid.path().join("workspace.json")).unwrap())
            .unwrap();
    assert_eq!(saved["product_id"], "private.fork");
    metadata["format_version"] = json!(99);
    fs::write(valid.path().join("workspace.json"), metadata.to_string()).unwrap();
    assert!(Store::open(valid.path())
        .err()
        .unwrap()
        .contains("NX-DB-9076"));
}

#[test]
fn build_diagnostic_is_an_allowlist_independent_of_engagement_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut api = Api::new(dir.path().into()).unwrap();
    let before = api.dispatch("diagnostic", json!({})).unwrap();
    api.dispatch(
        "create",
        json!({"name":"Sensitive client name","kind":"Other"}),
    )
    .unwrap();
    let after = api.dispatch("diagnostic", json!({})).unwrap();
    assert_eq!(before, after);
    let mut keys = after
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "application_version",
            "build_mode",
            "format",
            "git_commit",
            "product",
            "product_id",
            "schema_family",
            "schema_version"
        ]
    );
    assert_eq!(after["product_id"], "nexus.tw4rdy.core");
    assert_eq!(after, api.dispatch("build_metadata", json!({})).unwrap());
}

#[test]
fn actual_failures_expose_stable_codes_and_failed_writes_do_not_emit_success() {
    let dir = tempfile::tempdir().unwrap();
    let mut api = Api::new(dir.path().into()).unwrap();
    api.dispatch("demo", json!({})).unwrap();
    assert_eq!(
        api.dispatch("preview", json!({"tool":"Nmap","text":"bad xml"}))
            .unwrap_err()
            .code,
        "NX-IMP-2417"
    );
    assert_eq!(
        api.dispatch("snapshot", json!({"name":""}))
            .unwrap_err()
            .code,
        "NX-SNP-6193"
    );
    assert_eq!(
        api.dispatch("coverage", json!({"state":"NotReal"}))
            .unwrap_err()
            .code,
        "NX-COV-8251"
    );
    assert_eq!(
        api.dispatch("vault_unlock", json!({"password":"incorrect-password"}))
            .unwrap_err()
            .code,
        "NX-VLT-4826"
    );
    let store = api.store.as_ref().unwrap();
    let pivots = store.all("pivots").unwrap();
    let mut data = pivots[0].clone();
    data.as_object_mut().unwrap().remove("id");
    data.as_object_mut().unwrap().remove("created_at");
    data["asset_id"] = pivots[1]["asset_id"].clone();
    let count = store.all("timeline_events").unwrap().len();
    assert_eq!(
        api.dispatch("write", json!({"table":"pivots","data":data}))
            .unwrap_err()
            .code,
        "NX-PVT-7314"
    );
    assert_eq!(
        api.store
            .as_ref()
            .unwrap()
            .all("timeline_events")
            .unwrap()
            .len(),
        count
    );
}

#[test]
fn canonical_demo_exercises_graph_access_routes_and_recondelta() {
    let dir = tempfile::tempdir().unwrap();
    let mut api = Api::new(dir.path().into()).unwrap();
    api.dispatch("demo", json!({})).unwrap();
    let store = api.store.as_ref().unwrap();
    let assets = store.all("assets").unwrap();
    let host = |name: &str| assets.iter().find(|a| s(a, "name") == name).unwrap();
    let dc = host("NX-DEMO-DC01");
    let web = host("NX-DEMO-WEB02");
    let file = host("NX-DEMO-FILE01");
    assert_eq!(dc["ip"], "172.22.40.10");
    let reach = store.reach().unwrap();
    assert_eq!(reach[s(dc, "id")].len(), 2);
    assert_eq!(reach[s(file, "id")].len(), 1);
    assert!(reach[s(web, "id")].is_empty());
    let archive = store
        .all("credentials")
        .unwrap()
        .into_iter()
        .find(|c| s(c, "username") == "svc_archive")
        .unwrap();
    let graph = store.graph().unwrap();
    assert_eq!(
        graph["format_metadata"]["schema_family"],
        "nx-engagement-graph"
    );
    assert!(graph["edges"]
        .as_array()
        .unwrap()
        .iter()
        .all(|e| RelationshipKind::parse(s(e, "kind")).is_some()));
    assert!(graph["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["source"] == archive["id"]
            && e["target"] == file["id"]
            && e["kind"] == "NX_AUTHENTICATES_TO"));
    let events = store.all("timeline_events").unwrap();
    for kind in [
        DomainEvent::AssetDiscovered,
        DomainEvent::AssetEnriched,
        DomainEvent::ServiceObserved,
        DomainEvent::CredentialRecorded,
        DomainEvent::AuthenticationConfirmed,
        DomainEvent::SessionOpened,
        DomainEvent::PivotCreated,
        DomainEvent::NetworkReachable,
        DomainEvent::FindingConfirmed,
        DomainEvent::SnapshotCreated,
        DomainEvent::ImportCompleted,
    ] {
        assert!(
            events.iter().any(|e| e["kind"] == kind.as_str()),
            "Missing {}",
            kind.as_str()
        );
    }
    let snapshots = store
        .query("SELECT * FROM snapshots ORDER BY rowid", &[])
        .unwrap();
    let delta = store
        .diff(s(&snapshots[0], "id"), s(&snapshots[1], "id"))
        .unwrap();
    assert!(!delta.is_empty());
    for snapshot in snapshots {
        let metadata: Value = serde_json::from_str(s(&snapshot, "format_metadata")).unwrap();
        assert_eq!(metadata, workspace_metadata());
    }
    // Canonical names in independent NetExec fixtures resolve to the same host.
    let parsed = nexus_core::parsers::parse(
        "NetExec",
        include_str!("../../fixtures/internal_ad_lab/netexec.txt"),
    )
    .unwrap();
    assert!(parsed
        .hosts
        .iter()
        .any(|h| h.ip == "172.22.20.12" && h.auth.iter().any(|a| a["username"] == "svc_archive")));
}
