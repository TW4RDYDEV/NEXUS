use crate::domain::DomainEvent;
use crate::{
    db::Store,
    models::{s, Result},
};
use crate::{domain::RelationshipKind, errors::ErrorCode};
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
pub fn reachability(
    assets: &[Value],
    sessions: &[Value],
    pivots: &[Value],
) -> HashMap<String, Vec<String>> {
    // Expand each active pivot once. Sorted IPv4 ranges avoid scanning every
    // asset for every route; breadth-first traversal retains a shortest route.
    let active = sessions
        .iter()
        .filter(|se| s(se, "state") == "Active")
        .map(|se| (s(se, "id"), s(se, "asset_id")))
        .collect::<HashMap<_, _>>();
    let mut by_source: BTreeMap<&str, Vec<(&str, ipnet::Ipv4Net)>> = BTreeMap::new();
    for p in pivots {
        if s(p, "status") != "Active" {
            continue;
        }
        if let Some(owner) = active.get(s(p, "session_id")) {
            if !owner.is_empty() && *owner != s(p, "asset_id") {
                continue;
            }
            if let Ok(net) = s(p, "network").parse::<ipnet::Ipv4Net>() {
                by_source
                    .entry(s(p, "asset_id"))
                    .or_default()
                    .push((s(p, "id"), net));
            }
        }
    }
    for routes in by_source.values_mut() {
        routes.sort_by_key(|r| r.0);
    }
    let mut addresses: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
    let mut reachable = HashMap::new();
    let mut direct = vec![];
    for a in assets {
        if a["archived"].as_i64() == Some(1) {
            continue;
        }
        let id = s(a, "id");
        if a["direct"].as_i64() == Some(1) {
            direct.push(id);
        }
        if let Ok(ip) = s(a, "ip").parse::<std::net::Ipv4Addr>() {
            addresses.entry(u32::from(ip)).or_default().push(id);
        }
    }
    direct.sort();
    let mut queue = VecDeque::new();
    for id in direct {
        reachable.insert(id.to_string(), vec![]);
        queue.push_back(id);
    }
    while let Some(source) = queue.pop_front() {
        if let Some(routes) = by_source.get(source) {
            for (pivot, net) in routes {
                let mut route = reachable[source].clone();
                route.push(pivot.to_string());
                for (_, ids) in
                    addresses.range(u32::from(net.network())..=u32::from(net.broadcast()))
                {
                    for id in ids {
                        if !reachable.contains_key(*id) {
                            reachable.insert(id.to_string(), route.clone());
                            queue.push_back(id);
                        }
                    }
                }
            }
        }
    }
    reachable
}

