//! Stable support codes for real failures. Never infer a code from message wording.
use serde::Serialize;

macro_rules! error_registry {
    ($($variant:ident => ($code:literal, $condition:literal)),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum ErrorCode { $($variant),+ }
        impl ErrorCode {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub const fn as_str(self) -> &'static str { match self { $(Self::$variant => $code),+ } }
            pub const fn condition(self) -> &'static str { match self { $(Self::$variant => $condition),+ } }
        }
    }
}
error_registry! {
    ScopeRejected => ("NX-SCP-2749", "Scope rule or scan validation failed"),
    ImportFailed => ("NX-IMP-2417", "Import parsing or commit failed"),
    ImportTooLarge => ("NX-IMP-5931", "Import exceeds the supported size"),
    GraphFailed => ("NX-GPH-3802", "Graph or relationship operation failed"),
    VaultFailed => ("NX-VLT-4826", "Vault or credential operation failed"),
    PivotSourceMismatch => ("NX-PVT-7314", "Pivot host differs from its source session"),
    PivotNetworkInvalid => ("NX-RCH-8642", "Reachability requires a valid IPv4 pivot network"),
    DatabaseFailed => ("NX-DB-4283", "Workspace storage or request operation failed"),
    DatabaseFormatUnsupported => ("NX-DB-9076", "Database or workspace format is unsupported"),
    DatabaseIntegrityFailed => ("NX-DB-3568", "Database integrity check failed"),
    SnapshotFailed => ("NX-SNP-6193", "Snapshot creation or comparison failed"),
    CoverageFailed => ("NX-COV-8251", "Coverage read or validation failed")
}
impl ErrorCode {
    pub fn message(self, message: impl AsRef<str>) -> String {
        format!("[{}] {}", self.as_str(), message.as_ref())
    }
    fn for_operation(op: &str) -> Self {
        match op {
            "preview" | "import" | "read_import" => Self::ImportFailed,
            "graph" | "path" => Self::GraphFailed,
            "vault_unlock" | "vault_lock" | "credential" | "reveal" => Self::VaultFailed,
            "snapshot" | "diff" => Self::SnapshotFailed,
            "coverage" => Self::CoverageFailed,
            "remove_scope" | "runner_plan" | "runner_start" => Self::ScopeRejected,
            _ => Self::DatabaseFailed,
        }
    }
    pub fn registry() -> serde_json::Value {
        serde_json::json!(Self::ALL
            .iter()
            .map(|c| serde_json::json!({"code":c.as_str(),"condition":c.condition()}))
            .collect::<Vec<_>>())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NexusError {
    pub code: &'static str,
    pub message: String,
}
impl NexusError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str(),
            message: message.into(),
        }
    }
    pub fn from_operation(op: &str, message: String) -> Self {
        // Lower layers keep their existing Result<String> contract. Recognize only
        // a registered explicit prefix; free-form error prose is never classified.
        for code in ErrorCode::ALL {
            if let Some(detail) = message.strip_prefix(&format!("[{}] ", code.as_str())) {
                return Self::new(*code, detail);
            }
        }
        Self::new(ErrorCode::for_operation(op), message)
    }
}
impl std::fmt::Display for NexusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}
impl std::error::Error for NexusError {}
