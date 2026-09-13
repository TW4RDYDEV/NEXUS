use nexus_core::{core::Api, db::Store, models::s};
use serde_json::{json, Value};
use std::{fs, path::Path, time::Instant};
fn setup() -> (tempfile::TempDir, Api) {
    let dir = tempfile::tempdir().unwrap();
    let mut api = Api::new(dir.path().into()).unwrap();
    api.dispatch(
        "create",
        json!({"name":"Professional regression","kind":"Internal Pentest"}),
    )
    .unwrap();
    (dir, api)
}
const NESSUS: &str = r#"<NessusClientData_v2><Report name="test"><ReportHost name="10.22.0.1"><HostProperties><tag name="host-ip">10.22.0.1</tag></HostProperties><ReportItem port="443" protocol="tcp" svc_name="https" severity="2" pluginID="123" pluginName="Test TLS finding"><description>Analyst must validate</description><solution>Review configuration</solution><plugin_output>Observed test result</plugin_output></ReportItem><ReportItem port="22" protocol="tcp" svc_name="ssh" severity="1" pluginID="456" pluginName="SSH test"><plugin_output>Second service</plugin_output></ReportItem><ReportItem port="0" protocol="tcp" severity="0" pluginID="789" pluginName="Host observation"/></ReportHost></Report></NessusClientData_v2>"#;
const BURP: &str = r#"<!DOCTYPE issues [<!ELEMENT issues ANY>]><issues><issue><type>1048576</type><name>Test issue</name><host ip="10.22.0.2">https://app.example.test</host><path>/test</path><severity>High</severity><confidence>Certain</confidence><issueBackground><![CDATA[<script>alert('scanner text')</script>]]></issueBackground><issueDetail>Inspect output</issueDetail><remediationBackground>Encode output</remediationBackground></issue></issues>"#;