pub fn shortest_path(edges: &[Value], start: &str, end: &str) -> Vec<String> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for e in edges {
        if e["traversable"].as_bool() == Some(true) {
            adjacency
                .entry(s(e, "source"))
                .or_default()
                .push(s(e, "target"));
        }
    }
    for v in adjacency.values_mut() {
        v.sort();
        v.dedup()
    }
    let mut queue = VecDeque::from([start]);
    let mut previous: HashMap<&str, &str> = HashMap::new();
    let mut seen = HashSet::from([start]);
    while let Some(n) = queue.pop_front() {
        if n == end {
            let mut path = vec![n.to_string()];
            let mut cur = n;
            while let Some(p) = previous.get(cur) {
                path.push((*p).to_string());
                cur = p;
            }
            path.reverse();
            return path;
        }
        if let Some(neighbors) = adjacency.get(n) {
            for next in neighbors {
                if seen.insert(*next) {
                    previous.insert(*next, n);
                    queue.push_back(next);
                }
            }
        }
    }
    vec![]
}
pub fn family(service: &Value) -> &'static str {
    let n = s(service, "name").to_ascii_lowercase();
    match service["port"].as_i64().unwrap_or(0) {
        445 | 139 => "SMB",
        22 => "SSH",
        80 | 443 | 8000 | 8080 | 8443 => "HTTP",
        _ => {
            if n.contains("http") {
                "HTTP"
            } else {
                "Other"
            }
        }
    }
}
impl Store {
    pub fn reach(&self) -> Result<HashMap<String, Vec<String>>> {
        Ok(reachability(
            &self.query("SELECT id,ip,direct,archived FROM assets", &[])?,
            &self.query("SELECT id,asset_id,state FROM sessions", &[])?,
            &self.query(
                "SELECT id,session_id,asset_id,network,status FROM pivots",
                &[],
            )?,
        ))
    }
    pub fn graph(&self) -> Result<Value> {
        self.build_graph(None)
    }
    pub(super) fn build_graph(&self, selection: Option<&[String]>) -> Result<Value> {
        let assets = self.graph_records("assets", selection)?;
        let services = self.graph_records("services", selection)?;
        let credentials = self.graph_records("credentials", selection)?;
        let sessions = self.graph_records("sessions", selection)?;
        let pivots = self.graph_records("pivots", selection)?;
        let findings = self.graph_records("findings", selection)?;
        let reach = if selection.is_some() {
            self.reach()?
        } else {
            reachability(&assets, &sessions, &pivots)
        };
        let mut ports_by_asset: HashMap<&str, Vec<String>> = HashMap::new();
        for service in &services {
            if s(service, "status") == "open" {
                ports_by_asset
                    .entry(s(service, "asset_id"))
                    .or_default()
                    .push(service["port"].to_string());
            }
        }
        let mut nodes = vec![
            json!({"id":"operator","name":"Operator","kind":"Operator","subtitle":"Assessment origin","state":"active"}),
        ];
        let mut edges = vec![];
        let mut edge = |source: &str, target: &str, kind: RelationshipKind, traversable: bool| {
            let label = kind.label();
            let kind = kind.as_str();
            edges.push(json!({"id":format!("{source}:{kind}:{target}"),"source":source,"target":target,"kind":kind,"label":label,"traversable":traversable}))
        };
        for a in &assets {
            if a["archived"].as_i64() == Some(1) {
                continue;
            }
            let ports = ports_by_asset
                .get(s(a, "id"))
                .map(|p| p.join(" · "))
                .unwrap_or_default();
            nodes.push(json!({"id":a["id"],"name":a["name"],"kind":a["kind"],"subtitle":if s(a,"ip").is_empty(){s(a,"hostname")}else{s(a,"ip")},"detail":format!("{}\n{}",s(a,"os"),ports),"state":if s(a,"access")=="Administrator"||s(a,"access")=="root"{"privileged"}else if reach.contains_key(s(a,"id")){"reachable"}else{"unknown"}}));
            if a["direct"].as_i64() == Some(1) {
                edge("operator", s(a, "id"), RelationshipKind::ConnectsTo, false)
            }
        }
        for sv in &services {
            nodes.push(json!({"id":sv["id"],"name":format!("{} :{}",s(sv,"name"),sv["port"]),"kind":"Service","subtitle":sv["product"],"parent_id":sv["asset_id"],"state":sv["status"]}));
            edge(
                s(sv, "asset_id"),
                s(sv, "id"),
                RelationshipKind::Exposes,
                true,
            )
        }
        for c in &credentials {
            nodes.push(json!({"id":c["id"],"name":c["username"],"kind":"Credential","subtitle":c["context"]}));
            if !s(c, "source_id").is_empty() {
                edge(
                    s(c, "source_id"),
                    s(c, "id"),
                    RelationshipKind::DiscoveredFrom,
                    true,
                )
            }
        }
        for se in &sessions {
            nodes.push(json!({"id":se["id"],"name":se["name"],"kind":"Session","subtitle":format!("{} · {}",s(se,"username"),s(se,"privilege")),"state":se["state"]}));
            edge(
                s(se, "asset_id"),
                s(se, "id"),
                RelationshipKind::HasSession,
                s(se, "state") == "Active",
            );
            if s(se, "state") == "Active" {
                edge(
                    "operator",
                    s(se, "asset_id"),
                    RelationshipKind::HasSession,
                    true,
                )
            }
            if !s(se, "credential_id").is_empty() {
                edge(
                    s(se, "credential_id"),
                    s(se, "id"),
                    RelationshipKind::HasSession,
                    s(se, "state") == "Active",
                )
            }
        }
        for p in &pivots {
            nodes.push(json!({"id":p["id"],"name":p["name"],"kind":"Pivot","subtitle":p["network"],"state":p["status"]}));
            let netid = format!("network:{}", s(p, "network"));
            if !nodes.iter().any(|n| s(n, "id") == netid) {
                nodes.push(json!({"id":netid,"name":p["network"],"kind":"Network","subtitle":"Routed segment"}));
            }
            edge(
                s(p, "session_id"),
                s(p, "id"),
                RelationshipKind::ReachableVia,
                false,
            );
            edge(s(p, "id"), &netid, RelationshipKind::ConnectsTo, false);
            for a in &assets {
                let asset = s(a, "id");
                let Some(route) = reach.get(asset) else {
                    continue;
                };
                if route.last().is_some_and(|x| x == s(p, "id")) {
                    edge(&netid, asset, RelationshipKind::ReachableVia, false)
                }
            }
        }
        let tests = if selection.is_some() {
            let ids = assets.iter().map(|a| a["id"].clone()).collect::<Vec<_>>();
            if ids.is_empty() {
                vec![]
            } else {
                let placeholders = vec!["?"; ids.len()].join(",");
                self.query(&format!("SELECT t.* FROM credential_tests t WHERE t.asset_id IN ({placeholders}) AND t.rowid=(SELECT x.rowid FROM credential_tests x WHERE x.credential_id=t.credential_id AND x.service_id=t.service_id ORDER BY x.created_at DESC,x.rowid DESC LIMIT 1) ORDER BY t.id LIMIT 4000"),&ids)?
            }
        } else {
            self.query("SELECT t.* FROM credential_tests t WHERE t.rowid=(SELECT x.rowid FROM credential_tests x WHERE x.credential_id=t.credential_id AND x.service_id=t.service_id ORDER BY x.created_at DESC,x.rowid DESC LIMIT 1)",&[])?
        };
        for test in &tests {
            if s(test, "result") == "Valid" {
                edge(
                    s(test, "credential_id"),
                    s(test, "asset_id"),
                    RelationshipKind::AuthenticatesTo,
                    true,
                );
                edge(
                    s(test, "credential_id"),
                    s(test, "service_id"),
                    RelationshipKind::AuthenticatesTo,
                    true,
                )
            }
        }
        for f in &findings {
            nodes.push(json!({"id":f["id"],"name":f["name"],"kind":"Finding","subtitle":f["severity"],"state":f["status"]}));
            edge(
                s(f, "asset_id"),
                s(f, "id"),
                RelationshipKind::AffectedBy,
                false,
            )
        }
        for e in self.graph_relationships(&nodes, selection.is_some())? {
            edge(
                s(&e, "source_id"),
                s(&e, "target_id"),
                RelationshipKind::parse(s(&e, "kind")).ok_or_else(|| {
                    ErrorCode::GraphFailed.message("Unknown stored relationship kind")
                })?,
                false,
            )
        }
        let ids = nodes.iter().map(|n| s(n, "id")).collect::<HashSet<_>>();
        edges.retain(|e| ids.contains(s(e, "source")) && ids.contains(s(e, "target")));
        Ok(
            json!({"nodes":nodes,"edges":edges,"format_metadata":crate::identity::workspace_metadata()}),
        )
    }
    pub fn coverage(&self) -> Result<Value> {
        self.coverage_page("", "", 0, 500)
    }
    pub fn coverage_page(&self, q: &str, state: &str, offset: i64, limit: i64) -> Result<Value> {
        if !["", "Complete", "Partial", "Untested", "Not Applicable"].contains(&state) {
            return Err(ErrorCode::CoverageFailed.message("Unknown coverage state"));
        }
        let pattern = format!("%{q}%");
        let counts=self.query("SELECT count(*) AS total,coalesce(sum(state='Complete'),0) AS complete,coalesce(sum(state!='Not Applicable'),0) AS applicable FROM coverage_view",&[])?;
        let complete = counts[0]["complete"].as_i64().unwrap_or(0);
        let applicable = counts[0]["applicable"].as_i64().unwrap_or(0);
        let args = vec![
            json!(pattern),
            json!(pattern),
            json!(pattern),
            json!(state),
            json!(state),
        ];
        let predicate =
            "WHERE (name LIKE ? OR service LIKE ? OR asset_name LIKE ?) AND (?='' OR state=?)";
        let total = self.query(
            &format!("SELECT count(*) AS n FROM coverage_view {predicate}"),
            &args,
        )?[0]["n"]
            .clone();
        let mut args = args;
        args.push(json!(limit.clamp(1, 1000)));
        args.push(json!(offset.max(0)));
        let rows=self.query(&format!("SELECT * FROM coverage_view {predicate} ORDER BY asset_name,service,definition_id LIMIT ? OFFSET ?"),&args)?;
        Ok(
            json!({"rows":rows,"total":total,"complete":complete,"applicable":applicable,"percent":if applicable==0{Value::Null}else{json!(complete*100/applicable)}}),
        )
    }
    pub fn opportunities(&self) -> Result<Vec<Value>> {
        let reach = self.reach()?;
        let mut out = vec![];
        let credentials = self.query("SELECT count(*) AS n FROM credentials", &[])?[0]["n"]
            .as_i64()
            .unwrap_or(0);
        // Independent per-category limits keep summary latency and payload bounded.
        if credentials > 0 {
            let services=self.query("SELECT s.id,s.name,s.asset_id,s.port,?-(SELECT count(DISTINCT credential_id) FROM credential_tests t WHERE t.service_id=s.id) AS missing FROM services s WHERE s.status='open' AND (s.port IN (22,139,445,3389,5985,5986) OR lower(s.name) IN ('ssh','smb','microsoft-ds')) ORDER BY s.id",&[json!(credentials)])?;
            for sv in services {
                let missing = sv["missing"].as_i64().unwrap_or(0);
                if missing > 0 && reach.contains_key(s(&sv, "asset_id")) {
                    out.push(json!({"entity_id":sv["id"],"title":format!("{missing} credentials untested against {} :{}",s(&sv,"name"),sv["port"]),"why":"Reachable open service with identities that have no recorded test. Confirm authorization before validation.","kind":"Authentication"}));
                    if out.len() >= 50 {
                        break;
                    }
                }
            }
        }
        for row in self.query("SELECT service_id,service FROM coverage_view WHERE state='Untested' AND name='Content discovery' ORDER BY service_id LIMIT 50",&[])?{out.push(json!({"entity_id":row["service_id"],"title":format!("{} has no recorded content discovery",s(&row,"service")),"why":"The HTTP content-discovery check remains untested.","kind":"Coverage"}));}
        for t in self.query("SELECT asset_id FROM credential_tests t WHERE result='Valid' AND privilege='Unknown' AND t.rowid=(SELECT x.rowid FROM credential_tests x WHERE x.credential_id=t.credential_id AND x.service_id=t.service_id ORDER BY x.created_at DESC,x.rowid DESC LIMIT 1) ORDER BY created_at DESC LIMIT 50",&[])?{out.push(json!({"entity_id":t["asset_id"],"title":"Authentication confirmed; privilege remains unknown","why":"A successful test has no confirmed privilege level.","kind":"Access"}));}
        for f in self.query("SELECT id,name FROM findings f WHERE NOT EXISTS(SELECT 1 FROM evidence e WHERE e.entity_id=f.id) AND NOT EXISTS(SELECT 1 FROM assessment_checks c WHERE c.finding_id=f.id AND c.evidence_id IS NOT NULL) ORDER BY f.id LIMIT 50",&[])?{out.push(json!({"entity_id":f["id"],"title":format!("{} needs supporting evidence",s(&f,"name")),"why":"The finding has no linked evidence item.","kind":"Evidence"}));}
        Ok(out)
    }
    pub fn snapshot(&self, name: &str) -> Result<Value> {
        if name.trim().is_empty() {
            return Err("Name the snapshot".into());
        }
        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| e.to_string())?;
        let result = (|| {
            let sid = self.insert("snapshots", &json!({"name":name,"format_metadata":crate::identity::workspace_metadata().to_string()}))?;
            for (kind, eid, data) in self.meaningful_state()? {
                self.conn.execute("INSERT INTO snapshot_entities(snapshot_id,entity_id,kind,data) VALUES(?,?,?,?)",params![sid,eid,kind,data.to_string()]).map_err(|e|e.to_string())?;
            }
            self.event(
                DomainEvent::SnapshotCreated,
                Some(&sid),
                &format!("Snapshot created: {name}"),
                "Analyst",
            )?;
            Ok(json!({"id":sid}))
        })();
        self.finish(result)
    }
    fn meaningful_state(&self) -> Result<Vec<(String, String, Value)>> {
        let reach = self.reach()?;
        let mut out = vec![];
        for table in [
            "assets",
            "services",
            "sessions",
            "pivots",
            "findings",
            "relationships",
            "assessment_checks",
        ] {
            for mut row in self.all(table)? {
                let eid = s(&row, "id").to_string();
                if table == "assets" {
                    row["route"] = json!(reach.get(&eid));
                }
                let m = row.as_object_mut().unwrap();
                for key in [
                    "id",
                    "created_at",
                    "first_seen",
                    "last_seen",
                    "last_active",
                    "updated_at",
                ] {
                    m.remove(key);
                }
                out.push((table.into(), eid, row));
            }
        }
        Ok(out)
    }
    pub fn diff(&self, a: &str, b: &str) -> Result<Vec<Value>> {
        self.one("snapshots", a)?;
        self.one("snapshots", b)?;
        let read = |sid: &str| -> Result<BTreeMap<String, Value>> {
            Ok(self
                .query(
                    "SELECT * FROM snapshot_entities WHERE snapshot_id=?",
                    &[sid.into()],
                )?
                .into_iter()
                .map(|r| (s(&r, "entity_id").to_string(), r))
                .collect())
        };
        let before = read(a)?;
        let after = read(b)?;
        Ok(diff_maps(&before, &after))
    }
    pub fn summary(&mut self) -> Result<Value> {
        let reach = self.reach()?;
        let assets = self.query("SELECT id,access FROM assets WHERE archived=0", &[])?;
        let target = self.setting("path_target")?;
        let path = if target.is_empty() {
            vec![]
        } else {
            let pinned: Vec<String> =
                serde_json::from_str(&self.setting("pinned_path")?).unwrap_or_default();
            if pinned.last().map(String::as_str) == Some(&target)
                && self.validate_access_path(&pinned)?
            {
                pinned
            } else {
                self.access_path(&target)?
            }
        };
        let mut path_records = path
            .iter()
            .map(|id| json!({"entity_id":id}))
            .collect::<Vec<_>>();
        self.decorate(&mut path_records)?;
        let path_labels = path_records
            .iter()
            .map(|r| {
                (
                    s(r, "entity_id").to_string(),
                    r["reference_labels"]["entity_id"].clone(),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let route_labels = self
            .query("SELECT id,name FROM pivots", &[])?
            .into_iter()
            .map(|r| (s(&r, "id").to_string(), r["name"].clone()))
            .collect::<serde_json::Map<_, _>>();
        let mut counts = serde_json::Map::new();
        for table in [
            "services",
            "credentials",
            "sessions",
            "findings",
            "evidence",
        ] {
            counts.insert(
                table.into(),
                self.query(&format!("SELECT count(*) AS n FROM {table}"), &[])?[0]["n"].clone(),
            );
        }
        counts.insert("assets".into(), json!(assets.len()));
        counts.insert("reachable".into(), json!(reach.len()));
        counts.insert(
            "accessed".into(),
            json!(assets
                .iter()
                .filter(|a| !["None", "Unknown", ""].contains(&s(a, "access")))
                .count()),
        );
        counts.insert(
            "privileged".into(),
            json!(assets
                .iter()
                .filter(|a| ["root", "Administrator"].contains(&s(a, "access")))
                .count()),
        );
        Ok(
            json!({"format_metadata":crate::identity::workspace_metadata(),"relationship_vocabulary":RelationshipKind::registry(),"engagement":self.all("engagement")?.first(),"workspace":self.path,"counts":counts,"reachability":reach,"opportunities":self.opportunities()?,"coverage":self.coverage_page("", "", 0, 1)?,"path":path,"path_target":target,"path_labels":path_labels,"route_labels":route_labels,"vault_unlocked":self.vault.unlocked(),"vault_initialized":!self.setting("vault_salt")?.is_empty(),"timeline":self.query("SELECT * FROM timeline_events ORDER BY created_at DESC,rowid DESC LIMIT 100",&[])?,"snapshots":self.query("SELECT * FROM snapshots ORDER BY created_at DESC,rowid DESC",&[])?,"scope":self.all("scope_rules")?,"settings":{"auto_lock":self.setting("auto_lock")?,"density":self.setting("density")?,"reduced_motion":self.setting("reduced_motion")?,"backup_minutes":self.setting("backup_minutes")?}}),
        )
    }
}
pub fn diff_maps(before: &BTreeMap<String, Value>, after: &BTreeMap<String, Value>) -> Vec<Value> {
    let mut out = vec![];
    let ids = before
        .keys()
        .chain(after.keys())
        .collect::<std::collections::BTreeSet<_>>();
    for eid in ids {
        let a = before.get(eid);
        let b = after.get(eid);
        let status = match (a, b) {
            (None, Some(_)) => "New",
            (Some(_), None) => "Removed",
            (Some(a), Some(b)) if a["data"] != b["data"] => "Changed",
            _ => continue,
        };
        let old: Value = a
            .and_then(|a| serde_json::from_str(s(a, "data")).ok())
            .unwrap_or(Value::Null);
        let new: Value = b
            .and_then(|b| serde_json::from_str(s(b, "data")).ok())
            .unwrap_or(Value::Null);
        let fields = old
            .as_object()
            .into_iter()
            .flat_map(|o| o.keys())
            .chain(new.as_object().into_iter().flat_map(|o| o.keys()))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter(|k| old[*k] != new[*k])
            .map(|k| json!({"field":k,"before":old[k],"after":new[k]}))
            .collect::<Vec<_>>();
        out.push(json!({"entity_id":eid,"kind":b.or(a).unwrap()["kind"],"status":status,"name":new.get("name").or_else(||old.get("name")).cloned().unwrap_or(json!(eid)),"before":old,"after":new,"fields":fields}));
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multihop_reachability_requires_live_session() {
        let assets = vec![
            json!({"id":"a","direct":1}),
            json!({"id":"b","ip":"172.16.1.2"}),
            json!({"id":"c","ip":"172.16.2.2"}),
        ];
        let sessions = vec![
            json!({"id":"s1","state":"Active"}),
            json!({"id":"s2","state":"Active"}),
        ];
        let pivots = vec![
            json!({"id":"p2","asset_id":"b","session_id":"s2","network":"172.16.2.0/24","status":"Active"}),
            json!({"id":"p1","asset_id":"a","session_id":"s1","network":"172.16.1.0/24","status":"Active"}),
        ];
        assert_eq!(
            reachability(&assets, &sessions, &pivots)["c"],
            vec!["p1", "p2"]
        );
        assert!(!reachability(&assets, &[], &pivots).contains_key("c"));
    }
    #[test]
    fn paths_use_confirmed_direction_only() {
        let edges = vec![
            json!({"source":"a","target":"b","traversable":true}),
            json!({"source":"b","target":"c","traversable":false}),
        ];
        assert_eq!(shortest_path(&edges, "a", "b"), vec!["a", "b"]);
        assert!(shortest_path(&edges, "a", "c").is_empty());
        assert!(shortest_path(&edges, "b", "a").is_empty());
    }
    #[test]
    fn diff_reports_meaningful_fields() {
        let a = BTreeMap::from([(
            "x".into(),
            json!({"kind":"services","data":"{\"version\":\"1\"}"}),
        )]);
        let b = BTreeMap::from([(
            "x".into(),
            json!({"kind":"services","data":"{\"version\":\"2\"}"}),
        )]);
        let d = diff_maps(&a, &b);
        assert_eq!(d[0]["status"], "Changed");
        assert_eq!(d[0]["fields"][0]["field"], "version");
        assert!(diff_maps(&a, &a).is_empty());
    }
}
