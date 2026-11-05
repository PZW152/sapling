//! Tauri 命令：只做轉接，業務邏輯都在 sapling-core。

use std::time::{Duration, Instant};

use sapling_core::download::{DEFAULT_CONCURRENCY, DownloadProgress};
use sapling_core::versions::fetch_manifest;
use sapling_core::versions::install::{PrepareStage, prepare_version};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

const PROGRESS_EVENT: &str = "prepare-progress";
const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionList {
    latest: String,
    versions: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrepareProgress {
    stage: PrepareStage,
    #[serde(flatten)]
    progress: DownloadProgress,
}

#[tauri::command]
async fn list_versions() -> Result<VersionList, String> {
    let manifest = fetch_manifest().await.map_err(|error| error.to_string())?;
    let versions = manifest
        .supported_releases()
        .map_err(|error| error.to_string())?
        .iter()
        .map(|entry| entry.id.clone())
        .collect();
    Ok(VersionList {
        latest: manifest.latest.release.clone(),
        versions,
    })
}

#[tauri::command]
async fn prepare(app: AppHandle, id: String) -> Result<(), String> {
    let mut last_sent = Instant::now();
    prepare_version(&id, DEFAULT_CONCURRENCY, move |stage, progress| {
        let is_last = progress.finished_files == progress.total_files;
        if is_last || last_sent.elapsed() >= PROGRESS_INTERVAL {
            last_sent = Instant::now();
            // 進度只用於顯示，送不出去不影響下載本身。
            let _ = app.emit(PROGRESS_EVENT, PrepareProgress { stage, progress });
        }
    })
    .await
    .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_versions, prepare])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
