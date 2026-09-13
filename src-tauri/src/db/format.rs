//! Versioned format upgrades. No author identity is used as a tamper check.
use crate::{
    domain::{DomainEvent, RelationshipKind},
    errors::ErrorCode,
    identity::*,
    models::{now, Result},
};
use rusqlite::{params, Connection};
use serde_json::Value;
use std::{fs, path::Path};

pub const MIGRATIONS: &[(i64, &str)] = &[
    (1, "001_nx_core_engagement"),
    (2, "002_nx_format_identity"),
    (3, "003_nx_assessment_operations"),
];

pub fn initialize(conn: &Connection, path: &Path) -> Result<()> {
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let application_id: i32 = conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if version > NEXUS_SCHEMA_VERSION
        || (application_id != 0 && application_id != NEXUS_APPLICATION_ID)
    {
        return Err(ErrorCode::DatabaseFormatUnsupported
            .message("This database uses a different or newer file format"));
    }
    let table_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if table_count > 0 {
        // Recognize the original schema before claiming an untagged SQLite file.
        for table in [
            "engagement",
            "assets",
            "services",
            "credentials",
            "relationships",
            "provenance",
            "snapshots",
            "snapshot_entities",
            "timeline_events",
            "undo_history",
        ] {
            let exists: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?",
                    [table],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if exists != 1 || version == 0 {
                return Err(ErrorCode::DatabaseFormatUnsupported
                    .message("This file is not a supported NEXUS engagement database"));
            }
        }
    } else if version != 0 {
        return Err(ErrorCode::DatabaseFormatUnsupported.message("The database schema is missing"));
    }
    validate_workspace(path)?;
    if version < NEXUS_SCHEMA_VERSION {
        if version > 0 {
            // Preserve a recoverable copy of the original schema before upgrading.
            let backup = path
                .join("backups")
                .join(format!("schema-{version}-{}.db", crate::models::id()));
            conn.backup("main", &backup, None)
                .map_err(|e| e.to_string())?;
        }
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| e.to_string())?;
        let result = migrate(conn, version);
        match result {
            Ok(()) => conn.execute_batch("COMMIT").map_err(|e| e.to_string())?,
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(e);
            }
        }
    }
    write_workspace(path)
}

fn migrate(conn: &Connection, version: i64) -> Result<()> {
    if version == 0 {
        conn.execute_batch(include_str!("../../migrations/001_nx_core_engagement.sql"))
            .map_err(|e| e.to_string())?;
    }
    if version < 2 {
        conn.execute_batch(include_str!("../../migrations/002_nx_format_identity.sql"))
            .map_err(|e| e.to_string())?;
        for kind in RelationshipKind::ALL {
            let legacy = kind.as_str().strip_prefix("NX_").unwrap();
            conn.execute(
                "UPDATE relationships SET kind=? WHERE kind=?",
                params![kind.as_str(), legacy],
            )
            .map_err(|e| e.to_string())?;
            // Normalize previous snapshots and undo records too, so a format upgrade
            // produces no false ReconDelta changes and undo cannot restore old tokens.
            conn.execute("UPDATE snapshot_entities SET data=json_set(data,'$.kind',?) WHERE kind='relationships' AND json_extract(data,'$.kind')=?", params![kind.as_str(), legacy]).map_err(|e| e.to_string())?;
            conn.execute("UPDATE undo_history SET before_data=json_set(before_data,'$.kind',?) WHERE table_name='relationships' AND before_data IS NOT NULL AND json_extract(before_data,'$.kind')=?", params![kind.as_str(), legacy]).map_err(|e| e.to_string())?;
        }
        let kinds: Vec<String> = conn
            .prepare("SELECT DISTINCT kind FROM timeline_events")
            .map_err(|e| e.to_string())?
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| e.to_string())?;
        for old in kinds {
            if let Some(event) = DomainEvent::legacy(&old) {
                conn.execute(
                    "UPDATE timeline_events SET kind=? WHERE kind=?",
                    params![event.as_str(), old],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }
    if version < 3 {
        conn.execute_batch(include_str!(
            "../../migrations/003_nx_assessment_operations.sql"
        ))
        .map_err(|e| e.to_string())?;
    }
    for (key, value) in workspace_metadata().as_object().unwrap() {
        let text = value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string());
        conn.execute(
            "INSERT INTO nx_format_metadata(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, text],
        )
        .map_err(|e| e.to_string())?;
    }
    for (version, name) in MIGRATIONS {
        conn.execute(
            "INSERT OR IGNORE INTO nx_migrations(version,name,recorded_at) VALUES(?,?,?)",
            params![version, name, now()],
        )
        .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "UPDATE snapshots SET format_metadata=?",
        [workspace_metadata().to_string()],
    )
    .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "application_id", NEXUS_APPLICATION_ID)
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "user_version", NEXUS_SCHEMA_VERSION)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn validate_workspace(path: &Path) -> Result<()> {
    let file = path.join("workspace.json");
    if !file.exists() {
        return Ok(());
    }
    let metadata: Value = serde_json::from_str(
        &fs::read_to_string(file).map_err(|e| e.to_string())?,
    )
    .map_err(|_| {
        ErrorCode::DatabaseFormatUnsupported.message("Workspace metadata is not valid JSON")
    })?;
    if metadata["format"].as_str() != Some(NEXUS_FORMAT_ID)
        || metadata["format_version"].as_i64() != Some(NEXUS_FORMAT_VERSION)
        || metadata["schema_version"]
            .as_i64()
            .is_some_and(|v| v > NEXUS_SCHEMA_VERSION)
    {
        return Err(ErrorCode::DatabaseFormatUnsupported
            .message("This workspace format requires a compatible version of NEXUS"));
    }
    Ok(())
}

fn write_workspace(path: &Path) -> Result<()> {
    let file = path.join("workspace.json");
    let mut metadata = if file.exists() {
        serde_json::from_str::<Value>(&fs::read_to_string(&file).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
    } else {
        workspace_metadata()
    };
    // Preserve fork-specific attribution/extension fields; only advance the schema.
    metadata["schema_version"] = NEXUS_SCHEMA_VERSION.into();
    let contents = serde_json::to_string_pretty(&metadata).map_err(|e| e.to_string())? + "\n";
    if fs::read_to_string(&file).ok().as_deref() == Some(&contents) {
        return Ok(());
    }
    let temporary = path.join("workspace.json.tmp");
    fs::write(&temporary, contents).map_err(|e| e.to_string())?;
    // std::fs::rename replaces the destination atomically on supported platforms.
    fs::rename(temporary, file).map_err(|e| e.to_string())
}
