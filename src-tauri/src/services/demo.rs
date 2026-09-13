use crate::domain::DomainEvent;
use crate::{
    db::Store,
    models::{s, Result},
};
use serde_json::json;
pub fn canonical_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../fixtures/canonical-nexus-lab.json"))
        .expect("Canonical demo fixture must be valid JSON")
}
pub const DEMO_PASSPHRASE: &str = "nexus-demo-only";
pub fn seed(store: &mut Store) -> Result<()> {
    store.set_setting("vault_salt", &crate::vault::Vault::salt())?;
    let check = store
        .vault
        .unlock(DEMO_PASSPHRASE, &store.setting("vault_salt")?, None)?;
    store.set_setting("vault_check", &check)?;
    let fixture = canonical_fixture();
    for rule in fixture["scope"].as_array().unwrap() {
        store.insert("scope_rules", &json!({"rule":rule[0],"excluded":rule[1]}))?;
    }
    store.import_commit(
        "Nmap",
        "initial-discovery.xml",
        include_str!("../../../fixtures/small_lab/nmap.xml"),
        &[],
    )?;
    store.import_commit(
        "httpx",
        "web-observations.jsonl",
        include_str!("../../../fixtures/small_lab/httpx.jsonl"),
        &[],
    )?;
    let web = store.query(
        "SELECT id FROM assets WHERE ip=?",
        &[fixture["web_ip"].clone()],
    )?[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let file = store.query(
        "SELECT id FROM assets WHERE ip=?",
        &[fixture["file_ip"].clone()],
    )?[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    store.update("assets",&web,&json!({"direct":1,"access":"root","tags":"perimeter,linux","notes":"Fictional lab entry point. SSH access confirmed using the application service account."}))?;
    store.update("assets",&file,&json!({"access":"Administrator","tags":"internal,windows","notes":"Backup service identity has confirmed SMB administrative access."}))?;
    store.event(
        DomainEvent::AssetEnriched,
        Some(&web),
        "Fictional asset enriched with validated access and analyst notes",
        "DEMO",
    )?;
    let mut dc = String::new();
    for host in fixture["hosts"].as_array().unwrap() {
        let name = s(host, "name");
        let ip = s(host, "ip");
        let os = s(host, "os");
        let port = host["port"].as_i64().unwrap();
        let service = s(host, "service");
        let direct = host["direct"].as_i64().unwrap();
        let aid=store.insert("assets",&json!({"name":name,"ip":ip,"hostname":format!("{}.{}",name.to_lowercase(),s(&fixture,"domain")),"os":os,"direct":direct,"tags":if direct==1{"perimeter"}else{"internal"}}))?;
        store.insert("services",&json!({"asset_id":aid,"name":service,"port":port,"protocol":"tcp","tls":i64::from(port==443)}))?;
        if name == "NX-DEMO-DC01" {
            dc = aid;
        }
    }
    let svc_meridian = s(&fixture, "web_credential");
    let backup = s(&fixture, "archive_credential");
    let admin = s(&fixture, "admin_credential");
    for (cid, user, source, kind) in [
        (svc_meridian, "svc_meridian", web.as_str(), "Password"),
        (backup, "svc_archive", web.as_str(), "Password"),
        (admin, "lab_admin", file.as_str(), "NTLM hash"),
    ] {
        let ciphertext = store.vault.encrypt("DEMO-ONLY-FICTIONAL-SECRET", cid)?;
        store.insert("credentials",&json!({"id":cid,"name":user,"username":user,"context":"helios.lab","kind":kind,"ciphertext":ciphertext,"source_id":source,"source":"Fictional demo evidence","notes":"This credential is fictional and only demonstrates vault behavior."}))?;
    }
    let services = store.all("services")?;
    let webssh = services
        .iter()
        .find(|sv| s(sv, "asset_id") == web && sv["port"].as_i64() == Some(22))
        .unwrap()["id"]
        .clone();
    let filesmb = services
        .iter()
        .find(|sv| s(sv, "asset_id") == file && sv["port"].as_i64() == Some(445))
        .unwrap()["id"]
        .clone();
    let filewin = services
        .iter()
        .find(|sv| s(sv, "asset_id") == file && sv["port"].as_i64() == Some(5985))
        .unwrap()["id"]
        .clone();
    let dcsmb = services.iter().find(|sv| s(sv, "asset_id") == dc).unwrap()["id"].clone();
    for (credential, asset, service, result, privilege) in [
        (svc_meridian, web.as_str(), webssh, "Valid", "root"),
        (backup, file.as_str(), filesmb, "Valid", "Administrator"),
        (backup, file.as_str(), filewin, "Invalid", "Unknown"),
        (admin, dc.as_str(), dcsmb, "Valid", "Administrator"),
    ] {
        store.insert("credential_tests",&json!({"credential_id":credential,"asset_id":asset,"service_id":service,"result":result,"privilege":privilege,"source":"DEMO · manually validated lab observation"}))?;
    }
    let session=store.insert("sessions",&json!({"name":"NX-DEMO-WEB02 / root","asset_id":web,"username":"root","privilege":"root","kind":"SSH","state":"Active","credential_id":svc_meridian,"pivot_capable":1,"source":"DEMO · authorized lab access"}))?;
    store.insert("pivots",&json!({"name":"PIVOT-01","session_id":session,"asset_id":web,"network":"172.22.20.0/24","kind":"Ligolo","status":"Active","notes":"Fictional route through NX-DEMO-WEB02 into the internal segment."}))?;
    store.snapshot("01 · Initial foothold")?;
    store.import_commit(
        "Nuclei",
        "web-validation.jsonl",
        include_str!("../../../fixtures/small_lab/nuclei.jsonl"),
        &[],
    )?;
    let file_session=store.insert("sessions",&json!({"name":"NX-DEMO-FILE01 / administrator","asset_id":file,"username":"svc_archive","privilege":"Administrator","kind":"SMB administrative access","state":"Active","credential_id":backup,"pivot_capable":1,"source":"DEMO · explicit administrative success"}))?;
    store.insert("pivots",&json!({"name":"PIVOT-02","session_id":file_session,"asset_id":file,"network":"172.22.40.0/24","kind":"SSH tunnel","status":"Active","notes":"Second hop into the data segment."}))?;
    for (title,severity,aid,description,remediation) in [("Overprivileged backup identity","High",file.as_str(),"The fictional backup account has local administrative access beyond its operational need.","Constrain service-account privileges and rotate the credential after the assessment."),("SMB signing not enforced","Medium",dc.as_str(),"The lab domain controller does not require SMB signing. This is a demo observation.","Require SMB signing on domain assets after compatibility testing."),("Unreviewed administrative endpoint","Low",web.as_str(),"An administrative endpoint is present in the lab application and needs access-control review.","Limit management interfaces to the administrative network.")]{let fid=store.insert("findings",&json!({"name":title,"severity":severity,"asset_id":aid,"description":description,"impact":"Potential expansion of access within the fictional lab environment.","remediation":remediation,"status":"Confirmed"}))?;if severity!="Low"{store.insert("evidence",&json!({"name":format!("{title} · validation note"),"entity_id":fid,"kind":"Text note","body":"DEMO DATA — fictional analyst validation. No real systems or accounts are represented."}))?;}}
    store.update("assets", &dc, &json!({"access":"Administrator"}))?;
    store.set_setting("path_target", &dc)?;
    store.snapshot("02 · Internal access validated")?;
    store.event(
        DomainEvent::NetworkReachable,
        Some(&file),
        "Data segment became reachable through PIVOT-02",
        "DEMO",
    )?;
    store.event(
        DomainEvent::DemoReady,
        None,
        "Demo engagement ready — all identities and observations are fictional",
        "DEMO",
    )?;
    for (table, event) in [
        ("sessions", DomainEvent::SessionOpened),
        ("pivots", DomainEvent::PivotCreated),
        ("credentials", DomainEvent::CredentialRecorded),
    ] {
        for row in store.all(table)? {
            store.event(
                event,
                Some(s(&row, "id")),
                &format!("Fictional demo {} recorded", s(&row, "name")),
                "DEMO",
            )?;
        }
    }
    for row in store.all("findings")? {
        if s(&row, "status") == "Confirmed" {
            store.event(
                DomainEvent::FindingConfirmed,
                Some(s(&row, "id")),
                "Fictional finding validated",
                "DEMO",
            )?;
        }
    }
    for row in store.all("credential_tests")? {
        store.event(
            if s(&row, "result") == "Valid" {
                DomainEvent::AuthenticationConfirmed
            } else {
                DomainEvent::AuthenticationRecorded
            },
            Some(s(&row, "id")),
            "Fictional authentication result recorded",
            "DEMO",
        )?;
    }
    store.vault.lock();
    Ok(())
}
