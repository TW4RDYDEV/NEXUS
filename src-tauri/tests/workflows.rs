use nexus_core::{core::Api, models::s};
use serde_json::{json, Value};
fn setup() -> (tempfile::TempDir, Api) {
    let dir = tempfile::tempdir().unwrap();
    let mut api = Api::new(dir.path().into()).unwrap();
    api.dispatch(
        "create",
        json!({"name":"Integration lab","kind":"Lab / CTF"}),
    )
    .unwrap();
    (dir, api)
}
fn rows(api: &mut Api, table: &str) -> Vec<Value> {
    api.dispatch("list", json!({"table":table})).unwrap()["rows"]
        .as_array()
        .unwrap()
        .clone()
}
#[test]
fn nmap_to_sqlite_deduplicates_and_preserves_provenance() {
    let (_dir, mut api) = setup();
    let args = json!({"tool":"Nmap","name":"fixture.xml","text":include_str!("../../fixtures/small_lab/nmap.xml")});
    api.dispatch("import", args.clone()).unwrap();
    api.dispatch("import", args).unwrap();
    assert_eq!(rows(&mut api, "assets").len(), 2);
    assert_eq!(rows(&mut api, "services").len(), 5);
    api.dispatch("import",json!({"tool":"httpx","name":"httpx.jsonl","text":include_str!("../../fixtures/small_lab/httpx.jsonl")})).unwrap();
    assert_eq!(rows(&mut api, "assets").len(), 2);
    assert!(!rows(&mut api, "provenance").is_empty());
}
#[test]
fn manual_information_is_not_silently_overwritten() {
    let (_dir, mut api) = setup();
    api.dispatch("import",json!({"tool":"Nmap","name":"fixture.xml","text":include_str!("../../fixtures/small_lab/nmap.xml")})).unwrap();
    let a = rows(&mut api, "assets")
        .into_iter()
        .find(|a| s(a, "ip") == "10.20.4.17")
        .unwrap();
    api.dispatch(
        "write",
        json!({"table":"assets","data":{"id":a["id"],"os":"Analyst-confirmed OS"}}),
    )
    .unwrap();
    let text = include_str!("../../fixtures/small_lab/nmap.xml");
    let preview = api
        .dispatch("preview", json!({"tool":"Nmap","text":text}))
        .unwrap();
    assert!(!preview["conflicts"].as_array().unwrap().is_empty());
    api.dispatch(
        "import",
        json!({"tool":"Nmap","name":"again.xml","text":text}),
    )
    .unwrap();
    assert_eq!(
        api.store
            .as_ref()
            .unwrap()
            .one("assets", s(&a, "id"))
            .unwrap()["os"],
        "Analyst-confirmed OS"
    );
}
#[test]
fn engagement_isolation_and_restart() {
    let (dir, mut api) = setup();
    let path = api.store.as_ref().unwrap().path.clone();
    api.dispatch(
        "write",
        json!({"table":"assets","data":{"name":"isolated-host","ip":"10.0.0.2"}}),
    )
    .unwrap();
    api.dispatch("create", json!({"name":"Separate","kind":"Other"}))
        .unwrap();
    assert!(rows(&mut api, "assets").is_empty());
    drop(api);
    let mut reopened = Api::new(dir.path().into()).unwrap();
    reopened.dispatch("open", json!({"path":path})).unwrap();
    assert_eq!(rows(&mut reopened, "assets").len(), 1);
    assert!(
        !reopened.dispatch("summary", json!({})).unwrap()["vault_unlocked"]
            .as_bool()
            .unwrap()
    );
}
#[test]
fn demo_matrix_pivots_snapshots_and_vault() {
    let dir = tempfile::tempdir().unwrap();
    let mut api = Api::new(dir.path().into()).unwrap();
    let summary = api.dispatch("demo", json!({})).unwrap();
    assert_eq!(summary["counts"]["assets"], 8);
    assert_eq!(summary["counts"]["reachable"], 7);
    assert!(!summary["path"].as_array().unwrap().is_empty());
    assert!(!summary["vault_unlocked"].as_bool().unwrap());
    assert_eq!(rows(&mut api, "credential_tests").len(), 4);
    let snaps = summary["snapshots"].as_array().unwrap();
    let diff = api
        .dispatch(
            "diff",
            json!({"before":snaps[1]["id"],"after":snaps[0]["id"]}),
        )
        .unwrap();
    assert!(!diff.as_array().unwrap().is_empty());
    let c = rows(&mut api, "credentials")[0].clone();
    assert!(c.get("ciphertext").is_none());
    assert!(api.dispatch("reveal", json!({"id":c["id"]})).is_err());
    api.dispatch("vault_unlock", json!({"password":"nexus-demo-only"}))
        .unwrap();
    assert_eq!(
        api.dispatch("reveal", json!({"id":c["id"]})).unwrap()["secret"],
        "DEMO-ONLY-FICTIONAL-SECRET"
    );
}
#[test]
fn invalid_import_is_atomic_and_unknown_ipc_rejected() {
    let (_dir, mut api) = setup();
    assert!(api.dispatch("import",json!({"tool":"httpx","name":"bad.jsonl","text":"{\"url\":\"https://test.example\"}\nBAD"})).is_err());
    assert!(rows(&mut api, "assets").is_empty());
    assert!(api
        .dispatch(
            "write",
            json!({"table":"assets;DROP TABLE assets","data":{}})
        )
        .is_err());
    assert!(api.dispatch("shell", json!({})).is_err());
}
#[test]
fn coverage_is_only_based_on_observations() {
    let (_dir, mut api) = setup();
    api.dispatch("import",json!({"tool":"Nmap","name":"fixture.xml","text":include_str!("../../fixtures/small_lab/nmap.xml")})).unwrap();
    let summary = api.dispatch("summary", json!({})).unwrap();
    let full_coverage = api.dispatch("coverage", json!({"limit":500})).unwrap();
    assert_eq!(summary["coverage"]["complete"], full_coverage["complete"]);
    let coverage = full_coverage["rows"].as_array().unwrap();
    assert!(coverage
        .iter()
        .any(|c| c["name"] == "Version identified" && c["state"] == "Complete"));
    assert!(coverage
        .iter()
        .filter(|c| c["name"] == "Known credentials tested")
        .all(|c| c["state"] == "Untested"));
}
#[test]
fn undo_and_backup_recovery_preserve_original() {
    let (_dir, mut api) = setup();
    let id = api
        .dispatch("write", json!({"table":"assets","data":{"name":"before"}}))
        .unwrap()["id"]
        .clone();
    api.dispatch(
        "write",
        json!({"table":"assets","data":{"id":id,"name":"after"}}),
    )
    .unwrap();
    api.dispatch("undo", json!({})).unwrap();
    assert_eq!(rows(&mut api, "assets")[0]["name"], "before");
    let backup = api.dispatch("backup", json!({})).unwrap();
    let original = api.store.as_ref().unwrap().path.clone();
    api.dispatch("restore", json!({"name":backup["name"],"confirmed":true}))
        .unwrap();
    assert_ne!(api.store.as_ref().unwrap().path, original);
    assert!(original.join("nexus.db").is_file());
    assert_eq!(rows(&mut api, "assets")[0]["name"], "before");
}
