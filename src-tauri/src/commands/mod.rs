use serde_json::Value;
use std::sync::Mutex;
#[tauri::command]
pub async fn nexus(
    state: tauri::State<'_, Mutex<crate::core::Api>>,
    op: String,
    args: Value,
) -> Result<Value, crate::errors::NexusError> {
    state
        .lock()
        .map_err(|_| {
            crate::errors::NexusError::new(
                crate::errors::ErrorCode::DatabaseFailed,
                "The workspace lock was poisoned; restart NEXUS to recover",
            )
        })?
        .dispatch(&op, args)
}
