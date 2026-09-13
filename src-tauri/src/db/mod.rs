pub mod format;
use crate::domain::DomainEvent;
use crate::{
    domain::RelationshipKind,
    errors::ErrorCode,
    models::provenance::{NormalizationRecord, ObservationConfidence, ObservationSource},
};
use crate::{
    models::{id, now, required, s, Result},
    vault::Vault,
};
use rusqlite::{params, types::Value as SqlValue, Connection};
use serde_json::{json, Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub const TABLES: &[&str] = &[
    "assessment_checks",
    "assets",
    "services",
    "credentials",
    "credential_tests",
    "sessions",
    "pivots",
    "findings",
    "evidence",
    "relationships",
    "scope_rules",
    "coverage_results",
];
pub struct Store {
    pub conn: Connection,
    pub path: PathBuf,
    pub vault: Vault,
    pub last_backup: std::time::Instant,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        fs::create_dir_all(path).map_err(|e| e.to_string())?;
        for d in ["evidence", "imports", "exports", "backups"] {
            fs::create_dir_all(path.join(d)).map_err(|e| e.to_string())?
        }
        let conn = Connection::open(path.join("nexus.db")).map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;PRAGMA journal_mode=WAL;PRAGMA synchronous=FULL;",
        )
        .map_err(|e| e.to_string())?;
        let check: String = conn
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(|e| format!("Database cannot be read: {e}. Restore a local backup."))?;
        if check != "ok" {
            return Err(ErrorCode::DatabaseIntegrityFailed.message(
                "Database integrity check failed. Open a backup from the welcome screen.",
            ));
        }
        format::initialize(&conn, path)?;
        let mut store = Self {
            conn,
            path: path.to_path_buf(),
            vault: Vault::default(),
            last_backup: std::time::Instant::now(),
        };
        store.seed_coverage()?;
        store.vault.interval = store.setting("auto_lock")?.parse().unwrap_or(300);
        Ok(store)
    }
    pub fn query(&self, sql: &str, input: &[Value]) -> Result<Vec<Value>> {
        let values = input.iter().map(sql_value).collect::<Result<Vec<_>>>()?;
        let mut st = self.conn.prepare(sql).map_err(|e| e.to_string())?;
        let cols = st
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let rows = st
            .query_map(rusqlite::params_from_iter(values.iter()), |row| {
                let mut m = Map::new();
                for (i, k) in cols.iter().enumerate() {
                    let value: SqlValue = row.get(i)?;
                    m.insert(
                        k.clone(),
                        match value {
                            SqlValue::Null => Value::Null,
                            SqlValue::Integer(v) => json!(v),
                            SqlValue::Real(v) => json!(v),
                            SqlValue::Text(v) => json!(v),
                            SqlValue::Blob(_) => Value::Null,
                        },
                    );
                }
                Ok(Value::Object(m))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
    pub fn all(&self, table: &str) -> Result<Vec<Value>> {
        if !TABLES.contains(&table)
            && ![
                "engagement",
                "timeline_events",
                "provenance",
                "imports",
                "snapshots",
                "coverage_definitions",
                "backups",
                "asset_aliases",
            ]
            .contains(&table)
        {
            return Err("Unknown entity collection".into());
        }
        self.query(&format!("SELECT * FROM {table}"), &[])
    }
    pub fn one(&self, table: &str, entity_id: &str) -> Result<Value> {
        if !TABLES.contains(&table) && !["engagement", "imports", "snapshots"].contains(&table) {
            return Err("Unknown entity collection".into());
        }
        self.query(
            &format!("SELECT * FROM {table} WHERE id=?"),
            &[entity_id.into()],
        )?
        .into_iter()
        .next()
        .ok_or("Entity does not exist".into())
    }
    pub fn setting(&self, key: &str) -> Result<String> {
        Ok(self
            .query("SELECT value FROM app_settings WHERE key=?", &[key.into()])?
            .first()
            .map(|r| s(r, "value").to_string())
            .unwrap_or_default())
    }
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute("INSERT INTO app_settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn event(
        &self,
        kind: DomainEvent,
        entity: Option<&str>,
        name: &str,
        source: &str,
    ) -> Result<()> {
        self.conn.execute("INSERT INTO timeline_events(id,kind,entity_id,name,source,created_at) VALUES(?,?,?,?,?,?)",params![id(),kind.as_str(),entity,name,source,now()]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn insert(&self, table: &str, data: &Value) -> Result<String> {
        if !TABLES.contains(&table)
            && ![
                "engagement",
                "imports",
                "snapshots",
                "provenance",
                "asset_aliases",
            ]
            .contains(&table)
        {
            return Err("Unknown entity collection".into());
        }
        let mut m = data.as_object().ok_or("Expected an object")?.clone();
        let eid = m
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(id);
        m.insert("id".into(), json!(eid));
        let schema = self.query(&format!("PRAGMA table_info({table})"), &[])?;
        let columns = schema.iter().map(|r| s(r, "name")).collect::<Vec<_>>();
        for field in [
            "created_at",
            "updated_at",
            "first_seen",
            "last_seen",
            "last_active",
        ] {
            if columns.contains(&field) && !m.contains_key(field) {
                m.insert(field.into(), json!(now()));
            }
        }
        if m.keys().any(|k| !columns.contains(&k.as_str())) {
            return Err("Unknown field in write request".into());
        }
        let keys = m.keys().cloned().collect::<Vec<_>>();
        let vals = keys
            .iter()
            .map(|k| sql_value(&m[k]))
            .collect::<Result<Vec<_>>>()?;
        let sql = format!(
            "INSERT INTO {table} ({}) VALUES ({})",
            keys.join(","),
            vec!["?"; keys.len()].join(",")
        );
        self.conn
            .execute(&sql, rusqlite::params_from_iter(vals.iter()))
            .map_err(|e| format!("Cannot write {table}: {e}"))?;
        Ok(eid)
    }
    pub fn update(&self, table: &str, eid: &str, data: &Value) -> Result<()> {
        if !TABLES.contains(&table) && table != "engagement" {
            return Err("Unknown entity collection".into());
        }
        let cols = self.query(&format!("PRAGMA table_info({table})"), &[])?;
        let fields = data.as_object().ok_or("Expected fields")?;
        if fields.is_empty() {
            return Ok(());
        }
        if fields
            .keys()
            .any(|k| k == "id" || !cols.iter().any(|r| s(r, "name") == k))
        {
            return Err("Invalid update field".into());
        }
        let mut vals = fields.values().map(sql_value).collect::<Result<Vec<_>>>()?;
        vals.push(eid.to_string().into());
        let set = fields
            .keys()
            .map(|k| format!("{k}=?"))
            .collect::<Vec<_>>()
            .join(",");
        self.conn
            .execute(
                &format!("UPDATE {table} SET {set} WHERE id=?"),
                rusqlite::params_from_iter(vals.iter()),
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn manual_write(&mut self, table: &str, data: &Value) -> Result<Value> {
        if !TABLES.contains(&table) && table != "engagement" {
            return Err("Unknown collection".into());
        }
        if table == "credentials" {
            return Err("Use the encrypted vault workflow to modify credentials".into());
        }
        for key in [
            "ciphertext",
            "file_name",
            "sha256",
            "demo",
            "created_at",
            "updated_at",
            "template_id",
            "first_seen",
            "last_seen",
        ] {
            if data.get(key).is_some() {
                return Err(format!(
                    "{key} is managed by NEXUS and cannot be changed through a manual edit"
                ));
            }
        }
        let eid = s(data, "id");
        let before = if eid.is_empty() {
            None
        } else {
            Some(self.one(table, eid)?)
        };
        let mut next = before.clone().unwrap_or(json!({}));
        for (k, v) in data.as_object().ok_or("Expected fields")? {
            next[k] = v.clone()
        }
        self.validate(table, &next).map_err(|error| match table {
            "scope_rules" => ErrorCode::ScopeRejected.message(error),
            "relationships" if !error.starts_with("[NX-") => ErrorCode::GraphFailed.message(error),
            "coverage_results" => ErrorCode::CoverageFailed.message(error),
            _ => error,
        })?;
        let previous_reach = if ["pivots", "sessions", "assets"].contains(&table) {
            Some(self.reach()?)
        } else {
            None
        };
        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| e.to_string())?;
        let result = (|| {
            let id = if eid.is_empty() {
                self.insert(table, data)?
            } else {
                let mut fields = data.clone();
                fields.as_object_mut().unwrap().remove("id");
                if table == "assessment_checks" {
                    fields["updated_at"] = json!(now());
                }
                self.update(table, eid, &fields)?;
                eid.to_string()
            };
            self.conn.execute("INSERT INTO undo_history(table_name,entity_id,before_data,created_at) VALUES(?,?,?,?)",params![table,id,before.as_ref().map(|v|v.to_string()),now()]).map_err(|e|e.to_string())?;
            self.event(
                match table {
                    "assets" if eid.is_empty() => DomainEvent::AssetDiscovered,
                    "services" if eid.is_empty() => DomainEvent::ServiceObserved,
                    "sessions"
                        if (s(&next, "state").is_empty() || s(&next, "state") == "Active")
                            && before.as_ref().is_none_or(|v| s(v, "state") != "Active") =>
                    {
                        DomainEvent::SessionOpened
                    }
                    "pivots" if eid.is_empty() => DomainEvent::PivotCreated,
                    "findings"
                        if s(&next, "status") == "Confirmed"
                            && before
                                .as_ref()
                                .is_none_or(|v| s(v, "status") != "Confirmed") =>
                    {
                        DomainEvent::FindingConfirmed
                    }
                    "credential_tests" if s(&next, "result") == "Valid" => {
                        DomainEvent::AuthenticationConfirmed
                    }
                    "credential_tests" => DomainEvent::AuthenticationRecorded,
                    _ => DomainEvent::EntityEdited,
                },
                Some(&id),
                &format!(
                    "{} {}",
                    if eid.is_empty() { "Added" } else { "Updated" },
                    s(&next, "name").to_owned()
                        + if s(&next, "name").is_empty() {
                            table
                        } else {
                            ""
                        }
                ),
                "Analyst",
            )?;
            if let Some(previous) = &previous_reach {
                for (asset, route) in self.reach()? {
                    if !previous.contains_key(&asset) {
                        self.event(
                            DomainEvent::NetworkReachable,
                            Some(&asset),
                            &format!("Asset became reachable through {} pivot hops", route.len()),
                            "NEXUS",
                        )?;
                    }
                }
            }
            for (field, value) in data.as_object().unwrap() {
                if field != "id" {
                    self.provenance(NormalizationRecord {
                        entity_id: &id,
                        field,
                        value: &value.to_string(),
                        source: ObservationSource::Analyst,
                        import_id: None,
                        confidence: ObservationConfidence::Confirmed,
                        conflict: false,
                    })?
                }
            }
            Ok(json!({"id":id}))
        })();
        self.finish(result)
    }
    pub fn finish<T>(&self, result: Result<T>) -> Result<T> {
        match result {
            Ok(v) => {
                self.conn
                    .execute_batch("COMMIT")
                    .map_err(|e| e.to_string())?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }
    pub fn validate(&self, table: &str, v: &Value) -> Result<()> {
        let enum_check = |field: &str, allowed: &[&str]| -> Result<()> {
            let val = s(v, field);
            if !val.is_empty() && !allowed.contains(&val) {
                Err(format!("Invalid {field}"))
            } else {
                Ok(())
            }
        };
        for (key, val) in v.as_object().ok_or("Invalid object")? {
            if val.as_str().map(|s| s.len() > 100_000).unwrap_or(false) {
                return Err(format!("{key} exceeds the field limit"));
            }
        }
        match table {
            "assessment_checks" => {
                required(v, "name")?;
                required(v, "area")?;
                enum_check(
                    "state",
                    &[
                        "Not started",
                        "In progress",
                        "Blocked",
                        "Passed",
                        "Failed",
                        "Not applicable",
                    ],
                )?;
                enum_check("priority", &["Critical", "High", "Normal", "Low"])?;
                if ["Passed", "Failed", "Blocked", "Not applicable"].contains(&s(v, "state")) {
                    required(v, "result")?;
                }
                for (key, table) in [
                    ("asset_id", "assets"),
                    ("evidence_id", "evidence"),
                    ("finding_id", "findings"),
                ] {
                    if !s(v, key).is_empty() {
                        self.one(table, s(v, key))?;
                    }
                }
                if !s(v, "due_date").is_empty() {
                    chrono::NaiveDate::parse_from_str(s(v, "due_date"), "%Y-%m-%d")
                        .map_err(|_| "Due date must be YYYY-MM-DD")?;
                }
            }
            "engagement" => {
                required(v, "name")?;
                enum_check("status", &["Active", "Paused", "Completed", "Archived"])?;
            }
            "assets" => {
                required(v, "name")?;
                enum_check(
                    "kind",
                    &["Host", "Network", "Domain", "Web Application", "User"],
                )?;
                enum_check(
                    "access",
                    &["None", "User", "Administrator", "root", "Unknown"],
                )?;
                if !s(v, "ip").is_empty() && s(v, "ip").parse::<std::net::Ipv4Addr>().is_err() {
                    return Err("Asset IP must be an IPv4 address".into());
                }
                if !s(v, "hostname").is_empty() {
                    crate::scope::validate(s(v, "hostname"))?;
                }
            }
            "services" => {
                self.one("assets", required(v, "asset_id")?)?;
                let port = v.get("port").and_then(Value::as_i64).unwrap_or(0);
                if !(1..=65535).contains(&port) {
                    return Err("Port must be between 1 and 65535".into());
                }
                enum_check("protocol", &["tcp", "udp"])?;
            }
            "scope_rules" => crate::scope::validate(required(v, "rule")?)?,
            "sessions" => {
                required(v, "name")?;
                required(v, "username")?;
                self.one("assets", required(v, "asset_id")?)?;
                enum_check("state", &["Active", "Stale", "Closed", "Unknown"])?;
            }
            "pivots" => {
                required(v, "name")?;
                let session = self.one("sessions", required(v, "session_id")?)?;
                if s(&session, "asset_id") != s(v, "asset_id") {
                    return Err(ErrorCode::PivotSourceMismatch
                        .message("Pivot source host must match its source session"));
                }
                required(v, "network")?
                    .parse::<ipnet::Ipv4Net>()
                    .map_err(|_| {
                        ErrorCode::PivotNetworkInvalid.message("Pivot network must be an IPv4 CIDR")
                    })?;
                enum_check("status", &["Active", "Inactive", "Unknown"])?;
            }
            "credential_tests" => {
                let svc = self.one("services", required(v, "service_id")?)?;
                if s(&svc, "asset_id") != required(v, "asset_id")? {
                    return Err("Authentication target does not match service host".into());
                }
                self.one("credentials", required(v, "credential_id")?)?;
                enum_check("result", &["Valid", "Invalid", "Unknown"])?;
            }
            "findings" => {
                required(v, "name")?;
                self.one("assets", required(v, "asset_id")?)?;
                enum_check("severity", &["Critical", "High", "Medium", "Low", "Info"])?;
                enum_check(
                    "status",
                    &[
                        "Draft",
                        "Confirmed",
                        "Remediated",
                        "Accepted",
                        "False Positive",
                    ],
                )?;
                if !s(v, "service_id").is_empty() {
                    let svc = self.one("services", s(v, "service_id"))?;
                    if s(&svc, "asset_id") != s(v, "asset_id") {
                        return Err("Finding service must belong to its affected asset".into());
                    }
                }
            }
            "relationships" => {
                let source = required(v, "source_id")?;
                let target = required(v, "target_id")?;
                if source == target {
                    return Err("A relationship needs two distinct entities".into());
                }
                if !self.entity_exists(source)? || !self.entity_exists(target)? {
                    return Err("Relationship endpoints must exist".into());
                }
                let kind = RelationshipKind::parse(required(v, "kind")?)
                    .ok_or_else(|| ErrorCode::GraphFailed.message("Unknown relationship kind"))?;
                if kind == RelationshipKind::AuthenticatesTo {
                    return Err(
                        "Record an authentication test to establish this relationship".into(),
                    );
                }
            }
            "evidence" => {
                required(v, "name")?;
                if !self.entity_exists(required(v, "entity_id")?)? {
                    return Err("Evidence must link to an existing entity".into());
                }
            }
            "coverage_results" => {
                enum_check(
                    "state",
                    &["Complete", "Partial", "Untested", "Not Applicable"],
                )?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn entity_exists(&self, eid: &str) -> Result<bool> {
        if eid == "operator" {
            return Ok(true);
        }
        for table in TABLES {
            if !self
                .query(&format!("SELECT id FROM {table} WHERE id=?"), &[eid.into()])?
                .is_empty()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn provenance(&self, record: NormalizationRecord<'_>) -> Result<()> {
        self.insert("provenance", &json!({"entity_id":record.entity_id,"field":record.field,"value":record.value,"source":record.source.as_str(),"import_id":record.import_id,"confidence":record.confidence.as_str(),"conflict":i64::from(record.conflict)}))?;
        Ok(())
    }
    pub fn backup(&mut self, label: &str) -> Result<Value> {
        let name = format!("{}-{}.db", chrono::Utc::now().format("%Y%m%dT%H%M%S"), id());
        let dest = self.path.join("backups").join(&name);
        self.conn
            .backup("main", &dest, None)
            .map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "INSERT INTO backups(id,name,created_at) VALUES(?,?,?)",
                params![id(), name, now()],
            )
            .map_err(|e| e.to_string())?;
        let mut files = fs::read_dir(self.path.join("backups"))
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "db"))
            .collect::<Vec<_>>();
        files.sort_by_key(|e| e.file_name());
        let excess = files.len().saturating_sub(20);
        for f in files.into_iter().take(excess) {
            fs::remove_file(f.path()).map_err(|e| e.to_string())?;
            self.conn
                .execute(
                    "DELETE FROM backups WHERE name=?",
                    [f.file_name().to_string_lossy().as_ref()],
                )
                .map_err(|e| e.to_string())?;
        }
        self.last_backup = std::time::Instant::now();
        self.event(DomainEvent::BackupCreated, None, label, "NEXUS")?;
        Ok(json!({"name":name}))
    }
    pub fn undo(&mut self) -> Result<Value> {
        let row = self
            .query("SELECT * FROM undo_history ORDER BY id DESC LIMIT 1", &[])?
            .into_iter()
            .next()
            .ok_or("There is no manual edit to undo")?;
        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| e.to_string())?;
        let result = (|| {
            let table = s(&row, "table_name");
            let eid = s(&row, "entity_id");
            if !TABLES.contains(&table) && table != "engagement" {
                return Err("Invalid undo collection".into());
            }
            if let Some(before) = row.get("before_data").and_then(Value::as_str) {
                let mut v: Value = serde_json::from_str(before).map_err(|e| e.to_string())?;
                v.as_object_mut().unwrap().remove("id");
                self.update(table, eid, &v)?;
            } else {
                self.conn.execute(&format!("DELETE FROM {table} WHERE id=?"),[eid]).map_err(|_|"This record has dependent records. Archive it or remove the dependent records first.")?;
            }
            self.conn
                .execute(
                    "DELETE FROM undo_history WHERE id=?",
                    [row["id"].as_i64().unwrap_or(0)],
                )
                .map_err(|e| e.to_string())?;
            self.event(
                DomainEvent::EditUndone,
                Some(eid),
                "Manual edit undone",
                "Analyst",
            )?;
            Ok(json!({"undone":true}))
        })();
        self.finish(result)
    }
    fn seed_coverage(&mut self) -> Result<()> {
        for (family, checks) in [
            (
                "SMB",
                vec![
                    "Service identified",
                    "Version identified",
                    "Signing checked",
                    "Anonymous authentication checked",
                    "Guest authentication checked",
                    "Shares enumerated",
                    "Known credentials tested",
                    "Administrative access checked",
                ],
            ),
            (
                "SSH",
                vec![
                    "Version identified",
                    "Known credentials tested",
                    "Key authentication context recorded",
                    "Access state known",
                ],
            ),
            (
                "HTTP",
                vec![
                    "Technology fingerprinting",
                    "Content discovery",
                    "TLS observation",
                    "Virtual host discovery state",
                    "Relevant findings reviewed",
                ],
            ),
            (
                "Other",
                vec![
                    "Service identified",
                    "Version identified",
                    "Access state known",
                ],
            ),
        ] {
            for (i, name) in checks.iter().enumerate() {
                self.conn.execute("INSERT OR IGNORE INTO coverage_definitions(id,service_family,name) VALUES(?,?,?)",params![format!("{family}-{i}"),family,name]).map_err(|e|e.to_string())?;
            }
        }
        Ok(())
    }
}
fn sql_value(v: &Value) -> Result<SqlValue> {
    Ok(match v {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Integer(i)
            } else {
                SqlValue::Real(n.as_f64().ok_or("Invalid number")?)
            }
        }
        Value::String(s) => SqlValue::Text(s.clone()),
        _ => return Err("Structured metadata is not accepted in scalar fields".into()),
    })
}
