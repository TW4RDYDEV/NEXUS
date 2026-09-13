//! NEXUS — TWARDY.exe / TW4RDYDEV
//! Public file-format identity, independent of any installation or person.
use serde_json::{json, Value};

pub const NEXUS_PRODUCT: &str = "NEXUS";
pub const NEXUS_PRODUCT_ID: &str = "nexus.tw4rdy.core";
pub const NEXUS_FORMAT_ID: &str = "nexus-engagement";
pub const NEXUS_SCHEMA_FAMILY: &str = "nx-engagement-graph";
pub const NEXUS_APPLICATION_ID: i32 = 0x4E585553;
pub const NEXUS_FORMAT_VERSION: i64 = 1;
pub const NEXUS_SCHEMA_VERSION: i64 = 3;

pub fn workspace_metadata() -> Value {
    json!({"format":NEXUS_FORMAT_ID,"format_version":NEXUS_FORMAT_VERSION,
        "product":NEXUS_PRODUCT,"product_id":NEXUS_PRODUCT_ID,
        "schema_family":NEXUS_SCHEMA_FAMILY,"schema_version":NEXUS_SCHEMA_VERSION})
}

/// Deliberate allowlist: never reads runtime environment, workspace data or paths.
pub fn build_metadata() -> Value {
    json!({"product":NEXUS_PRODUCT,"product_id":NEXUS_PRODUCT_ID,
        "application_version":env!("CARGO_PKG_VERSION"),
        "schema_version":NEXUS_SCHEMA_VERSION,"format":NEXUS_FORMAT_ID,
        "schema_family":NEXUS_SCHEMA_FAMILY,"git_commit":env!("NEXUS_BUILD_COMMIT"),
        "build_mode":env!("NEXUS_BUILD_MODE")})
}