#[test]
fn locked_bundle_preserves_encrypted_netexec_source_and_vault() {
    let (_d, mut api) = setup();
    api.dispatch(
        "vault_unlock",
        json!({"password":"fictional-bundle-passphrase"}),
    )
    .unwrap();
    api.dispatch("import",json!({"tool":"NetExec","name":"fictional.txt","text":"SMB 10.0.0.2 445 LAB [+] LAB\\alice:FICTIONAL-BUNDLE-SECRET (Pwn3d!)"})).unwrap();
    let cid = api.store.as_ref().unwrap().all("credentials").unwrap()[0]["id"].clone();
    api.dispatch("vault_lock", json!({})).unwrap();
    let bundle = api.dispatch("workspace_export", json!({})).unwrap();
    assert_eq!(bundle["verified"], true);
    let imports = Path::new(s(&bundle, "path")).join("imports");
    for file in fs::read_dir(imports).unwrap() {
        assert!(!fs::read_to_string(file.unwrap().path())
            .unwrap()
            .contains("FICTIONAL-BUNDLE-SECRET"));
    }
    api.dispatch("recover_bundle", json!({"path":bundle["path"]}))
        .unwrap();
    assert!(api.dispatch("reveal", json!({"id":cid})).is_err());
    api.dispatch(
        "vault_unlock",
        json!({"password":"fictional-bundle-passphrase"}),
    )
    .unwrap();
    assert_eq!(
        api.dispatch("reveal", json!({"id":cid})).unwrap()["secret"],
        "FICTIONAL-BUNDLE-SECRET"
    );
}
#[test]
fn methodologies_deduplicate_require_results_and_preserve_undo() {
    let (_d, mut api) = setup();
    let added = api
        .dispatch(
            "assessment_template",
            json!({"template_id":"web-api","owner":"Analyst"}),
        )
        .unwrap();
    assert_eq!(added["added"], 6);
    assert_eq!(
        api.dispatch("assessment_template", json!({"template_id":"web-api"}))
            .unwrap()["added"],
        0
    );
    let plan = api.dispatch("assessment_plan", json!({})).unwrap();
    assert_eq!(
        plan["templates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["checks"].as_array().unwrap().len())
            .sum::<usize>(),
        52
    );
    let check = &plan["rows"][0];
    assert!(api
        .dispatch(
            "write",
            json!({"table":"assessment_checks","data":{"id":check["id"],"state":"Passed"}})
        )
        .is_err());
    api.dispatch("write",json!({"table":"assessment_checks","data":{"id":check["id"],"state":"Blocked","result":"Required account not provided"}})).unwrap();
    assert_eq!(
        api.dispatch("assessment_plan", json!({"state":"Blocked"}))
            .unwrap()["total"],
        1
    );
    api.dispatch("undo", json!({})).unwrap();
    assert_eq!(
        api.dispatch("assessment_plan", json!({"state":"Blocked"}))
            .unwrap()["total"],
        0
    );
    assert!(api
        .dispatch(
            "write",
            json!({"table":"assessment_checks","data":{"id":check["id"],"template_id":"forged"}})
        )
        .is_err());
}
#[test]
fn version_two_migrates_in_place_with_a_recovery_copy() {
    let dir = tempfile::tempdir().unwrap();
    let db = rusqlite::Connection::open(dir.path().join("nexus.db")).unwrap();
    db.execute_batch(include_str!("../migrations/001_nx_core_engagement.sql"))
        .unwrap();
    db.execute_batch(include_str!("../migrations/002_nx_format_identity.sql"))
        .unwrap();
    db.execute_batch("PRAGMA application_id=1314411859;PRAGMA user_version=2;INSERT INTO assets(id,name,first_seen,last_seen) VALUES('old','Preserved','then','then');").unwrap();
    drop(db);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(store.one("assets", "old").unwrap()["name"], "Preserved");
    assert_eq!(
        store.query("PRAGMA user_version", &[]).unwrap()[0]["user_version"],
        3
    );
    assert!(store.all("assessment_checks").unwrap().is_empty());
    assert_eq!(fs::read_dir(dir.path().join("backups")).unwrap().count(), 1);
}
#[test]
fn scanner_reports_are_drafts_deduplicate_and_link_the_right_service() {
    let (_d, mut api) = setup();
    for (tool, text) in [("Nessus", NESSUS), ("Burp", BURP)] {
        let args = json!({"tool":tool,"text":text,"name":format!("{tool}.xml")});
        api.dispatch("preview", args.clone()).unwrap();
        api.dispatch("import", args.clone()).unwrap();
        api.dispatch("import", args).unwrap();
    }
    let store = api.store.as_ref().unwrap();
    let findings = store.all("findings").unwrap();
    assert_eq!(findings.len(), 4);
    assert!(findings.iter().all(|f| f["status"] == "Draft"));
    for f in findings {
        let port = match s(&f, "template_id") {
            "nessus:123" => 443,
            "nessus:456" => 22,
            "burp:1048576" => 443,
            _ => {
                assert!(f["service_id"].is_null());
                continue;
            }
        };
        assert_eq!(
            store.one("services", s(&f, "service_id")).unwrap()["port"],
            port
        );
    }
    assert_eq!(store.all("evidence").unwrap().len(), 4);
}
#[test]
fn scanner_xml_rejects_entity_expansion_and_bad_ports() {
    let absolute = BURP.replace(
        "<path>/test</path>",
        "<path>https://app.example.test/test</path>",
    );
    let parsed = nexus_core::parsers::parse("Burp", &absolute).unwrap();
    assert!(s(&parsed.hosts[0].findings[0], "reproduction")
        .starts_with("https://app.example.test/test\n"));
    assert!(nexus_core::parsers::parse(
        "Burp",
        &absolute.replace(
            "<path>https://app.example.test/",
            "<path>https://different.example.test/"
        )
    )
    .is_err());
    for tool in ["Nessus", "Burp"] {
        assert!(nexus_core::parsers::parse(
            tool,
            "<!DOCTYPE issues [<!ENTITY x SYSTEM 'file:///secret'>]><issues>&x;</issues>"
        )
        .is_err());
    }
    assert!(nexus_core::parsers::parse(
        "Nessus",
        &NESSUS.replace("port=\"443\"", "port=\"70000\"")
    )
    .is_err());
}
#[test]
fn complete_bundles_recover_attachments_and_reject_tampering_and_paths() {
    let (_d, mut api) = setup();
    api.dispatch(
        "import",
        json!({"tool":"Nessus","name":"fixture.nessus","text":NESSUS}),
    )
    .unwrap();
    let entity = api.store.as_ref().unwrap().all("assets").unwrap()[0]["id"].clone();
    api.dispatch(
        "evidence_attach",
        json!({"entity_id":entity,"name":"Proof","kind":"Text note","body":"original bytes"}),
    )
    .unwrap();
    let export = api.dispatch("workspace_export", json!({})).unwrap();
    let path = Path::new(s(&export, "path"));
    assert_eq!(
        api.dispatch("workspace_verify", json!({"path":path}))
            .unwrap()["verified"],
        true
    );
    let original = api.store.as_ref().unwrap().path.clone();
    api.dispatch("recover_bundle", json!({"path":path}))
        .unwrap();
    assert_ne!(api.store.as_ref().unwrap().path, original);
    assert_eq!(
        api.store.as_ref().unwrap().all("evidence").unwrap().len(),
        4
    );
    let evidence = api
        .store
        .as_ref()
        .unwrap()
        .all("evidence")
        .unwrap()
        .into_iter()
        .find(|e| e["name"] == "Proof")
        .unwrap();
    assert_eq!(
        api.dispatch("evidence_read", json!({"id":evidence["id"]}))
            .unwrap()["text"],
        "original bytes"
    );
    fs::write(
        path.join("evidence").join(s(&evidence, "file_name")),
        "tampered",
    )
    .unwrap();
    assert!(api
        .dispatch("workspace_verify", json!({"path":path}))
        .is_err());
    let manifest_path = path.join("bundle-manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["files"][0]["path"] = json!("../outside");
    fs::write(manifest_path, manifest.to_string()).unwrap();
    assert!(api
        .dispatch("recover_bundle", json!({"path":path}))
        .is_err());
}
#[test]
fn report_escapes_untrusted_text_and_omits_drafts_by_default() {
    let (_d, mut api) = setup();
    api.dispatch(
        "import",
        json!({"tool":"Burp","name":"burp.xml","text":BURP}),
    )
    .unwrap();
    let report = api.dispatch("client_report", json!({})).unwrap();
    assert!(!s(&report, "text").contains("scanner text"));
    let report = api
        .dispatch("client_report", json!({"include_drafts":true}))
        .unwrap();
    assert!(s(&report, "text").contains("&lt;script&gt;"));
    assert!(!s(&report, "text").contains("<script>"));
    assert!(s(&report, "text").contains("default-src 'none'"));
}
#[test]
fn high_index_reference_matrix_graph_and_latest_authentication_are_accessible() {
    let (_d, mut api) = setup();
    let store = api.store.as_ref().unwrap();
    store.conn.execute_batch("BEGIN").unwrap();
    for i in 0..1505 {
        let asset = format!("a{i:04}");
        store.conn.execute("INSERT INTO assets(id,name,ip,direct,first_seen,last_seen) VALUES(?,?,?,1,'then','then')",rusqlite::params![asset,format!("Host {i:04}"),format!("10.0.{}.{}",i/256,i%256)]).unwrap();
        store.conn.execute("INSERT INTO credentials(id,name,username,kind,ciphertext,created_at) VALUES(?,?,?,'Password','encrypted-fixture','then')",rusqlite::params![format!("c{i:04}"),format!("User {i:04}"),format!("u{i:04}")]).unwrap();
    }
    for i in 1..=705 {
        store.conn.execute("INSERT INTO services(id,asset_id,name,port,first_seen,last_seen) VALUES(?,'a1504','service',?,'then','then')",rusqlite::params![format!("s{i:04}"),i]).unwrap();
    }
    store.conn.execute_batch("INSERT INTO credential_tests(id,credential_id,asset_id,service_id,result,created_at) VALUES('t1','c1504','a1504','s0705','Valid','then'),('t2','c1504','a1504','s0705','Invalid','then');COMMIT;").unwrap();
    let start = Instant::now();
    let refs = api
        .dispatch(
            "references",
            json!({"table":"assets","q":"Host 1504","selected_id":"a0000"}),
        )
        .unwrap();
    assert_eq!(refs["total"], 1);
    assert_eq!(refs["selected"]["id"], "a0000");
    assert_eq!(
        api.dispatch("references", json!({"table":"credentials","q":"User 1504"}))
            .unwrap()["rows"][0]["id"],
        "c1504"
    );
    let matrix = api
        .dispatch(
            "matrix_page",
            json!({"credential_id":"c1504","q":"Host 1504"}),
        )
        .unwrap();
    assert_eq!(matrix["hosts"][0]["service_total"], 705);
    assert_eq!(matrix["history"][0]["result"], "Invalid");
    assert!(api
        .store
        .as_ref()
        .unwrap()
        .access_path("a1504")
        .unwrap()
        .is_empty());
    let graph = api
        .dispatch(
            "graph",
            json!({"focus":"s0705","layers":["Host","Service"]}),
        )
        .unwrap();
    assert!(graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["id"] == "s0705"));
    assert_eq!(graph["nodes"].as_array().unwrap().len(), 500);
    assert!(api
        .dispatch("references", json!({"table":"assets;DROP TABLE assets"}))
        .is_err());
    eprintln!(
        "PROFESSIONAL SCALE 1505 identities + 1505 assets + 705 services: {:?}",
        start.elapsed()
    );
    assert!(start.elapsed().as_secs() < 10);
}
