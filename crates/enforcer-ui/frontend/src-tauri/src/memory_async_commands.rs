//! Async adapter for memory indexing; filesystem and store work stays in the
//! synchronous memory command module and never runs on Tauri's async reactor.

use super::memory_commands::{create_memory_index_sync, IndexProjectPayload};

#[tauri::command]
pub(crate) async fn create_memory_index(root: String) -> Result<IndexProjectPayload, String> {
    tauri::async_runtime::spawn_blocking(move || create_memory_index_sync(root))
        .await
        .map_err(|error| format!("memory index task failed: {error}"))?
}
