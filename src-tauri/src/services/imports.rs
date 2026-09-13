use crate::domain::DomainEvent;
use crate::models::provenance::{NormalizationRecord, ObservationConfidence, ObservationSource};
use crate::{
    db::Store,
    models::{id, s, DiscoveryRecord, Result},
    parsers,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
impl Store {
    fn resolve_host(&self, h: &DiscoveryRecord) -> Result<Option<Value>> {
        if !h.ip.is_empty() {
            let matches = self.query(
                "SELECT DISTINCT a.* FROM assets a LEFT JOIN asset_aliases x ON x.asset_id=a.id WHERE a.kind='Host' AND (a.ip=? OR x.alias=?)",
                &[h.ip.clone().into(), h.ip.clone().into()],
            )?;
            if matches.len() > 1 {
                return Err(format!(
                    "Multiple assets share {}. Resolve the identity conflict before importing.",
                    h.ip
                ));
            }
            if let Some(a) = matches.into_iter().next() {
                return Ok(Some(a));
            }
        }
        if !h.hostname.is_empty() {
            let matches=self.query("SELECT DISTINCT a.* FROM assets a LEFT JOIN asset_aliases x ON x.asset_id=a.id WHERE a.kind='Host' AND (lower(a.hostname)=? OR lower(x.alias)=?)",&[h.hostname.to_ascii_lowercase().into(),h.hostname.to_ascii_lowercase().into()])?;
            let matches = matches
                .into_iter()
                .filter(|a| h.ip.is_empty() || s(a, "ip").is_empty() || s(a, "ip") == h.ip)
                .collect::<Vec<_>>();
            if matches.len() > 1 {
                return Err(format!(
                    "Ambiguous hostname {}. Resolve aliases before importing.",
                    h.hostname
                ));
            }
            return Ok(matches.into_iter().next());
        }
        Ok(None)
    }
    pub fn import_preview(&mut self, tool: &str, text: &str) -> Result<Value> {
        let parsed = parsers::parse(tool, text)?;
        let mut hosts = std::collections::HashSet::new();
        let mut services = std::collections::HashSet::new();
        let mut existing = 0;
        let mut conflicts = vec![];
        let mut updates = 0;
        let mut review = vec![];
        for h in &parsed.hosts {
            let key = if h.ip.is_empty() {
                h.hostname.clone()
            } else {
                h.ip.clone()
            };
            if hosts.insert(key.clone()) {
                let found = self.resolve_host(h)?;
                if let Some(a) = &found {
                    existing += 1;
                    for (field, incoming) in
                        [("os", h.os.as_str()), ("hostname", h.hostname.as_str())]
                    {
                        if !incoming.is_empty()
                            && !s(a, field).is_empty()
                            && s(a, field) != incoming
                        {
                            conflicts.push(json!({"key":format!("{}:{field}",s(a,"id")),"entity":a["name"],"field":field,"current":a[field],"incoming":incoming}))
                        }
                    }
                }
                review.push(json!({"target":key,"action":if found.is_some(){"Merge observations"}else{"Create host"},"services":h.services.len()}));
            }
            for svc in &h.services {
                services.insert(format!("{key}:{}:{}", svc.protocol, svc.port));
                if let Some(a) = self.resolve_host(h)? {
                    if let Some(old) = self
                        .query(
                            "SELECT * FROM services WHERE asset_id=? AND protocol=? AND port=?",
                            &[
                                s(&a, "id").into(),
                                svc.protocol.clone().into(),
                                i64::from(svc.port).into(),
                            ],
                        )?
                        .first()
                    {
                        updates += 1;
                        for (field, incoming) in [
                            ("version", svc.version.as_str()),
                            ("product", svc.product.as_str()),
                            ("title", svc.title.as_str()),
                            ("status", svc.status.as_str()),
                        ] {
                            if !incoming.is_empty()
                                && !s(old, field).is_empty()
                                && s(old, field) != incoming
                            {
                                conflicts.push(json!({"key":format!("{}:{field}",s(old,"id")),"entity":format!("{} :{}",s(&a,"name"),svc.port),"field":field,"current":old[field],"incoming":incoming}));
                            }
                        }
                    }
                }
            }
        }
        Ok(
            json!({"hosts":hosts.len(),"services":services.len(),"existing":existing,"new_entities":hosts.len()-existing,"updates":updates,"conflicts":conflicts,"warnings":parsed.warnings,"review":review,"requires_vault":tool=="NetExec"}),
        )
    }
    pub fn import_commit(
        &mut self,
        tool: &str,
        name: &str,
        text: &str,
        accept: &[String],
    ) -> Result<Value> {
        let preview = self.import_preview(tool, text)?;
        let parsed = parsers::parse(tool, text)?;
        if tool == "NetExec" && !self.vault.unlocked() {
            return Err("Unlock the vault to retain NetExec source securely".into());
        }
        self.backup("Backup before import")?;
        let import_id = id();
        let raw_name = format!(
            "{import_id}.{}",
            if tool == "NetExec" {
                "encrypted"
            } else {
                "source"
            }
        );
        let raw = if tool == "NetExec" {
            self.vault.encrypt(text, &import_id)?
        } else {
            text.to_string()
        };
        let path = self.path.join("imports").join(&raw_name);
        fs::write(&path, &raw).map_err(|e| e.to_string())?;
        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| e.to_string())?;
        let result = (|| {
            self.insert("imports",&json!({"id":import_id,"name":name,"tool":tool,"file_name":raw_name,"sha256":format!("{:x}",Sha256::digest(text.as_bytes())),"entities":preview["hosts"]}))?;
            for h in &parsed.hosts {
                let asset_id = if let Some(a) = self.resolve_host(h)? {
                    let aid = s(&a, "id").to_string();
                    self.observe("assets", &aid, "ip", &h.ip, tool, &import_id, accept)?;
                    self.observe(
                        "assets",
                        &aid,
                        "hostname",
                        &h.hostname,
                        tool,
                        &import_id,
                        accept,
                    )?;
                    self.observe("assets", &aid, "os", &h.os, tool, &import_id, accept)?;
                    self.update("assets", &aid, &json!({"last_seen":crate::models::now()}))?;
                    aid
                } else {
                    let name = if !h.hostname.is_empty() {
                        h.hostname
                            .split('.')
                            .next()
                            .unwrap_or(&h.hostname)
                            .to_uppercase()
                    } else {
                        h.ip.clone()
                    };
                    let aid=self.insert("assets",&json!({"name":name,"ip":h.ip,"hostname":h.hostname,"os":h.os,"kind":"Host"}))?;
                    self.event(
                        DomainEvent::AssetDiscovered,
                        Some(&aid),
                        &format!("Discovered {name}"),
                        tool,
                    )?;
                    for (field, value) in [("ip", &h.ip), ("hostname", &h.hostname), ("os", &h.os)]
                    {
                        if !value.is_empty() {
                            self.provenance(NormalizationRecord {
                                entity_id: &aid,
                                field,
                                value,
                                source: ObservationSource::Tool(tool),
                                import_id: Some(&import_id),
                                confidence: ObservationConfidence::High,
                                conflict: false,
                            })?
                        }
                    }
                    aid
                };
                if !h.hostname.is_empty() {
                    self.conn
                        .execute(
                            "INSERT OR IGNORE INTO asset_aliases(id,asset_id,alias) VALUES(?,?,?)",
                            rusqlite::params![id(), asset_id, h.hostname.to_ascii_lowercase()],
                        )
                        .map_err(|e| e.to_string())?;
                }
                let mut service_ids = vec![];
                for svc in &h.services {
                    let existing = self.query(
                        "SELECT * FROM services WHERE asset_id=? AND protocol=? AND port=?",
                        &[
                            asset_id.clone().into(),
                            svc.protocol.clone().into(),
                            i64::from(svc.port).into(),
                        ],
                    )?;
                    let sid = if let Some(old) = existing.first() {
                        let sid = s(old, "id");
                        for (field, value) in [
                            ("name", &svc.name),
                            ("product", &svc.product),
                            ("version", &svc.version),
                            ("status", &svc.status),
                            ("url", &svc.url),
                            ("title", &svc.title),
                            ("banner", &svc.banner),
                        ] {
                            self.observe("services", sid, field, value, tool, &import_id, accept)?;
                        }
                        self.update("services", sid, &json!({"last_seen":crate::models::now()}))?;
                        sid.to_string()
                    } else {
                        let sid=self.insert("services",&json!({"asset_id":asset_id,"name":svc.name,"protocol":svc.protocol,"port":svc.port,"product":svc.product,"version":svc.version,"status":svc.status,"tls":i64::from(svc.tls),"url":svc.url,"title":svc.title,"banner":svc.banner}))?;
                        self.provenance(NormalizationRecord {
                            entity_id: &sid,
                            field: "observation",
                            value: &serde_json::to_string(svc).map_err(|e| e.to_string())?,
                            source: ObservationSource::Tool(tool),
                            import_id: Some(&import_id),
                            confidence: ObservationConfidence::High,
                            conflict: false,
                        })?;
                        self.event(
                            DomainEvent::ServiceObserved,
                            Some(&sid),
                            &format!("Service discovered: {} :{}", svc.name, svc.port),
                            tool,
                        )?;
                        sid
                    };
                    service_ids.push(sid.clone());
                    let family = crate::core::intelligence::family(&self.one("services", &sid)?);
                    for (label, observed) in [
                        ("Service identified", !svc.name.is_empty()),
                        ("Version identified", !svc.version.is_empty()),
                        ("Technology fingerprinting", !svc.product.is_empty()),
                        ("TLS observation", svc.tls),
                    ] {
                        if observed {
                            let def=self.query("SELECT id FROM coverage_definitions WHERE service_family=? AND name=?",&[family.into(),label.into()])?;
                            if let Some(def) = def.first() {
                                self.conn.execute("INSERT OR IGNORE INTO coverage_results(id,service_id,definition_id,state,source,created_at) VALUES(?,?,?,'Complete',?,?)",rusqlite::params![id(),sid,s(def,"id"),tool,crate::models::now()]).map_err(|e|e.to_string())?;
                            }
                        }
                    }
                }
                for finding in &h.findings {
                    let existing=self.query("SELECT id FROM findings WHERE asset_id=? AND template_id=? AND reproduction=?",&[asset_id.clone().into(),s(finding,"template_id").into(),s(finding,"reproduction").into()])?;
                    if existing.is_empty() {
                        let mut f = finding.clone();
                        f.as_object_mut().unwrap().remove("raw");
                        f["asset_id"] = json!(asset_id);
                        let observed_port = f["observed_port"].as_i64();
                        let observed_protocol = s(&f, "observed_protocol").to_string();
                        f.as_object_mut().unwrap().remove("observed_port");
                        f.as_object_mut().unwrap().remove("observed_protocol");
                        f["status"] = json!("Draft");
                        f["service_id"] = if let Some(port) = observed_port {
                            self.query("SELECT id FROM services WHERE asset_id=? AND port=? AND protocol=?",&[json!(asset_id),json!(port),json!(observed_protocol)])?.first().map(|s|s["id"].clone()).unwrap_or(Value::Null)
                        } else {
                            json!(service_ids.first())
                        };
                        let fid = self.insert("findings", &f)?;
                        let evidence=self.insert("evidence",&json!({"name":format!("Imported {tool} observation"),"kind":"Terminal output","entity_id":fid,"body":finding["raw"].to_string()}))?;
                        self.provenance(NormalizationRecord {
                            entity_id: &fid,
                            field: "finding",
                            value: s(finding, "name"),
                            source: ObservationSource::Tool(tool),
                            import_id: Some(&import_id),
                            confidence: ObservationConfidence::Medium,
                            conflict: false,
                        })?;
                        self.event(
                            DomainEvent::FindingRecorded,
                            Some(&fid),
                            "Finding imported for analyst validation",
                            tool,
                        )?;
                        let _ = evidence;
                    }
                }
                for auth in &h.auth {
                    let candidates = self.query(
                        "SELECT id,ciphertext FROM credentials WHERE username=? AND context=?",
                        &[s(auth, "username").into(), s(auth, "context").into()],
                    )?;
                    let mut existing = Vec::new();
                    for candidate in candidates {
                        let plain = zeroize::Zeroizing::new(
                            self.vault
                                .decrypt(s(&candidate, "ciphertext"), s(&candidate, "id"))?,
                        );
                        if plain.as_str() == s(auth, "secret") {
                            existing.push(candidate);
                            break;
                        }
                    }
                    let cid = if let Some(c) = existing.first() {
                        s(c, "id").to_string()
                    } else {
                        let cid = id();
                        let encrypted = self.vault.encrypt(s(auth, "secret"), &cid)?;
                        self.insert("credentials",&json!({"id":cid,"name":s(auth,"username"),"username":s(auth,"username"),"context":s(auth,"context"),"kind":"Password","ciphertext":encrypted,"source":format!("NetExec import {import_id}")}))?;
                        self.event(
                            DomainEvent::CredentialRecorded,
                            Some(&cid),
                            "Credential recorded from import",
                            tool,
                        )?;
                        cid
                    };
                    if let Some(sid) = service_ids.first() {
                        self.insert("credential_tests",&json!({"credential_id":cid,"asset_id":asset_id,"service_id":sid,"result":auth["result"],"privilege":auth["privilege"],"source":format!("NetExec import {import_id}")}))?;
                        self.event(
                            if s(auth, "result") == "Valid" {
                                DomainEvent::AuthenticationConfirmed
                            } else {
                                DomainEvent::AuthenticationRecorded
                            },
                            Some(&asset_id),
                            &format!("{} authentication recorded", s(auth, "result")),
                            tool,
                        )?;
                    }
                }
            }
            self.event(
                DomainEvent::ImportCompleted,
                Some(&import_id),
                &format!(
                    "{name} imported · {} hosts · {} services",
                    preview["hosts"], preview["services"]
                ),
                tool,
            )?;
            Ok(json!({"id":import_id,"preview":preview}))
        })();
        let result = self.finish(result);
        if result.is_err() {
            let _ = fs::remove_file(path);
        }
        result
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "The adapter passes entity identity and immutable source context explicitly"
    )]
    fn observe(
        &self,
        table: &str,
        eid: &str,
        field: &str,
        value: &str,
        tool: &str,
        import_id: &str,
        accept: &[String],
    ) -> Result<()> {
        if value.is_empty() {
            return Ok(());
        }
        let old = self.one(table, eid)?;
        let previous = s(&old, field);
        let conflict = !previous.is_empty() && previous != value;
        let accepted = accept.contains(&format!("{eid}:{field}"));
        self.provenance(NormalizationRecord {
            entity_id: eid,
            field,
            value,
            source: ObservationSource::Tool(tool),
            import_id: Some(import_id),
            confidence: ObservationConfidence::High,
            conflict: conflict && !accepted,
        })?;
        if previous.is_empty() || previous == value || accepted {
            let mut data = json!({});
            data[field] = json!(value);
            self.update(table, eid, &data)?;
            if table == "assets" && previous != value {
                self.event(
                    DomainEvent::AssetEnriched,
                    Some(eid),
                    &format!("Asset {field} enriched"),
                    tool,
                )?;
            }
        }
        Ok(())
    }
}
