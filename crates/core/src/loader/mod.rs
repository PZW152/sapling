//! 載入器：在原版版本資料之上補充或覆蓋 libraries、mainClass、參數。
//!
//! 新載入器只要能產生 [`profile::LoaderProfile`]，其餘流程都共用。

mod fabric;
mod forge;
mod installer;
mod profile;

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::download::{DownloadError, download_all, new_client};
use crate::instance::Instance;
use crate::platform::DataDirs;
use crate::storage::{StorageError, read_json, write_json};
use crate::versions::details::VersionDetails;

use profile::LoaderProfile;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("載入器資料的格式看不懂，可能是對方改了格式")]
    Parse(#[from] serde_json::Error),
    #[error("{loader} 還不支援 Minecraft {version}")]
    Unsupported {
        loader: &'static str,
        version: String,
    },
    #[error("載入器資料裡有無法使用的項目：{name}")]
    BadLibrary { name: String },
    #[error("載入器的安裝程式看不懂，可能已損壞")]
    BadInstaller,
    #[error("無法寫入檔案：{}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("無法執行載入器的安裝工具，請重新下載遊戲檔案後再試一次")]
    Processor(#[source] io::Error),
    #[error("載入器安裝失敗（{tool}）：{detail}")]
    ProcessorFailed { tool: String, detail: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoaderKind {
    Vanilla,
    Fabric,
    Forge,
    NeoForge,
}

/// 載入器在 `versions` 裡的資料夾名稱。原版沒有。
pub fn folder_id(kind: LoaderKind, loader_version: &str, game_version: &str) -> Option<String> {
    match kind {
        LoaderKind::Vanilla => None,
        LoaderKind::Fabric => Some(fabric::profile_id(loader_version, game_version)),
        LoaderKind::Forge => Some(forge::forge_request(loader_version, game_version).id),
        LoaderKind::NeoForge => Some(forge::neoforge_request(loader_version, game_version).id),
    }
}

/// 這個實例的載入器用到的函式庫路徑。
/// 回傳 `None` 代表無法列全：Forge 系列有安裝時才產生、清單裡沒有的檔案。
pub async fn known_libraries(instance: &Instance, dirs: &DataDirs) -> Option<Vec<PathBuf>> {
    match instance.loader {
        LoaderKind::Vanilla => Some(Vec::new()),
        LoaderKind::Forge | LoaderKind::NeoForge => None,
        LoaderKind::Fabric => {
            let id = fabric::profile_id(&instance.loader_version, &instance.version);
            let path = dirs.versions().join(&id).join(format!("{id}.json"));
            let raw: Option<Value> = read_json(&path).await.ok()?;
            let Some(raw) = raw else {
                return Some(Vec::new());
            };
            let profile: LoaderProfile = serde_json::from_value(raw).ok()?;
            let libraries = profile.libraries().ok()?;
            Some(
                libraries
                    .iter()
                    .filter_map(|library| library.downloads.artifact.as_ref())
                    .map(|artifact| PathBuf::from(&artifact.path))
                    .collect(),
            )
        }
    }
}

/// 這個遊戲版本可以用的最新穩定載入器版本。原版沒有載入器版本。
pub async fn latest_version(kind: LoaderKind, game_version: &str) -> Result<String, LoaderError> {
    match kind {
        LoaderKind::Vanilla => Ok(String::new()),
        LoaderKind::Fabric => fabric::latest_version(&new_client()?, game_version).await,
        LoaderKind::Forge => forge::latest_forge(&new_client()?, game_version).await,
        LoaderKind::NeoForge => forge::latest_neoforge(&new_client()?, game_version).await,
    }
}

/// 確保載入器需要的檔案都在，回傳實際用於啟動的版本資料。
///
/// 載入器資料下載過就存在本機，之後沒有網路也能啟動。
pub async fn prepare(
    kind: LoaderKind,
    loader_version: &str,
    details: VersionDetails,
    java: &Path,
    dirs: &DataDirs,
    concurrency: usize,
) -> Result<VersionDetails, LoaderError> {
    let client = new_client()?;
    let game_version = details.id.clone();
    let raw = match kind {
        LoaderKind::Vanilla => return Ok(details),
        LoaderKind::Fabric => fabric_profile(&client, loader_version, &game_version, dirs).await?,
        LoaderKind::Forge => {
            let request = forge::forge_request(loader_version, &game_version);
            installer::ensure_installed(&client, &request, java, dirs, concurrency).await?
        }
        LoaderKind::NeoForge => {
            let request = forge::neoforge_request(loader_version, &game_version);
            installer::ensure_installed(&client, &request, java, dirs, concurrency).await?
        }
    };
    let profile: LoaderProfile = serde_json::from_value(raw)?;

    let libraries = profile.libraries()?;
    let tasks = installer::library_tasks(&libraries, dirs);
    // 載入器的檔案不大，不單獨顯示進度。
    download_all(&client, tasks, concurrency, |_| {}).await?;

    Ok(profile.apply(details, libraries))
}

async fn fabric_profile(
    client: &reqwest::Client,
    loader_version: &str,
    game_version: &str,
    dirs: &DataDirs,
) -> Result<Value, LoaderError> {
    let id = fabric::profile_id(loader_version, game_version);
    let path = dirs.versions().join(&id).join(format!("{id}.json"));
    if let Some(saved) = read_json(&path).await? {
        return Ok(saved);
    }
    let fetched = fabric::fetch_profile(client, loader_version, game_version).await?;
    write_json(&path, &fetched).await?;
    Ok(fetched)
}
