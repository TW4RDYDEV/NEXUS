PRAGMA foreign_keys=ON;
CREATE TABLE IF NOT EXISTS engagement(id TEXT PRIMARY KEY,name TEXT NOT NULL,description TEXT NOT NULL DEFAULT '',kind TEXT NOT NULL DEFAULT 'Lab / CTF',status TEXT NOT NULL DEFAULT 'Active',created_at TEXT NOT NULL,start_date TEXT NOT NULL,client TEXT NOT NULL DEFAULT '',demo INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS scope_rules(id TEXT PRIMARY KEY,rule TEXT NOT NULL,excluded INTEGER NOT NULL DEFAULT 0,UNIQUE(rule,excluded));
CREATE TABLE IF NOT EXISTS assets(id TEXT PRIMARY KEY,name TEXT NOT NULL,kind TEXT NOT NULL DEFAULT 'Host',ip TEXT NOT NULL DEFAULT '',hostname TEXT NOT NULL DEFAULT '',os TEXT NOT NULL DEFAULT '',status TEXT NOT NULL DEFAULT 'Active',direct INTEGER NOT NULL DEFAULT 0,access TEXT NOT NULL DEFAULT 'None',tags TEXT NOT NULL DEFAULT '',notes TEXT NOT NULL DEFAULT '',first_seen TEXT NOT NULL,last_seen TEXT NOT NULL,archived INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS assets_ip ON assets(ip);
CREATE INDEX IF NOT EXISTS assets_hostname ON assets(hostname);
CREATE TABLE IF NOT EXISTS asset_aliases(id TEXT PRIMARY KEY,asset_id TEXT NOT NULL REFERENCES assets(id),alias TEXT NOT NULL,UNIQUE(asset_id,alias));
CREATE INDEX IF NOT EXISTS aliases_value ON asset_aliases(alias);
CREATE TABLE IF NOT EXISTS services(id TEXT PRIMARY KEY,asset_id TEXT NOT NULL REFERENCES assets(id),name TEXT NOT NULL DEFAULT '',protocol TEXT NOT NULL DEFAULT 'tcp',port INTEGER NOT NULL CHECK(port BETWEEN 1 AND 65535),product TEXT NOT NULL DEFAULT '',version TEXT NOT NULL DEFAULT '',banner TEXT NOT NULL DEFAULT '',tls INTEGER NOT NULL DEFAULT 0,url TEXT NOT NULL DEFAULT '',title TEXT NOT NULL DEFAULT '',status TEXT NOT NULL DEFAULT 'open',notes TEXT NOT NULL DEFAULT '',first_seen TEXT NOT NULL,last_seen TEXT NOT NULL,UNIQUE(asset_id,protocol,port));
CREATE TABLE IF NOT EXISTS credentials(id TEXT PRIMARY KEY,name TEXT NOT NULL,username TEXT NOT NULL,context TEXT NOT NULL DEFAULT '',kind TEXT NOT NULL DEFAULT 'Password',ciphertext TEXT NOT NULL,source_id TEXT REFERENCES assets(id),source TEXT NOT NULL DEFAULT '',notes TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS credential_tests(id TEXT PRIMARY KEY,credential_id TEXT NOT NULL REFERENCES credentials(id),asset_id TEXT NOT NULL REFERENCES assets(id),service_id TEXT NOT NULL REFERENCES services(id),result TEXT NOT NULL CHECK(result IN ('Valid','Invalid','Unknown')),privilege TEXT NOT NULL DEFAULT 'Unknown',source TEXT NOT NULL DEFAULT 'Manual',evidence_id TEXT REFERENCES evidence(id),created_at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS tests_pair ON credential_tests(credential_id,service_id,created_at);
CREATE TABLE IF NOT EXISTS sessions(id TEXT PRIMARY KEY,name TEXT NOT NULL,asset_id TEXT NOT NULL REFERENCES assets(id),username TEXT NOT NULL,privilege TEXT NOT NULL DEFAULT 'Unknown',kind TEXT NOT NULL DEFAULT 'SSH',source TEXT NOT NULL DEFAULT '',state TEXT NOT NULL DEFAULT 'Active',credential_id TEXT REFERENCES credentials(id),pivot_capable INTEGER NOT NULL DEFAULT 0,notes TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL,last_active TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS pivots(id TEXT PRIMARY KEY,name TEXT NOT NULL,session_id TEXT NOT NULL REFERENCES sessions(id),asset_id TEXT NOT NULL REFERENCES assets(id),network TEXT NOT NULL,kind TEXT NOT NULL DEFAULT 'SOCKS',status TEXT NOT NULL DEFAULT 'Active',notes TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS findings(id TEXT PRIMARY KEY,name TEXT NOT NULL,severity TEXT NOT NULL DEFAULT 'Info',asset_id TEXT NOT NULL REFERENCES assets(id),service_id TEXT REFERENCES services(id),description TEXT NOT NULL DEFAULT '',impact TEXT NOT NULL DEFAULT '',reproduction TEXT NOT NULL DEFAULT '',remediation TEXT NOT NULL DEFAULT '',refs TEXT NOT NULL DEFAULT '',cve TEXT NOT NULL DEFAULT '',cwe TEXT NOT NULL DEFAULT '',cvss TEXT NOT NULL DEFAULT '',status TEXT NOT NULL DEFAULT 'Draft',template_id TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS findings_asset ON findings(asset_id);
CREATE TABLE IF NOT EXISTS evidence(id TEXT PRIMARY KEY,name TEXT NOT NULL,kind TEXT NOT NULL DEFAULT 'Text note',entity_id TEXT NOT NULL,body TEXT NOT NULL DEFAULT '',file_name TEXT NOT NULL DEFAULT '',sha256 TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS evidence_entity ON evidence(entity_id);
CREATE TABLE IF NOT EXISTS relationships(id TEXT PRIMARY KEY,source_id TEXT NOT NULL,target_id TEXT NOT NULL,kind TEXT NOT NULL,notes TEXT NOT NULL DEFAULT '',created_at TEXT NOT NULL,UNIQUE(source_id,target_id,kind));
CREATE INDEX IF NOT EXISTS rel_source ON relationships(source_id);
CREATE INDEX IF NOT EXISTS rel_target ON relationships(target_id);
CREATE TABLE IF NOT EXISTS imports(id TEXT PRIMARY KEY,name TEXT NOT NULL,tool TEXT NOT NULL,file_name TEXT NOT NULL,sha256 TEXT NOT NULL,created_at TEXT NOT NULL,entities INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS provenance(id TEXT PRIMARY KEY,entity_id TEXT NOT NULL,field TEXT NOT NULL,value TEXT NOT NULL,source TEXT NOT NULL,import_id TEXT REFERENCES imports(id),confidence TEXT NOT NULL,conflict INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS provenance_entity ON provenance(entity_id);
CREATE TABLE IF NOT EXISTS timeline_events(id TEXT PRIMARY KEY,kind TEXT NOT NULL,entity_id TEXT,name TEXT NOT NULL,source TEXT NOT NULL DEFAULT 'NEXUS',created_at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS timeline_time ON timeline_events(created_at);
CREATE TABLE IF NOT EXISTS coverage_definitions(id TEXT PRIMARY KEY,service_family TEXT NOT NULL,name TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS coverage_results(id TEXT PRIMARY KEY,service_id TEXT NOT NULL REFERENCES services(id),definition_id TEXT NOT NULL REFERENCES coverage_definitions(id),state TEXT NOT NULL CHECK(state IN ('Complete','Partial','Untested','Not Applicable')),notes TEXT NOT NULL DEFAULT '',source TEXT NOT NULL DEFAULT 'Manual',created_at TEXT NOT NULL,UNIQUE(service_id,definition_id));
CREATE TABLE IF NOT EXISTS snapshots(id TEXT PRIMARY KEY,name TEXT NOT NULL,created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS snapshot_entities(snapshot_id TEXT NOT NULL REFERENCES snapshots(id),entity_id TEXT NOT NULL,kind TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(snapshot_id,entity_id));
CREATE TABLE IF NOT EXISTS app_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS backups(id TEXT PRIMARY KEY,name TEXT NOT NULL,created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS undo_history(id INTEGER PRIMARY KEY AUTOINCREMENT,table_name TEXT NOT NULL,entity_id TEXT NOT NULL,before_data TEXT,created_at TEXT NOT NULL);
PRAGMA user_version=1;
CREATE VIEW IF NOT EXISTS coverage_view AS
SELECT coalesce(r.id,'') AS id,s.id AS service_id,s.asset_id,a.name AS asset_name,
       s.name || ' :' || s.port AS service,d.id AS definition_id,d.name,
       coalesce(r.state,'Untested') AS state,coalesce(r.notes,'') AS notes,coalesce(r.source,'') AS source
FROM services s JOIN assets a ON a.id=s.asset_id
JOIN coverage_definitions d ON d.service_family = CASE
 WHEN s.port IN (445,139) THEN 'SMB' WHEN s.port=22 THEN 'SSH'
 WHEN s.port IN (80,443,8000,8080,8443) OR lower(s.name) LIKE '%http%' THEN 'HTTP' ELSE 'Other' END
LEFT JOIN coverage_results r ON r.service_id=s.id AND r.definition_id=d.id
WHERE s.status='open' AND a.archived=0;
CREATE INDEX IF NOT EXISTS sessions_asset ON sessions(asset_id);
CREATE INDEX IF NOT EXISTS pivots_session ON pivots(session_id);
CREATE INDEX IF NOT EXISTS provenance_source_entity ON provenance(source,entity_id);
