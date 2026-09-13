//! Bounded SQL graph exploration; full inventories remain searchable and pageable.
use crate::{
    db::Store,
    models::{s, Result},
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

impl Store {
    pub(super) fn graph_records(
        &self,
        table: &str,
        selection: Option<&[String]>,
    ) -> Result<Vec<Value>> {
        let Some(ids) = selection else {
            return self.all(table);
        };
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders = vec!["?"; ids.len()].join(",");
        let fields = match table {
            "assets" => vec!["id"],
            "credentials" => vec!["id", "source_id"],
            "services" | "findings" => vec!["id", "asset_id"],
            "sessions" => vec!["id", "asset_id", "credential_id"],
            "pivots" => vec!["id", "asset_id", "session_id"],
            _ => return Err("Unsupported graph collection".into()),
        };
        let mut predicates = fields
            .iter()
            .map(|field| format!("{field} IN ({placeholders})"))
            .collect::<Vec<_>>();
        let mut values = fields
            .iter()
            .flat_map(|_| ids.iter().map(|id| json!(id)))
            .collect::<Vec<_>>();
        if table == "credentials" {
            predicates.push(format!("id IN (SELECT credential_id FROM credential_tests WHERE asset_id IN ({placeholders}))"));
            values.extend(ids.iter().map(|id| json!(id)));
        }
        values.extend(ids.iter().map(|id| json!(id)));
        self.query(&format!("SELECT * FROM {table} WHERE {} ORDER BY CASE WHEN id IN ({placeholders}) THEN 0 ELSE 1 END,id LIMIT 600",predicates.join(" OR ")),&values)
    }
    pub(super) fn graph_relationships(&self, nodes: &[Value], bounded: bool) -> Result<Vec<Value>> {
        if !bounded {
            return self.all("relationships");
        }
        let ids = nodes.iter().map(|n| n["id"].clone()).collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders = vec!["?"; ids.len()].join(",");
        let mut values = ids.clone();
        values.extend(ids);
        self.query(&format!("SELECT * FROM relationships WHERE source_id IN ({placeholders}) AND target_id IN ({placeholders}) ORDER BY id LIMIT 4000"),&values)
    }
    pub fn graph_view(&self, args: &Value) -> Result<Value> {
        let q = super::paging::pattern(s(args, "q"))?;
        let offset = args["offset"].as_i64().unwrap_or(0).max(0);
        let limit = 60;
        let total=self.query("SELECT count(*) AS n FROM assets WHERE archived=0 AND (name LIKE ? ESCAPE '\\' OR ip LIKE ? ESCAPE '\\' OR hostname LIKE ? ESCAPE '\\')",&[json!(q),json!(q),json!(q)])?[0]["n"].clone();
        let focus = s(args, "focus");
        let mut selected = BTreeSet::new();
        if focus.is_empty() {
            for row in self.query("SELECT id FROM assets WHERE archived=0 AND (name LIKE ? ESCAPE '\\' OR ip LIKE ? ESCAPE '\\' OR hostname LIKE ? ESCAPE '\\') ORDER BY name COLLATE NOCASE,id LIMIT ? OFFSET ?",&[json!(q),json!(q),json!(q),json!(limit),json!(offset)])?{selected.insert(s(&row,"id").to_string());}
        } else {
            selected.insert(focus.to_string());
            for row in self.query("SELECT source_id AS id FROM relationships WHERE target_id=? UNION SELECT target_id FROM relationships WHERE source_id=? LIMIT 180",&[json!(focus),json!(focus)])?{selected.insert(s(&row,"id").to_string());}
            for row in self.query("SELECT source AS id FROM nx_confirmed_access_edges WHERE target=? UNION SELECT target FROM nx_confirmed_access_edges WHERE source=? LIMIT 180",&[json!(focus),json!(focus)])?{selected.insert(s(&row,"id").to_string());}
            // Include the owner host for selected non-host entities, so a high-index
            // service or credential is usable without loading earlier inventories.
            let initial = selected.clone();
            for id in initial {
                for table in ["services", "sessions", "pivots", "findings"] {
                    if let Ok(row) = self.one(table, &id) {
                        let asset = s(&row, "asset_id");
                        if !asset.is_empty() {
                            selected.insert(asset.to_string());
                        }
                    }
                }
            }
            if let Some(network) = focus.strip_prefix("network:") {
                for row in self.query(
                    "SELECT id,asset_id,session_id FROM pivots WHERE network=? LIMIT 30",
                    &[json!(network)],
                )? {
                    for key in ["id", "asset_id", "session_id"] {
                        selected.insert(s(&row, key).to_string());
                    }
                }
            }
        }
        let mut path_ids = BTreeSet::new();
        if args["include_path"].as_bool() == Some(true) {
            let target = self.setting("path_target")?;
            if !target.is_empty() {
                let pinned: Vec<String> =
                    serde_json::from_str(&self.setting("pinned_path")?).unwrap_or_default();
                let path =
                    if pinned.last() == Some(&target) && self.validate_access_path(&pinned)? {
                        pinned
                    } else {
                        self.access_path(&target)?
                    };
                for id in path {
                    selected.insert(id.clone());
                    path_ids.insert(id);
                }
            }
        }
        let ids = selected.into_iter().collect::<Vec<_>>();
        let mut graph = self.build_graph(Some(&ids))?;
        let defaults = vec![
            "Operator",
            "Host",
            "Network",
            "Domain",
            "Web Application",
            "User",
            "Credential",
            "Session",
            "Pivot",
        ];
        let layers = args["layers"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .unwrap_or(defaults);
        let nodes = graph["nodes"].as_array().unwrap();
        let candidate_count = nodes
            .iter()
            .filter(|n| {
                layers.contains(&s(n, "kind"))
                    || s(n, "id") == focus
                    || path_ids.contains(s(n, "id"))
            })
            .count();
        let mut nodes = nodes
            .iter()
            .filter(|n| {
                layers.contains(&s(n, "kind"))
                    || s(n, "id") == focus
                    || path_ids.contains(s(n, "id"))
            })
            .cloned()
            .collect::<Vec<_>>();
        // Keep the exact requested entity in a capped focus view.
        nodes.sort_by_key(|n| {
            if s(n, "id") == focus || path_ids.contains(s(n, "id")) {
                0
            } else if s(n, "id") == "operator" {
                1
            } else {
                2
            }
        });
        nodes.truncate(500);
        let visible = nodes.iter().map(|n| s(n, "id")).collect::<BTreeSet<_>>();
        graph["edges"] = json!(graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| visible.contains(s(e, "source")) && visible.contains(s(e, "target")))
            .take(4000)
            .cloned()
            .collect::<Vec<_>>());
        graph["nodes"] = json!(nodes);
        graph["total"] = json!(candidate_count);
        graph["asset_total"] = total;
        graph["asset_offset"] = json!(offset);
        graph["asset_limit"] = json!(limit);
        graph["bounded"] = json!(true);
        Ok(graph)
    }
}
