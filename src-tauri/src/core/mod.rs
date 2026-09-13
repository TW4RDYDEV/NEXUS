use crate::domain::DomainEvent;
use crate::{
    errors::{ErrorCode, NexusError},
    models::provenance::{NormalizationRecord, ObservationConfidence, ObservationSource},
};
mod assessment;
pub mod delivery;
mod graph_view;
pub mod intelligence;
pub mod paging;
use crate::{
    db::{Store, TABLES},
    models::{id, now, required, s, Result},
    services::runner::{self, Job},
};
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub struct Api {
    pub root: PathBuf,
    pub store: Option<Store>,
    pub job: Option<Job>,
}
impl Api {
    pub fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        Ok(Self {
            root,
            store: None,
            job: None,
        })
    }
    fn recent(&self) -> Vec<Value> {
        fs::read_to_string(self.root.join("recent.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn remember(&self, store: &Store) -> Result<()> {
        let e = store
            .all("engagement")?
            .into_iter()
            .next()
            .ok_or("Not a NEXUS engagement")?;
        let mut recent = self.recent();
        recent.retain(|r| r["path"] != json!(store.path));
        recent.insert(
            0,
            json!({"name":e["name"],"path":store.path,"demo":e["demo"],"status":e["status"]}),
        );
        recent.truncate(50);
        let temp = self.root.join("recent.tmp");
        fs::write(
            &temp,
            serde_json::to_vec_pretty(&recent).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(temp, self.root.join("recent.json")).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn dispatch(&mut self, op: &str, args: Value) -> std::result::Result<Value, NexusError> {
        self.dispatch_inner(op, args)
            .map_err(|e| NexusError::from_operation(op, e))
    }
    fn dispatch_inner(&mut self, op: &str, args: Value) -> Result<Value> {
        if op == "build_metadata" || op == "diagnostic" {
            return Ok(crate::identity::build_metadata());
        }
        if op == "vocabulary" {
            return Ok(
                json!({"events":DomainEvent::registry(),"relationships":crate::domain::RelationshipKind::registry()}),
            );
        }

        if serde_json::to_vec(&args).map_err(|e| e.to_string())?.len() > 50 * 1024 * 1024 {
            return Err("Request exceeds the size limit".into());
        }
        match op {
            "workspace_verify" => {
                return delivery::verify_bundle(Path::new(required(&args, "path")?))
            }
            "recover_bundle" => {
                if self.job.as_ref().is_some_and(|j| !j.done) {
                    return Err("Stop the running scan before switching engagements".into());
                }
                let destination = self.root.join("engagements").join(id()).join(".nexus");
                let recovered =
                    delivery::recover_bundle(Path::new(required(&args, "path")?), &destination)?;
                self.remember(&recovered)?;
                self.store = Some(recovered);
                return Ok(json!({"path":destination,"verified":true}));
            }
            "recover" => {
                if self.job.as_ref().is_some_and(|j| !j.done) {
                    return Err("Stop the running scan before switching engagements".into());
                }
                let source = PathBuf::from(required(&args, "path")?);
                if source.extension().and_then(|s| s.to_str()) != Some("db") || !source.is_file() {
                    return Err("Select a NEXUS .db backup file".into());
                }
                let destination = self.root.join("engagements").join(id()).join(".nexus");
                fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
                fs::copy(&source, destination.join("nexus.db")).map_err(|e| e.to_string())?;
                let recovered = Store::open(&destination)?;
                let engagement = recovered
                    .all("engagement")?
                    .into_iter()
                    .next()
                    .ok_or("This backup does not contain a NEXUS engagement")?;
                if let Some(workspace) = source
                    .parent()
                    .filter(|p| p.file_name().is_some_and(|n| n == "backups"))
                    .and_then(Path::parent)
                {
                    for dir in ["evidence", "imports"] {
                        if let Ok(entries) = fs::read_dir(workspace.join(dir)) {
                            for entry in entries.flatten() {
                                if entry.file_type().is_ok_and(|t| t.is_file()) {
                                    fs::copy(
                                        entry.path(),
                                        destination.join(dir).join(entry.file_name()),
                                    )
                                    .map_err(|e| e.to_string())?;
                                }
                            }
                        }
                    }
                }
                recovered.update(
                    "engagement",
                    s(&engagement, "id"),
                    &json!({"name":format!("{} · recovered",s(&engagement,"name"))}),
                )?;
                recovered.event(
                    DomainEvent::WorkspaceRecovered,
                    None,
                    "Recovered a backup into a separate workspace",
                    "Analyst",
                )?;
                self.remember(&recovered)?;
                self.store = Some(recovered);
                return self.store.as_mut().unwrap().summary();
            }
            "welcome" => {
                return Ok(
                    json!({"recent":self.recent(),"default_workspace":self.root.join("engagements")}),
                )
            }
            "create" | "demo" => {
                if self.job.as_ref().is_some_and(|j| !j.done) {
                    return Err("Stop the running scan before switching engagements".into());
                }
                let eid = id();
                let base = if !s(&args, "directory").is_empty() {
                    PathBuf::from(s(&args, "directory"))
                } else {
                    self.root.join("engagements")
                };
                let path = base.join(&eid).join(".nexus");
                let mut store = Store::open(&path)?;
                let demo_fixture = crate::services::demo::canonical_fixture();
                let name = if op == "demo" {
                    s(&demo_fixture, "name")
                } else {
                    required(&args, "name")?
                };
                store.insert("engagement",&json!({"id":eid,"name":name,"description":s(&args,"description"),"kind":if op=="demo"{"Internal Pentest"}else{required(&args,"kind")?},"start_date":if s(&args,"start_date").is_empty(){now()}else{s(&args,"start_date").to_string()},"client":s(&args,"client"),"demo":i64::from(op=="demo")}))?;
                store.event(
                    DomainEvent::EngagementCreated,
                    Some(&eid),
                    "Engagement created",
                    "Analyst",
                )?;
                if op == "demo" {
                    crate::services::demo::seed(&mut store)?;
                }
                self.remember(&store)?;
                self.store = Some(store);
                return self.store.as_mut().unwrap().summary();
            }
            "open" => {
                if self.job.as_ref().is_some_and(|j| !j.done) {
                    return Err("Stop the running scan before switching engagements".into());
                }
                let mut path = PathBuf::from(required(&args, "path")?);
                if path.is_file() {
                    path = path.parent().ok_or("Invalid workspace")?.into()
                }
                if path.join(".nexus/nexus.db").is_file() {
                    path = path.join(".nexus")
                }
                if !path.join("nexus.db").is_file() {
                    return Err("Select a NEXUS workspace containing nexus.db".into());
                }
                let store = Store::open(&path)?;
                self.remember(&store)?;
                self.store = Some(store);
                return self.store.as_mut().unwrap().summary();
            }
            "close" => {
                if self.job.as_ref().is_some_and(|j| !j.done) {
                    return Err("Stop the scan before closing the engagement".into());
                }
                self.store = None;
                self.job = None;
                return Ok(json!(true));
            }
            "integrations" => return Ok(runner::integrations()),
            _ => {}
        }
        let store = self.store.as_mut().ok_or("Open an engagement first")?;
        let backup_interval = store
            .setting("backup_minutes")?
            .parse::<u64>()
            .unwrap_or(10)
            .clamp(1, 120)
            * 60;
        if store.last_backup.elapsed().as_secs() > backup_interval {
            store.backup("Scheduled local backup")?;
        }
        match op {
            "references" => store.references(&args),
            "assessment_plan" => store.assessment_plan(&args),
            "assessment_template" => store.add_assessment_template(&args),
            "pivots_page" => store.pivots_page(&args),
            "matrix_page" => store.matrix_page(&args),
            "summary" => {
                if let Some(job) = self.job.as_mut() {
                    let _ = job.status(store, false)?;
                }
                store.summary()
            }
            "search" => {
                let q = required(&args, "q")?;
                if q.len() > 200 {
                    return Err("Search is limited to 200 characters".into());
                }
                let pattern = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
                let mut out = vec![];
                for (table, fields) in [
                    ("assets", vec!["name", "ip", "hostname", "os", "tags"]),
                    ("services", vec!["name", "port", "product", "url", "title"]),
                    ("credentials", vec!["username", "context", "source"]),
                    ("sessions", vec!["name", "username", "kind"]),
                    ("pivots", vec!["name", "network"]),
                    ("findings", vec!["name", "description", "cve", "cwe"]),
                    ("evidence", vec!["name", "kind"]),
                    (
                        "assessment_checks",
                        vec!["name", "area", "owner", "objective"],
                    ),
                ] {
                    let sql = format!(
                        "SELECT * FROM {table} WHERE {} LIMIT 8",
                        fields
                            .iter()
                            .map(|f| format!("{f} LIKE ? ESCAPE '\\'"))
                            .collect::<Vec<_>>()
                            .join(" OR ")
                    );
                    for row in store.query(&sql, &vec![json!(pattern); fields.len()])? {
                        out.push(json!({"id":row["id"],"table":table,"name":if s(&row,"name").is_empty(){s(&row,"username")}else{s(&row,"name")},"detail":if !s(&row,"ip").is_empty(){s(&row,"ip")}else if !s(&row,"context").is_empty(){s(&row,"context")}else{s(&row,"kind")}}));
                    }
                }
                Ok(json!(out))
            }
            "list" => {
                let table = required(&args, "table")?;
                if !TABLES.contains(&table)
                    && ![
                        "imports",
                        "timeline_events",
                        "snapshots",
                        "provenance",
                        "backups",
                    ]
                    .contains(&table)
                {
                    return Err("Unknown collection".into());
                }
                let cols = store.query(&format!("PRAGMA table_info({table})"), &[])?;
                let search_cols = cols
                    .iter()
                    .map(|r| s(r, "name"))
                    .filter(|c| !["ciphertext", "body", "value"].contains(c))
                    .collect::<Vec<_>>();
                let q = s(&args, "q");
                let mut clauses = vec![];
                let mut values = vec![];
                if !q.is_empty() {
                    clauses.push(format!(
                        "({})",
                        search_cols
                            .iter()
                            .map(|c| format!("{c} LIKE ? ESCAPE '\\'"))
                            .collect::<Vec<_>>()
                            .join(" OR ")
                    ));
                    let q = format!(
                        "%{}%",
                        q.replace('\\', "\\\\")
                            .replace('%', "\\%")
                            .replace('_', "\\_")
                    );
                    for _ in &search_cols {
                        values.push(q.clone().into())
                    }
                }
                if table == "assets" && args["archived"].as_bool() != Some(true) {
                    clauses.push("archived=0".into())
                }
                if let Some(filters) = args["filters"].as_object() {
                    for (field, value) in filters {
                        if !search_cols.contains(&field.as_str()) {
                            return Err("Invalid filter".into());
                        }
                        clauses.push(format!("{field}=?"));
                        values.push(if let Some(n) = value.as_i64() {
                            n.into()
                        } else {
                            s(filters.get(field).unwrap_or(&Value::Null), "").into()
                        });
                        let last = values.len() - 1;
                        values[last] = if let Some(n) = value.as_i64() {
                            n.into()
                        } else {
                            value.as_str().unwrap_or("").into()
                        };
                    }
                }
                let where_sql = if clauses.is_empty() {
                    String::new()
                } else {
                    format!(" WHERE {}", clauses.join(" AND "))
                };
                let count = store.query(
                    &format!("SELECT count(*) AS n FROM {table}{where_sql}"),
                    &values,
                )?[0]["n"]
                    .clone();
                let sort = if search_cols.contains(&s(&args, "sort")) {
                    s(&args, "sort")
                } else {
                    if search_cols.contains(&"name") {
                        "name"
                    } else {
                        "created_at"
                    }
                };
                let direction = if args["descending"].as_bool() == Some(true) {
                    "DESC"
                } else {
                    "ASC"
                };
                let limit = args["limit"].as_i64().unwrap_or(100).clamp(1, 1000);
                let offset = args["offset"].as_i64().unwrap_or(0).max(0);
                values.push(limit.into());
                values.push(offset.into());
                let mut rows=store.query(&format!("SELECT * FROM {table}{where_sql} ORDER BY {sort} {direction},id {direction} LIMIT ? OFFSET ?"),&values)?;
                for r in &mut rows {
                    r.as_object_mut().unwrap().remove("ciphertext");
                    if table == "timeline_events" {
                        if let Some(event) = DomainEvent::parse(s(r, "kind")) {
                            r["kind_label"] = json!(event.label());
                        }
                    }
                    if table == "relationships" {
                        if let Some(kind) = crate::domain::RelationshipKind::parse(s(r, "kind")) {
                            r["kind_label"] = json!(kind.label());
                        }
                    }
                }
                for row in &mut rows {
                    if table == "assets" {
                        row["observed_services"]=json!(store.query("SELECT id,port FROM services WHERE asset_id=? AND status='open' ORDER BY port,id LIMIT 12",&[row["id"].clone()])?);
                        row["service_count"] = store.query(
                            "SELECT count(*) AS n FROM services WHERE asset_id=? AND status='open'",
                            &[row["id"].clone()],
                        )?[0]["n"]
                            .clone();
                    }
                    if table == "findings" {
                        row["evidence_count"] = store.query(
                            "SELECT count(*) AS n FROM evidence WHERE entity_id=?",
                            &[row["id"].clone()],
                        )?[0]["n"]
                            .clone();
                    }
                }
                store.decorate(&mut rows)?;
                Ok(json!({"rows":rows,"total":count}))
            }
            "inspect" => {
                let eid = required(&args, "id")?;
                let mut entity = None;
                for table in TABLES {
                    if let Ok(mut found) = store.one(table, eid) {
                        found.as_object_mut().unwrap().remove("ciphertext");
                        entity = Some((table, found));
                        break;
                    }
                }
                let (table, mut row) = entity.ok_or("Entity is no longer available")?;
                store.decorate(std::slice::from_mut(&mut row))?;
                let mut related = vec![];
                for t in [
                    "services",
                    "sessions",
                    "pivots",
                    "findings",
                    "evidence",
                    "credentials",
                    "credential_tests",
                    "assessment_checks",
                ] {
                    let cols = store.query(&format!("PRAGMA table_info({t})"), &[])?;
                    let keys = cols
                        .iter()
                        .map(|r| s(r, "name"))
                        .filter(|c| c.ends_with("_id"))
                        .collect::<Vec<_>>();
                    if !keys.is_empty() {
                        let sql = format!(
                            "SELECT * FROM {t} WHERE {} LIMIT 100",
                            keys.iter()
                                .map(|k| format!("{k}=?"))
                                .collect::<Vec<_>>()
                                .join(" OR ")
                        );
                        for mut r in store.query(&sql, &vec![eid.into(); keys.len()])? {
                            r.as_object_mut().unwrap().remove("ciphertext");
                            related.push(json!({"table":t,"entity":r}));
                        }
                    }
                }
                let provenance=store.query("SELECT p.*, i.name AS source_file FROM provenance p LEFT JOIN imports i ON i.id=p.import_id WHERE p.entity_id=? ORDER BY p.created_at DESC LIMIT 100",&[eid.into()])?;
                let aliases = store.query(
                    "SELECT alias FROM asset_aliases WHERE asset_id=? ORDER BY alias",
                    &[json!(eid)],
                )?;
                Ok(
                    json!({"table":table,"entity":row,"related":related,"provenance":provenance,"aliases":aliases}),
                )
            }
            "alias" => {
                let asset = required(&args, "asset_id")?;
                store.one("assets", asset)?;
                let alias = required(&args, "alias")?.to_ascii_lowercase();
                crate::scope::validate(&alias)?;
                if alias.contains('*') || alias.contains('/') {
                    return Err("An alias must be one address or hostname".into());
                }
                store.insert("asset_aliases", &json!({"asset_id":asset,"alias":alias}))?;
                store.provenance(NormalizationRecord {
                    entity_id: asset,
                    field: "alias",
                    value: &alias,
                    source: ObservationSource::Analyst,
                    import_id: None,
                    confidence: ObservationConfidence::Confirmed,
                    conflict: false,
                })?;
                store.event(
                    DomainEvent::AssetAliasRecorded,
                    Some(asset),
                    "Asset alias recorded",
                    "Analyst",
                )?;
                Ok(json!(true))
            }
            "write" => store.manual_write(required(&args, "table")?, &args["data"]),
            "archive" => {
                if args["confirmed"].as_bool() != Some(true) {
                    return Err("Confirm archiving the selected assets".into());
                };
                let ids = args["ids"].as_array().ok_or("Select assets")?;
                for eid in ids {
                    store.manual_write("assets", &json!({"id":eid,"archived":1}))?;
                }
                Ok(json!(true))
            }
            "remove_scope" => {
                let eid = required(&args, "id")?;
                store
                    .conn
                    .execute("DELETE FROM scope_rules WHERE id=?", [eid])
                    .map_err(|e| e.to_string())?;
                store.event(
                    DomainEvent::ScopeChanged,
                    None,
                    "Scope rule removed",
                    "Analyst",
                )?;
                Ok(json!(true))
            }
            "undo" => store.undo(),
            "graph" => store.graph_view(&args),
            "path" => {
                let mut path = store.access_path(required(&args, "target")?)?;
                if let Some(proposed) = args["nodes"].as_array() {
                    path = proposed
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect();
                    if path.first().map(String::as_str) != Some("operator")
                        || path.len() < 2
                        || path.len() > 100
                        || path.last().map(String::as_str) != Some(required(&args, "target")?)
                        || !store.validate_access_path(&path)?
                    {
                        return Err("Every pinned path step must follow a known, confirmed access relationship".into());
                    }
                }
                if path.is_empty() {
                    return Err("No directed path through confirmed access relationships reaches this entity".into());
                }
                if args["pin"].as_bool() == Some(true) {
                    store.set_setting("path_target", s(&args, "target"))?;
                    store.set_setting(
                        "pinned_path",
                        &serde_json::to_string(&path).map_err(|e| e.to_string())?,
                    )?;
                    store.event(
                        DomainEvent::PathPinned,
                        Some(s(&args, "target")),
                        "Preferred attack path pinned",
                        "Analyst",
                    )?;
                }
                Ok(json!(path))
            }
            "snapshot" => store.snapshot(required(&args, "name")?),
            "coverage" => store.coverage_page(
                s(&args, "q"),
                s(&args, "state"),
                args["offset"].as_i64().unwrap_or(0),
                args["limit"].as_i64().unwrap_or(100),
            ),
            "diff" => Ok(json!(
                store.diff(required(&args, "before")?, required(&args, "after")?)?
            )),
            "preview" => store.import_preview(required(&args, "tool")?, required(&args, "text")?),
            "import" => store.import_commit(
                required(&args, "tool")?,
                required(&args, "name")?,
                required(&args, "text")?,
                &args["accept"]
                    .as_array()
                    .map(|v| {
                        v.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            ),
            "read_import" => {
                let path = PathBuf::from(required(&args, "path")?);
                let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
                if metadata.len() > crate::parsers::MAX_IMPORT as u64 {
                    return Err(ErrorCode::ImportTooLarge.message("Import exceeds 32 MiB"));
                }
                let text = fs::read_to_string(&path)
                    .map_err(|_| "Choose a UTF-8 XML, JSON, JSONL, or text file")?;
                Ok(
                    json!({"name":path.file_name().unwrap_or_default().to_string_lossy(),"text":text}),
                )
            }
            "vault_unlock" => {
                let mut salt = store.setting("vault_salt")?;
                let initial = salt.is_empty();
                if initial {
                    salt = crate::vault::Vault::salt()
                }
                let check = store.setting("vault_check")?;
                required(&args, "password")?;
                let mut password = zeroize::Zeroizing::new(s(&args, "password").to_string());
                let result =
                    store
                        .vault
                        .unlock(&password, &salt, if initial { None } else { Some(&check) });
                zeroize::Zeroize::zeroize(&mut *password);
                let check = result?;
                if initial {
                    store.set_setting("vault_salt", &salt)?;
                    store.set_setting("vault_check", &check)?;
                }
                store.event(
                    DomainEvent::VaultUnlocked,
                    None,
                    "Vault unlocked",
                    "Analyst",
                )?;
                Ok(json!(true))
            }
            "vault_lock" => {
                store.vault.lock();
                store.event(DomainEvent::VaultLocked, None, "Vault locked", "NEXUS")?;
                Ok(json!(true))
            }
            "credential" => {
                let username = required(&args, "username")?;
                let eid = if s(&args, "id").is_empty() {
                    id()
                } else {
                    s(&args, "id").to_string()
                };
                if s(&args, "secret").is_empty() {
                    return Err("Enter a non-empty secret".into());
                }
                let encrypted = store.vault.encrypt(s(&args, "secret"), &eid)?;
                let source = if s(&args, "source_id").is_empty() {
                    Value::Null
                } else {
                    store.one("assets", s(&args, "source_id"))?;
                    args["source_id"].clone()
                };
                let data = json!({"name":username,"username":username,"context":s(&args,"context"),"kind":required(&args,"kind")?,"source_id":source,"source":s(&args,"source"),"notes":s(&args,"notes"),"ciphertext":encrypted});
                if s(&args, "id").is_empty() {
                    let mut data = data;
                    data["id"] = json!(eid);
                    store.insert("credentials", &data)?;
                } else {
                    store.one("credentials", &eid)?;
                    store.update("credentials", &eid, &data)?;
                }
                store.event(
                    DomainEvent::CredentialRecorded,
                    Some(&eid),
                    "Credential inventory updated",
                    "Analyst",
                )?;
                Ok(json!({"id":eid}))
            }
            "reveal" => {
                let c = store.one("credentials", required(&args, "id")?)?;
                Ok(json!({"secret":store.vault.decrypt(s(&c,"ciphertext"),s(&c,"id"))?}))
            }
            "settings" => {
                for (key, value) in args.as_object().ok_or("Invalid preferences")? {
                    let value = value.as_str().ok_or("Invalid preference value")?;
                    match key.as_str() {
                        "auto_lock" => {
                            let seconds = value
                                .parse::<u64>()
                                .map_err(|_| "Invalid auto-lock duration")?;
                            if !(30..=3600).contains(&seconds) {
                                return Err("Auto-lock must be 30–3600 seconds".into());
                            }
                            store.vault.interval = seconds
                        }
                        "backup_minutes" => {
                            if !(1..=120).contains(
                                &value
                                    .parse::<u64>()
                                    .map_err(|_| "Invalid backup interval")?,
                            ) {
                                return Err("Backup interval must be 1–120 minutes".into());
                            }
                        }
                        "density" if ["compact", "comfortable"].contains(&value) => {}
                        "reduced_motion" if ["true", "false"].contains(&value) => {}
                        _ => return Err("Unknown or invalid preference".into()),
                    }
                    store.set_setting(key, value)?;
                }
                Ok(json!(true))
            }
            "backup" => store.backup("Manual backup created"),
            "restore" => {
                if args["confirmed"].as_bool() != Some(true) {
                    return Err("Confirm restoration into a separate recovered workspace".into());
                }
                let name = required(&args, "name")?;
                if Path::new(name).file_name().and_then(|n| n.to_str()) != Some(name)
                    || !name.ends_with(".db")
                {
                    return Err("Invalid backup name".into());
                }
                let destination = self.root.join("engagements").join(id()).join(".nexus");
                fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
                fs::copy(
                    store.path.join("backups").join(name),
                    destination.join("nexus.db"),
                )
                .map_err(|e| e.to_string())?;
                for d in ["evidence", "imports"] {
                    let to = destination.join(d);
                    fs::create_dir_all(&to).map_err(|e| e.to_string())?;
                    for f in fs::read_dir(store.path.join(d)).map_err(|e| e.to_string())? {
                        let f = f.map_err(|e| e.to_string())?;
                        if f.file_type().map_err(|e| e.to_string())?.is_file() {
                            fs::copy(f.path(), to.join(f.file_name()))
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
                let recovered = Store::open(&destination)?;
                let e = recovered.all("engagement")?[0].clone();
                recovered.update(
                    "engagement",
                    s(&e, "id"),
                    &json!({"name":format!("{} · recovered",s(&e,"name"))}),
                )?;
                recovered.event(
                    DomainEvent::WorkspaceRecovered,
                    None,
                    "Restored from local backup into a separate workspace",
                    "Analyst",
                )?;
                self.remember(&recovered)?;
                self.store = Some(recovered);
                Ok(json!({"path":destination}))
            }
            "evidence_attach" => {
                let entity = required(&args, "entity_id")?;
                if !store.entity_exists(entity)? {
                    return Err("Select an existing entity for evidence".into());
                }
                let eid = id();
                let mut bytes = if !s(&args, "path").is_empty() {
                    let path = Path::new(s(&args, "path"));
                    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 20 * 1024 * 1024 {
                        return Err("Evidence file exceeds 20 MiB".into());
                    }
                    fs::read(path).map_err(|e| e.to_string())?
                } else if !s(&args, "base64").is_empty() {
                    base64::engine::general_purpose::STANDARD
                        .decode(s(&args, "base64"))
                        .map_err(|_| "Invalid attachment encoding")?
                } else {
                    s(&args, "body").as_bytes().to_vec()
                };
                if bytes.len() > 20 * 1024 * 1024 {
                    return Err("Evidence exceeds 20 MiB".into());
                }
                let filename = format!("{eid}.attachment");
                fs::write(store.path.join("evidence").join(&filename), &bytes)
                    .map_err(|e| e.to_string())?;
                let result=store.insert("evidence",&json!({"id":eid,"name":required(&args,"name")?,"kind":required(&args,"kind")?,"entity_id":entity,"body":s(&args,"body"),"file_name":filename,"sha256":format!("{:x}",Sha256::digest(&bytes))}));
                zeroize::Zeroize::zeroize(&mut bytes);
                result?;
                store.event(
                    DomainEvent::EvidenceAttached,
                    Some(entity),
                    "Evidence attached",
                    "Analyst",
                )?;
                Ok(json!({"id":eid}))
            }
            "evidence_read" => {
                let e = store.one("evidence", required(&args, "id")?)?;
                let filename = s(&e, "file_name");
                if filename.is_empty() {
                    return Ok(json!({"text":e["body"]}));
                }
                if Path::new(filename).file_name().and_then(|p| p.to_str()) != Some(filename) {
                    return Err("Invalid internal evidence path".into());
                }
                let attachment = store
                    .path
                    .join("evidence")
                    .join(filename)
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                let evidence_root = store
                    .path
                    .join("evidence")
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                if !attachment.starts_with(evidence_root) {
                    return Err("Evidence path escapes the workspace".into());
                }
                let bytes = fs::read(attachment).map_err(|e| e.to_string())?;
                let digest = format!("{:x}", Sha256::digest(&bytes));
                if digest != s(&e, "sha256") {
                    return Err("Evidence integrity check failed".into());
                }
                let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                    "image/png"
                } else if bytes.starts_with(b"\xff\xd8\xff") {
                    "image/jpeg"
                } else {
                    ""
                };
                Ok(
                    json!({"mime":mime,"base64":base64::engine::general_purpose::STANDARD.encode(&bytes),"text":if mime.is_empty(){String::from_utf8(bytes).unwrap_or("Binary attachment. Export to inspect with a trusted viewer.".into())}else{String::new()},"verified":true}),
                )
            }
            "evidence_export" => {
                let e = store.one("evidence", required(&args, "id")?)?;
                let destination = PathBuf::from(required(&args, "path")?);
                let source = store.path.join("evidence").join(s(&e, "file_name"));
                if destination.exists() {
                    return Err(
                        "Choose a new export filename; existing files are never overwritten".into(),
                    );
                }
                if s(&e, "file_name").is_empty() {
                    fs::write(destination, s(&e, "body")).map_err(|e| e.to_string())?;
                } else {
                    let source = source.canonicalize().map_err(|e| e.to_string())?;
                    if !source.starts_with(
                        store
                            .path
                            .join("evidence")
                            .canonicalize()
                            .map_err(|e| e.to_string())?,
                    ) {
                        return Err("Evidence path escapes the workspace".into());
                    }
                    let bytes = fs::read(source).map_err(|e| e.to_string())?;
                    if format!("{:x}", Sha256::digest(&bytes)) != s(&e, "sha256") {
                        return Err("Evidence integrity check failed".into());
                    }
                    fs::write(destination, bytes).map_err(|e| e.to_string())?;
                }
                Ok(json!(true))
            }
            "runner_plan" => {
                let result =
                    runner::plan(store, required(&args, "target")?, required(&args, "ports")?);
                if result.is_err() {
                    store.event(
                        DomainEvent::ScanBlocked,
                        None,
                        "Scan blocked; target or integration validation failed",
                        "NEXUS",
                    )?;
                }
                result
            }
            "runner_start" => {
                if self.job.as_ref().is_some_and(|j| !j.done) {
                    return Err("A scan is already running".into());
                }
                self.job = Some(runner::start(
                    store,
                    required(&args, "target")?,
                    required(&args, "ports")?,
                )?);
                Ok(json!(true))
            }
            "runner_status" => match self.job.as_mut() {
                Some(job) => job.status(store, args["cancel"].as_bool() == Some(true)),
                None => Ok(Value::Null),
            },
            "workspace_export" => store.export_bundle(s(&args, "directory")),
            "client_report" => store.client_report(&args),
            "export_report" => {
                let mut report = format!(
                    "# NEXUS engagement report\n\n{}\n\nGenerated {}. Secrets are excluded.\n",
                    s(&store.all("engagement")?[0], "name"),
                    now()
                );
                for f in store.all("findings")? {
                    report.push_str(&format!("\n## {}\n\n{} · {}\n\n{}\n\n### Impact\n{}\n\n### Validation\n{}\n\n### Remediation\n{}\n\n### References\n{}\n",s(&f,"name"),s(&f,"severity"),s(&f,"status"),s(&f,"description"),s(&f,"impact"),s(&f,"reproduction"),s(&f,"remediation"),s(&f,"refs")));
                    for e in store.query(
                        "SELECT name,sha256 FROM evidence WHERE entity_id=?",
                        &[s(&f, "id").into()],
                    )? {
                        report.push_str(&format!(
                            "\nEvidence: {} · SHA-256 {}\n",
                            s(&e, "name"),
                            s(&e, "sha256")
                        ));
                    }
                }
                let name = format!(
                    "nexus-report-{}.md",
                    chrono::Utc::now().format("%Y%m%d-%H%M%S")
                );
                let path = store.path.join("exports").join(name);
                fs::write(&path, &report).map_err(|e| e.to_string())?;
                store.event(
                    DomainEvent::ReportExported,
                    None,
                    "Finding report exported; secrets excluded",
                    "Analyst",
                )?;
                Ok(json!({"path":path,"text":report}))
            }
            _ => Err("Unknown NEXUS operation".into()),
        }
    }
}
