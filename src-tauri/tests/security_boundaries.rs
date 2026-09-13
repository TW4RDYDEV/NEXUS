use nexus_core::{core::Api, models::s};
use serde_json::{json, Value};
fn setup() -> (tempfile::TempDir, Api) {
    let dir = tempfile::tempdir().unwrap();
    let mut api = Api::new(dir.path().into()).unwrap();
    api.dispatch(
        "create",
        json!({"name":"Security regression fixture","kind":"Lab / CTF"}),
    )
    .unwrap();
    (dir, api)
}
fn all(api: &mut Api, table: &str) -> Vec<Value> {
    api.dispatch("list", json!({"table":table})).unwrap()["rows"]
        .as_array()
        .unwrap()
        .clone()
}
#[test]
fn credential_whitespace_is_preserved_and_never_present_in_inventory_or_database() {
    let (_dir, mut api) = setup();
    api.dispatch(
        "vault_unlock",
        json!({"password":"fictional-test-passphrase"}),
    )
    .unwrap();
    let secret = "  FICTIONAL Secret: with spaces  ";
    let c = api
        .dispatch(
            "credential",
            json!({"username":"lab-user","kind":"Password","secret":secret}),
        )
        .unwrap();
    assert_eq!(
        api.dispatch("reveal", json!({"id":c["id"]})).unwrap()["secret"],
        secret
    );
    assert!(!all(&mut api, "credentials")
        .to_vec()
        .iter()
        .any(|r| r.get("ciphertext").is_some()));
    let stored = api.store.as_ref().unwrap().all("credentials").unwrap();
    assert!(!stored[0].to_string().contains(secret));
    assert!(api
        .dispatch("vault_unlock", json!({"password":"wrong-long-passphrase"}))
        .is_err());
    assert!(api.dispatch("reveal", json!({"id":c["id"]})).is_err());
}
#[test]
fn netexec_retention_is_encrypted_and_changed_secrets_are_distinct_credentials() {
    let (_dir, mut api) = setup();
    api.dispatch(
        "vault_unlock",
        json!({"password":"fictional-test-passphrase"}),
    )
    .unwrap();
    let line = "SMB 10.0.0.2 445 LAB [+] LAB\\alice:FICTIONAL-SECRET-A (Pwn3d!)";
    for text in [line.to_string(), line.replace("SECRET-A", "SECRET-B")] {
        api.dispatch(
            "import",
            json!({"tool":"NetExec","name":"lab.txt","text":text}),
        )
        .unwrap();
    }
    let credentials = all(&mut api, "credentials");
    assert_eq!(credentials.len(), 2);
    let mut revealed = credentials
        .iter()
        .map(|c| {
            api.dispatch("reveal", json!({"id":c["id"]})).unwrap()["secret"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    revealed.sort();
    assert_eq!(revealed, vec!["FICTIONAL-SECRET-A", "FICTIONAL-SECRET-B"]);
    for entry in std::fs::read_dir(api.store.as_ref().unwrap().path.join("imports")).unwrap() {
        assert!(!std::fs::read_to_string(entry.unwrap().path())
            .unwrap()
            .contains("FICTIONAL-SECRET"));
    }
}
#[test]
fn evidence_integrity_and_managed_field_boundaries_are_enforced() {
    let (dir, mut api) = setup();
    let a = api
        .dispatch("write", json!({"table":"assets","data":{"name":"LAB"}}))
        .unwrap();
    let e=api.dispatch("evidence_attach",json!({"entity_id":a["id"],"name":"Observation","kind":"Note","body":"original observation"})).unwrap();
    assert_eq!(
        api.dispatch("evidence_read", json!({"id":e["id"]}))
            .unwrap()["verified"],
        true
    );
    assert!(api
        .dispatch(
            "write",
            json!({"table":"evidence","data":{"id":e["id"],"file_name":"../../outside"}})
        )
        .is_err());
    let stored = api
        .store
        .as_ref()
        .unwrap()
        .one("evidence", s(&e, "id"))
        .unwrap();
    std::fs::write(
        api.store
            .as_ref()
            .unwrap()
            .path
            .join("evidence")
            .join(s(&stored, "file_name")),
        "tampered",
    )
    .unwrap();
    assert!(api
        .dispatch("evidence_read", json!({"id":e["id"]}))
        .is_err());
    let export = dir.path().join("export.txt");
    assert!(api
        .dispatch("evidence_export", json!({"id":e["id"],"path":export}))
        .is_err());
    assert!(!export.exists());
}
#[test]
fn welcome_recovery_opens_a_copy_and_scope_blocks_before_execution() {
    let (_dir, mut api) = setup();
    let original = api.store.as_ref().unwrap().path.clone();
    let backup = api.dispatch("backup", json!({})).unwrap();
    api.dispatch("close", json!({})).unwrap();
    api.dispatch(
        "recover",
        json!({"path":original.join("backups").join(s(&backup,"name"))}),
    )
    .unwrap();
    assert_ne!(api.store.as_ref().unwrap().path, original);
    assert!(original.join("nexus.db").is_file());
    let err = api
        .dispatch("runner_plan", json!({"target":"192.0.2.1","ports":"80"}))
        .unwrap_err();
    assert_eq!(err.code, "NX-SCP-2749");
    assert!(err.message.contains("Scope Guard"));
    assert!(api.job.is_none());
}
#[test]
fn manual_paths_cannot_claim_unconfirmed_access() {
    let dir = tempfile::tempdir().unwrap();
    let mut api = Api::new(dir.path().into()).unwrap();
    let summary = api.dispatch("demo", json!({})).unwrap();
    let path = summary["path"].as_array().unwrap();
    let target = path.last().unwrap();
    api.dispatch("path", json!({"target":target,"nodes":path,"pin":true}))
        .unwrap();
    assert!(api
        .dispatch(
            "path",
            json!({"target":target,"nodes":["operator","made-up-node",target],"pin":true})
        )
        .is_err());
}
