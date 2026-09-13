//! NEXUS — TWARDY.exe / TW4RDYDEV
//! Bounded database views. A display cache never determines which records exist.
use crate::{
    db::Store,
    errors::ErrorCode,
    models::{s, Result},
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn pattern(query: &str) -> Result<String> {
    if query.len() > 200 {
        return Err("Search is limited to 200 characters".into());
    }
    Ok(format!(
        "%{}%",
        query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    ))
}
impl Store {
    pub fn references(&self, args: &Value) -> Result<Value> {
        let table = s(args, "table");
        let tables = if table == "all" {
            vec![
                "assets",
                "services",
                "credentials",
                "sessions",
                "pivots",
                "findings",
                "evidence",
                "assessment_checks",
            ]
        } else {
            vec![table]
        };
        let allowed = [
            "assets",
            "services",
            "credentials",
            "sessions",
            "pivots",
            "findings",
            "evidence",
            "assessment_checks",
        ];
        if tables.iter().any(|t| !allowed.contains(t)) {
            return Err("Unknown reference collection".into());
        }
        let mut parts = Vec::new();
        for table in &tables {
            let detail=match *table {
                "assets"=>"ip || ' ' || hostname",
                "services"=>"protocol || ':' || port || ' · ' || coalesce((SELECT name FROM assets WHERE id=t.asset_id),'')",
                "credentials"=>"context || ' · ' || kind",
                "sessions"=>"username || ' · ' || privilege",
                "pivots"=>"network",
                "findings"=>"severity || ' · ' || status",
                "assessment_checks"=>"area || ' · ' || state",
                _=>"kind",
            };
            let parent = if [
                "services",
                "sessions",
                "pivots",
                "findings",
                "assessment_checks",
            ]
            .contains(table)
            {
                "coalesce(asset_id,'')"
            } else {
                "''"
            };
            parts.push(format!("SELECT id,name,{detail} AS detail,'{table}' AS collection,{parent} AS asset_id FROM {table} t"));
        }
        if table == "all" {
            parts.push("SELECT 'operator' AS id,'Operator' AS name,'Assessment origin' AS detail,'operator' AS collection,'' AS asset_id".into());
        }
        let union = parts.join(" UNION ALL ");
        let predicate =
            "WHERE (name LIKE ? ESCAPE '\\' OR detail LIKE ? ESCAPE '\\') AND (?='' OR asset_id=?)";
        let pattern = pattern(s(args, "q"))?;
        let parent = s(args, "asset_id");
        let values = vec![json!(pattern), json!(pattern), json!(parent), json!(parent)];
        let total = self.query(
            &format!("SELECT count(*) AS n FROM ({union}) {predicate}"),
            &values,
        )?[0]["n"]
            .clone();
        let limit = args["limit"].as_i64().unwrap_or(50).clamp(1, 100);
        let offset = args["offset"].as_i64().unwrap_or(0).max(0);
        let mut page = values;
        page.extend([json!(limit), json!(offset)]);
        let rows=self.query(&format!("SELECT * FROM ({union}) {predicate} ORDER BY name COLLATE NOCASE,id LIMIT ? OFFSET ?"),&page)?;
        let selected = if s(args, "selected_id").is_empty() {
            Value::Null
        } else {
            self.query(
                &format!("SELECT * FROM ({union}) WHERE id=? AND (?='' OR asset_id=?) LIMIT 1"),
                &[args["selected_id"].clone(), json!(parent), json!(parent)],
            )?
            .into_iter()
            .next()
            .unwrap_or(Value::Null)
        };
        Ok(json!({"rows":rows,"total":total,"offset":offset,"limit":limit,"selected":selected}))
    }

    /// Resolve display labels on the rows being viewed, independent of the cache.
    pub fn decorate(&self, rows: &mut [Value]) -> Result<()> {
        let mut ids = HashSet::new();
        for row in rows.iter() {
            for (key, value) in row.as_object().unwrap() {
                if key.ends_with("_id") {
                    if let Some(id) = value.as_str().filter(|v| !v.is_empty()) {
                        ids.insert(id.to_string());
                    }
                }
            }
        }
        if ids.is_empty() {
            return Ok(());
        }
        let ids = ids.into_iter().collect::<Vec<_>>();
        let mut labels = HashMap::new();
        for chunk in ids.chunks(400) {
            let placeholders = vec!["?"; chunk.len()].join(",");
            let values = chunk.iter().map(|v| json!(v)).collect::<Vec<_>>();
            for table in [
                "assets",
                "services",
                "credentials",
                "sessions",
                "pivots",
                "findings",
                "evidence",
                "assessment_checks",
            ] {
                let name = if table == "services" {
                    "name || ' :' || port"
                } else {
                    "name"
                };
                for row in self.query(
                    &format!("SELECT id,{name} AS name FROM {table} WHERE id IN ({placeholders})"),
                    &values,
                )? {
                    labels.insert(s(&row, "id").to_string(), row["name"].clone());
                }
            }
        }
        for row in rows {
            let mut display = serde_json::Map::new();
            for (key, value) in row.as_object().unwrap() {
                if key.ends_with("_id") {
                    if let Some(label) = labels.get(value.as_str().unwrap_or("")) {
                        display.insert(key.clone(), label.clone());
                    }
                }
            }
            row["reference_labels"] = json!(display);
        }
        Ok(())
    }

    pub fn matrix_page(&self, args: &Value) -> Result<Value> {
        let credential = s(args, "credential_id");
        self.one("credentials", credential)?;
        let pattern = pattern(s(args, "q"))?;
        let limit = args["limit"].as_i64().unwrap_or(25).clamp(1, 50);
        let offset = args["offset"].as_i64().unwrap_or(0).max(0);
        let predicate="archived=0 AND (name LIKE ? ESCAPE '\\' OR ip LIKE ? ESCAPE '\\' OR hostname LIKE ? ESCAPE '\\')";
        let total = self.query(
            &format!("SELECT count(*) AS n FROM assets WHERE {predicate}"),
            &[json!(pattern), json!(pattern), json!(pattern)],
        )?[0]["n"]
            .clone();
        let mut hosts=self.query(&format!("SELECT id,name,ip,hostname FROM assets WHERE {predicate} ORDER BY name COLLATE NOCASE,id LIMIT ? OFFSET ?"),&[json!(pattern),json!(pattern),json!(pattern),json!(limit),json!(offset)])?;
        for host in &mut hosts {
            let total_services = self.query(
                "SELECT count(*) AS n FROM services WHERE asset_id=? AND status='open'",
                &[host["id"].clone()],
            )?[0]["n"]
                .clone();
            let services=self.query("SELECT sv.id,sv.name,sv.port,sv.protocol,t.id AS test_id,t.result,t.privilege,t.created_at AS tested_at FROM services sv LEFT JOIN credential_tests t ON t.rowid=(SELECT x.rowid FROM credential_tests x WHERE x.credential_id=? AND x.service_id=sv.id ORDER BY x.created_at DESC,x.rowid DESC LIMIT 1) WHERE sv.asset_id=? AND sv.status='open' ORDER BY sv.port,sv.id LIMIT 200",&[json!(credential),host["id"].clone()])?;
            host["service_total"] = total_services;
            host["services"] = json!(services);
        }
        let history_offset = args["history_offset"].as_i64().unwrap_or(0).max(0);
        let history_total = self.query(
            "SELECT count(*) AS n FROM credential_tests WHERE credential_id=?",
            &[json!(credential)],
        )?[0]["n"]
            .clone();
        let history=self.query("SELECT t.*,a.name AS asset_name,s.name || ' :' || s.port AS service_name FROM credential_tests t JOIN assets a ON a.id=t.asset_id JOIN services s ON s.id=t.service_id WHERE t.credential_id=? ORDER BY t.created_at DESC,t.rowid DESC LIMIT 50 OFFSET ?",&[json!(credential),json!(history_offset)])?;
        Ok(
            json!({"hosts":hosts,"total":total,"offset":offset,"limit":limit,"history":history,"history_total":history_total}),
        )
    }

    pub fn validate_access_path(&self, path: &[String]) -> Result<bool> {
        if path.len() < 2
            || path.len() > 100
            || path.first().map(String::as_str) != Some("operator")
        {
            return Ok(false);
        }
        for pair in path.windows(2) {
            if self.query("SELECT 1 AS present FROM nx_confirmed_access_edges WHERE source=? AND target=? LIMIT 1",&[json!(pair[0]),json!(pair[1])])?.is_empty(){return Ok(false);}
        }
        Ok(true)
    }
    /// Reverse breadth-first search asks SQLite only for incoming confirmed edges.
    pub fn access_path(&self, target: &str) -> Result<Vec<String>> {
        let mut queue = VecDeque::from([target.to_string()]);
        let mut seen = HashSet::from([target.to_string()]);
        let mut next = HashMap::new();
        while let Some(node) = queue.pop_front() {
            if node == "operator" {
                let mut path = vec![node];
                while path.last().unwrap() != target {
                    path.push(
                        next.get(path.last().unwrap())
                            .cloned()
                            .ok_or("Incomplete access path")?,
                    );
                }
                if path.len() > 100 {
                    return Err(
                        ErrorCode::GraphFailed.message("The confirmed path exceeds 100 steps")
                    );
                }
                return Ok(path);
            }
            for edge in self.query("SELECT DISTINCT source FROM nx_confirmed_access_edges WHERE target=? ORDER BY source",&[json!(node)])? {
                let source=s(&edge,"source").to_string();if seen.insert(source.clone()){next.insert(source.clone(),node.clone());queue.push_back(source);}
            }
            if seen.len() > 50_000 {
                return Err(ErrorCode::GraphFailed.message("Access exploration exceeded 50,000 entities; narrow the engagement or pin a known path"));
            }
        }
        Ok(vec![])
    }
}
