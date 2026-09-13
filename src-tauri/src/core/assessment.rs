//! Local methodology plans record analyst decisions; templates never imply execution.
use crate::{
    db::Store,
    domain::DomainEvent,
    models::{s, Result},
};
use serde_json::{json, Value};
pub fn templates() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/assessment-templates.json"))
        .expect("built-in assessment templates")
}
impl Store {
    pub fn assessment_plan(&self, args: &Value) -> Result<Value> {
        let q = super::paging::pattern(s(args, "q"))?;
        let area = s(args, "area");
        let state = s(args, "state");
        let owner = s(args, "owner");
        let predicate="WHERE (c.name LIKE ? ESCAPE '\\' OR c.objective LIKE ? ESCAPE '\\' OR c.owner LIKE ? ESCAPE '\\') AND (?='' OR c.area=?) AND (?='' OR c.state=?) AND (?='' OR c.owner=?)";
        let mut values = vec![
            json!(q),
            json!(q),
            json!(q),
            json!(area),
            json!(area),
            json!(state),
            json!(state),
            json!(owner),
            json!(owner),
        ];
        let total = self.query(
            &format!("SELECT count(*) AS n FROM assessment_checks c {predicate}"),
            &values,
        )?[0]["n"]
            .clone();
        values.extend([json!(args["offset"].as_i64().unwrap_or(0).max(0))]);
        let mut rows=self.query(&format!("SELECT c.* FROM assessment_checks c {predicate} ORDER BY CASE priority WHEN 'Critical' THEN 0 WHEN 'High' THEN 1 WHEN 'Normal' THEN 2 ELSE 3 END,area,name,id LIMIT 50 OFFSET ?"),&values)?;
        self.decorate(&mut rows)?;
        let counts=self.query("SELECT count(*) AS total,coalesce(sum(state IN ('Passed','Failed')),0) AS completed,coalesce(sum(state='Failed'),0) AS failed,coalesce(sum(state='Blocked'),0) AS blocked,coalesce(sum(state='Not applicable'),0) AS excluded,coalesce(sum(due_date!='' AND due_date<date('now') AND state NOT IN ('Passed','Failed','Not applicable')),0) AS overdue FROM assessment_checks",&[])?;
        Ok(
            json!({"rows":rows,"total":total,"counts":counts[0],"templates":templates(),"areas":self.query("SELECT DISTINCT area FROM assessment_checks ORDER BY area",&[])?}),
        )
    }
    pub fn add_assessment_template(&mut self, args: &Value) -> Result<Value> {
        let library = templates();
        let selected = library
            .as_array()
            .unwrap()
            .iter()
            .find(|t| s(t, "id") == s(args, "template_id"))
            .ok_or("Select a known assessment template")?;
        let asset = s(args, "asset_id");
        if !asset.is_empty() {
            self.one("assets", asset)?;
        }
        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| e.to_string())?;
        let result = (|| {
            let mut added = 0;
            for check in selected["checks"].as_array().unwrap() {
                let key = format!("{}:{}", s(selected, "id"), s(check, "id"));
                if !self.query("SELECT id FROM assessment_checks WHERE template_id=? AND coalesce(asset_id,'')=?",&[json!(key),json!(asset)])?.is_empty(){continue;}
                self.insert("assessment_checks",&json!({"name":check["name"],"area":selected["name"],"template_id":key,"objective":check["objective"],"reference_url":selected["reference_url"],"asset_id":if asset.is_empty(){Value::Null}else{json!(asset)},"owner":s(args,"owner"),"priority":"Normal","state":"Not started"}))?;
                added += 1;
            }
            self.event(
                DomainEvent::EntityEdited,
                None,
                &format!(
                    "Added {added} assessment checks from {}",
                    s(selected, "name")
                ),
                "Analyst",
            )?;
            Ok(json!({"added":added}))
        })();
        self.finish(result)
    }
    pub fn pivots_page(&self, args: &Value) -> Result<Value> {
        let q = super::paging::pattern(s(args, "q"))?;
        let offset = args["offset"].as_i64().unwrap_or(0).max(0);
        let total=self.query("SELECT count(*) AS n FROM pivots WHERE name LIKE ? ESCAPE '\\' OR network LIKE ? ESCAPE '\\'",&[json!(q),json!(q)])?[0]["n"].clone();
        let active = self.query(
            "SELECT count(*) AS n FROM pivots WHERE status='Active'",
            &[],
        )?[0]["n"]
            .clone();
        let mut rows=self.query("SELECT p.*,a.name AS source_name,s.username,s.privilege,s.state AS session_state FROM pivots p JOIN assets a ON a.id=p.asset_id JOIN sessions s ON s.id=p.session_id WHERE p.name LIKE ? ESCAPE '\\' OR p.network LIKE ? ESCAPE '\\' ORDER BY p.name,p.id LIMIT 20 OFFSET ?",&[json!(q),json!(q),json!(offset)])?;
        let reach = self.reach()?;
        for row in &mut rows {
            let mut ids = reach
                .iter()
                .filter(|(_, route)| route.iter().any(|p| p == s(row, "id")))
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            ids.sort();
            row["host_total"] = json!(ids.len());
            ids.truncate(20);
            let hosts = ids
                .iter()
                .map(|id| {
                    self.query("SELECT id,name,ip FROM assets WHERE id=?", &[json!(id)])
                        .map(|v| v[0].clone())
                })
                .collect::<Result<Vec<_>>>()?;
            row["hosts"] = json!(hosts);
        }
        Ok(json!({"rows":rows,"total":total,"active":active}))
    }
}
