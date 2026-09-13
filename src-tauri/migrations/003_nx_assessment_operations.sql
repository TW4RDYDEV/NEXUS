-- NEXUS — TWARDY.exe / TW4RDYDEV
-- Professional assessment plans and indexed, confirmed-access exploration.
CREATE TABLE assessment_checks (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, area TEXT NOT NULL DEFAULT 'Custom',
    template_id TEXT NOT NULL DEFAULT '', objective TEXT NOT NULL DEFAULT '',
    reference_url TEXT NOT NULL DEFAULT '', asset_id TEXT REFERENCES assets(id),
    owner TEXT NOT NULL DEFAULT '', priority TEXT NOT NULL DEFAULT 'Normal',
    state TEXT NOT NULL DEFAULT 'Not started' CHECK(state IN ('Not started','In progress','Blocked','Passed','Failed','Not applicable')),
    result TEXT NOT NULL DEFAULT '', evidence_id TEXT REFERENCES evidence(id), finding_id TEXT REFERENCES findings(id),
    due_date TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX assessment_template_target ON assessment_checks(template_id,coalesce(asset_id,'')) WHERE template_id!='';
CREATE INDEX assessment_state_area ON assessment_checks(state,area,priority);
CREATE INDEX assets_name_id ON assets(name COLLATE NOCASE,id);
CREATE INDEX services_asset_status ON services(asset_id,status,port);
CREATE INDEX credentials_name_id ON credentials(username COLLATE NOCASE,id);
CREATE INDEX credential_tests_asset ON credential_tests(asset_id,credential_id,service_id);
CREATE INDEX credential_tests_service ON credential_tests(service_id,credential_id,created_at DESC);
CREATE INDEX sessions_active_asset ON sessions(asset_id,state);
CREATE INDEX credentials_source ON credentials(source_id);
CREATE INDEX findings_status_asset ON findings(status,asset_id);
CREATE INDEX relationships_kind_source ON relationships(kind,source_id,target_id);

CREATE VIEW nx_confirmed_access_edges AS
SELECT a.id AS source,s.id AS target,'NX_EXPOSES' AS kind FROM services s JOIN assets a ON a.id=s.asset_id AND a.archived=0
UNION ALL SELECT a.id,c.id,'NX_DISCOVERED_FROM' FROM credentials c JOIN assets a ON a.id=c.source_id AND a.archived=0
UNION ALL SELECT a.id,s.id,'NX_HAS_SESSION' FROM sessions s JOIN assets a ON a.id=s.asset_id AND a.archived=0 WHERE s.state='Active'
UNION ALL SELECT 'operator',a.id,'NX_HAS_SESSION' FROM assets a WHERE a.archived=0 AND EXISTS(SELECT 1 FROM sessions s WHERE s.asset_id=a.id AND s.state='Active')
UNION ALL SELECT c.id,s.id,'NX_HAS_SESSION' FROM sessions s JOIN credentials c ON c.id=s.credential_id JOIN assets a ON a.id=s.asset_id AND a.archived=0 WHERE s.state='Active'
UNION ALL SELECT t.credential_id,a.id,'NX_AUTHENTICATES_TO' FROM credential_tests t JOIN assets a ON a.id=t.asset_id AND a.archived=0 WHERE t.result='Valid' AND t.rowid=(SELECT x.rowid FROM credential_tests x WHERE x.credential_id=t.credential_id AND x.service_id=t.service_id ORDER BY x.created_at DESC,x.rowid DESC LIMIT 1)
UNION ALL SELECT t.credential_id,s.id,'NX_AUTHENTICATES_TO' FROM credential_tests t JOIN services s ON s.id=t.service_id JOIN assets a ON a.id=s.asset_id AND a.archived=0 WHERE t.result='Valid' AND t.rowid=(SELECT x.rowid FROM credential_tests x WHERE x.credential_id=t.credential_id AND x.service_id=t.service_id ORDER BY x.created_at DESC,x.rowid DESC LIMIT 1);
