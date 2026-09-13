//! Complete local workspace transfer with independent attachment integrity checks.
use crate::{
    db::Store,
    domain::DomainEvent,
    identity,
    models::{id, now, s, Result},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};
fn hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
fn child_file(root: &Path, name: &str) -> Result<PathBuf> {
    let parts = name.split('/').collect::<Vec<_>>();
    if !(["nexus.db", "workspace.json"].contains(&name)
        || (parts.len() == 2
            && ["evidence", "imports"].contains(&parts[0])
            && !parts[1].is_empty()))
        || parts
            .iter()
            .any(|p| p.contains(['\\', ':']) || *p == "." || *p == "..")
    {
        return Err("Invalid bundle file path".into());
    }
    let path = root.join(name);
    let meta =
        fs::symlink_metadata(&path).map_err(|e| format!("Missing bundle file {name}: {e}"))?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("Bundle files must be regular files".into());
    }
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(root.canonicalize().map_err(|e| e.to_string())?) {
        return Err("Bundle file escapes its directory".into());
    }
    Ok(path)
}
pub fn verify_bundle(root: &Path) -> Result<Value> {
    let manifest = root.join("bundle-manifest.json");
    if fs::metadata(&manifest).map_err(|e| e.to_string())?.len() > 32 * 1024 * 1024 {
        return Err("Bundle manifest exceeds 32 MiB".into());
    }
    let data: Value = serde_json::from_slice(&fs::read(manifest).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if s(&data, "format") != "nx-workspace-bundle" || data["version"] != json!(1) {
        return Err("Unsupported workspace bundle".into());
    }
    let files = data["files"]
        .as_array()
        .ok_or("Bundle has no file manifest")?;
    if files.len() > 100_000 {
        return Err("Bundle contains more than 100,000 files".into());
    }
    let mut seen = BTreeSet::new();
    let mut bytes = 0u64;
    for file in files {
        let name = s(file, "path");
        if !seen.insert(name) {
            return Err("Duplicate bundle path".into());
        }
        let path = child_file(root, name)?;
        let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
        if Some(size) != file["bytes"].as_u64() || hash(&path)? != s(file, "sha256") {
            return Err(format!("Bundle integrity check failed: {name}"));
        }
        bytes += size;
    }
    if !seen.contains("nexus.db") || !seen.contains("workspace.json") {
        return Err("Bundle lacks database or format metadata".into());
    }
    // Validate database and evidence references without opening a writable Store.
    let conn = rusqlite::Connection::open_with_flags(
        root.join("nexus.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| e.to_string())?;
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if check != "ok" {
        return Err("Bundle database integrity failed".into());
    }
    let app: i64 = conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if app != i64::from(identity::NEXUS_APPLICATION_ID) {
        return Err("Bundle is not a NEXUS database".into());
    }
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if !(2..=identity::NEXUS_SCHEMA_VERSION).contains(&version) {
        return Err("Unsupported bundle database version".into());
    }
    let broken: bool = conn
        .prepare("PRAGMA foreign_key_check")
        .map_err(|e| e.to_string())?
        .query([])
        .map_err(|e| e.to_string())?
        .next()
        .map_err(|e| e.to_string())?
        .is_some();
    if broken {
        return Err("Bundle contains broken database references".into());
    }
    for table in ["evidence", "imports"] {
        let encrypted = if table == "imports" {
            "tool='NetExec'"
        } else {
            "0"
        };
        let mut st = conn
            .prepare(&format!(
                "SELECT file_name,sha256,{encrypted} AS encrypted FROM {table} WHERE file_name!=''"
            ))
            .map_err(|e| e.to_string())?;
        let refs = st
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, bool>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for pair in refs {
            let (name, digest, encrypted) = pair.map_err(|e| e.to_string())?;
            let name = format!("{table}/{name}");
            if !seen.contains(name.as_str())
                || (!encrypted && hash(&child_file(root, &name)?)? != digest)
            {
                return Err(format!("Attachment reference fails integrity: {name}"));
            }
        }
    }
    Ok(json!({"verified":true,"files":files.len(),"bytes":bytes,"manifest":data}))
}
pub fn recover_bundle(source: &Path, destination: &Path) -> Result<Store> {
    let verified = verify_bundle(source)?;
    if destination.exists() {
        return Err("Recovery destination already exists".into());
    }
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    for file in verified["manifest"]["files"].as_array().unwrap() {
        let name = s(file, "path");
        let from = child_file(source, name)?;
        let to = destination.join(name);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(from, to).map_err(|e| e.to_string())?;
    }
    // Re-verify copied bytes to catch source changes during the transfer.
    fs::write(
        destination.join("bundle-manifest.json"),
        serde_json::to_vec_pretty(&verified["manifest"]).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    verify_bundle(destination)?;
    fs::remove_file(destination.join("bundle-manifest.json")).map_err(|e| e.to_string())?;
    let store = Store::open(destination)?;
    store.event(
        DomainEvent::WorkspaceRecovered,
        None,
        "Recovered a verified complete workspace bundle",
        "Analyst",
    )?;
    Ok(store)
}
impl Store {
    pub fn export_bundle(&self, directory: &str) -> Result<Value> {
        let parent = if directory.is_empty() {
            self.path.join("exports")
        } else {
            PathBuf::from(directory)
        };
        if !parent.is_dir() {
            return Err("Choose an existing export directory".into());
        }
        let dest = parent.join(format!("NEXUS-workspace-{}", id()));
        fs::create_dir(&dest).map_err(|e| e.to_string())?;
        self.conn
            .backup("main", dest.join("nexus.db"), None)
            .map_err(|e| e.to_string())?;
        fs::copy(
            self.path.join("workspace.json"),
            dest.join("workspace.json"),
        )
        .map_err(|e| e.to_string())?;
        let mut names = vec!["nexus.db".to_string(), "workspace.json".to_string()];
        for dir in ["evidence", "imports"] {
            fs::create_dir(dest.join(dir)).map_err(|e| e.to_string())?;
            for entry in fs::read_dir(self.path.join(dir)).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let name = format!("{dir}/{}", entry.file_name().to_string_lossy());
                let source = child_file(&self.path, &name)?;
                fs::copy(source, dest.join(&name)).map_err(|e| e.to_string())?;
                names.push(name);
            }
        }
        names.sort();
        let files=names.iter().map(|name|{let path=dest.join(name);Ok(json!({"path":name,"bytes":fs::metadata(&path).map_err(|e|e.to_string())?.len(),"sha256":hash(&path)?}))}).collect::<Result<Vec<_>>>()?;
        let manifest = json!({"format":"nx-workspace-bundle","version":1,"created_at":now(),"format_metadata":identity::workspace_metadata(),"files":files});
        fs::write(
            dest.join("bundle-manifest.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let result = verify_bundle(&dest)?;
        self.event(
            DomainEvent::ReportExported,
            None,
            "Complete workspace exported and integrity verified",
            "Analyst",
        )?;
        Ok(json!({"path":dest,"files":result["files"],"bytes":result["bytes"],"verified":true}))
    }
    pub fn client_report(&self, args: &Value) -> Result<Value> {
        let e = self.all("engagement")?.remove(0);
        let draft = args["include_drafts"].as_bool() == Some(true);
        let escape = |v: &str| {
            v.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#39;")
        };
        let mut html=format!("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src 'none'; base-uri 'none'; form-action 'none'\"><title>{}</title><style>body{{font:15px/1.6 system-ui,sans-serif;color:#183044;max-width:1050px;margin:50px auto;padding:0 28px}}header{{border-bottom:4px solid #187e7b;padding-bottom:30px}}h1{{font-size:40px;line-height:1.15}}h2{{margin-top:38px}}h3{{margin:20px 0 5px}}.muted{{color:#64748b}}.text{{white-space:pre-wrap;overflow-wrap:anywhere}}table{{border-collapse:collapse;width:100%;font-size:13px}}td,th{{border-bottom:1px solid #d9e2e8;padding:10px;text-align:left;vertical-align:top}}article{{border-top:1px solid #d9e2e8;margin-top:32px;padding-top:16px}}code{{font-size:11px;overflow-wrap:anywhere}}@media print{{body{{margin:0;max-width:none}}article{{break-before:page}}h2,h3{{break-after:avoid}}tr{{break-inside:avoid}}}}</style><header><p>NEXUS · ASSESSMENT REPORT</p><h1>{}</h1><p>{} · {} · {}</p><p class=\"muted\">Generated {} · {} findings included. Credential inventory and secret values are excluded; review analyst text for sensitive content before sharing.</p></header>",escape(s(&e,"name")),escape(s(&e,"name")),escape(s(&e,"client")),escape(s(&e,"kind")),escape(s(&e,"status")),escape(&now()),if draft{"All including drafts"}else{"Non-draft"});
        html.push_str(&format!("<h2>Engagement context</h2><div class=\"text\">{}</div><h2>Scope and exclusions</h2><table><tr><th>Type</th><th>Rule</th><th>Notes</th></tr>",escape(s(&e,"description"))));
        for rule in self.all("scope_rules")? {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td class=\"text\">{}</td></tr>",
                if rule["excluded"].as_i64() == Some(1) {
                    "Excluded"
                } else {
                    "Included"
                },
                escape(s(&rule, "rule")),
                escape(s(&rule, "notes"))
            ));
        }
        html.push_str("</table><h2>Assessment coverage</h2><p>These are recorded analyst outcomes. Unstarted, blocked or excluded work is not counted as validated.</p><table><tr><th>Domain</th><th>State</th><th>Checks</th></tr>");
        for r in self.query("SELECT area,state,count(*) AS n FROM assessment_checks GROUP BY area,state ORDER BY area,state",&[])?{html.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td></tr>",escape(s(&r,"area")),escape(s(&r,"state")),r["n"]));}
        html.push_str("</table><h2>Findings summary</h2><table><tr><th>Severity</th><th>Finding</th><th>Status</th><th>Asset</th></tr>");
        let mut findings=self.query("SELECT * FROM findings WHERE (?=1 OR status!='Draft') ORDER BY CASE severity WHEN 'Critical' THEN 0 WHEN 'High' THEN 1 WHEN 'Medium' THEN 2 WHEN 'Low' THEN 3 ELSE 4 END,name",&[json!(i64::from(draft))])?;
        self.decorate(&mut findings)?;
        for f in &findings {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape(s(f, "severity")),
                escape(s(f, "name")),
                escape(s(f, "status")),
                escape(s(&f["reference_labels"], "asset_id"))
            ));
        }
        html.push_str("</table>");
        if findings.is_empty() {
            html.push_str("<p>No findings match this report selection. This does not establish the absence of vulnerabilities.</p>");
        }
        for f in findings {
            html.push_str(&format!(
                "<article><p>{} · {}</p><h2>{}</h2><p>Asset: {}</p><code>Record {}</code>",
                escape(s(&f, "severity")),
                escape(s(&f, "status")),
                escape(s(&f, "name")),
                escape(s(&f["reference_labels"], "asset_id")),
                escape(s(&f, "id"))
            ));
            for (key, title) in [
                ("description", "Description"),
                ("impact", "Impact"),
                ("reproduction", "Validation and reproduction"),
                ("remediation", "Remediation"),
                ("refs", "References"),
                ("cve", "CVE"),
                ("cwe", "CWE"),
            ] {
                if !s(&f, key).is_empty() {
                    html.push_str(&format!(
                        "<h3>{title}</h3><div class=\"text\">{}</div>",
                        escape(s(&f, key))
                    ));
                }
            }
            html.push_str("<h3>Evidence references</h3><ul>");
            for evidence in self.query("SELECT DISTINCT e.id,e.name,e.sha256 FROM evidence e WHERE e.entity_id=? OR e.id IN (SELECT evidence_id FROM assessment_checks WHERE finding_id=?) ORDER BY e.name",&[f["id"].clone(),f["id"].clone()])?{html.push_str(&format!("<li>{}<br><code>{} · SHA-256 {}</code></li>",escape(s(&evidence,"name")),escape(s(&evidence,"id")),escape(s(&evidence,"sha256"))));}
            html.push_str("</ul></article>");
        }
        html.push_str("<h2>Unfinished and blocked work</h2><table><tr><th>Check</th><th>Owner</th><th>State</th><th>Reason / result</th></tr>");
        for r in self.query("SELECT name,owner,state,result FROM assessment_checks WHERE state NOT IN ('Passed','Failed') ORDER BY area,name",&[])?{html.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td><td class=\"text\">{}</td></tr>",escape(s(&r,"name")),escape(s(&r,"owner")),escape(s(&r,"state")),escape(s(&r,"result"))));}
        html.push_str("</table><footer><p class=\"muted\">Produced locally by NEXUS. Attachment contents are distributed separately in verified workspace bundles.</p></footer></html>");
        let path = self
            .path
            .join("exports")
            .join(format!("NEXUS-client-report-{}.html", id()));
        fs::write(&path, &html).map_err(|e| e.to_string())?;
        self.event(
            DomainEvent::ReportExported,
            None,
            "Client HTML report exported; credential inventory excluded",
            "Analyst",
        )?;
        Ok(json!({"path":path,"text":html}))
    }
}
