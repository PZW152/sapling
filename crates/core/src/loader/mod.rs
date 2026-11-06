//! 載入器：在原版版本資料之上補充或覆蓋 libraries、mainClass、參數。
//!
//! 新載入器只要能產生 [`profile::LoaderProfile`]，其餘流程都共用。

mod fabric;
mod profile;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::download::{DownloadError, DownloadTask, download_all, new_client};
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoaderKind {
    Vanilla,
    Fabric,
}

/// 這個遊戲版本可以用的最新穩定載入器版本。原版沒有載入器版本。
pub async fn latest_version(kind: LoaderKind, game_version: &str) -> Result<String, LoaderError> {
    match kind {
        LoaderKind::Vanilla => Ok(String::new()),
        LoaderKind::Fabric => fabric::latest_version(&new_client()?, game_version).await,
    }
}

/// 確保載入器需要的檔案都在，回傳實際用於啟動的版本資料。
///
/// 載入器資料下載過就存在本機，之後沒有網路也能啟動。
pub async fn prepare(
    kind: LoaderKind,
    loader_version: &str,
    details: VersionDetails,
    dirs: &DataDirs,
    concurrency: usize,
) -> Result<VersionDetails, LoaderError> {
    if kind == LoaderKind::Vanilla {
        return Ok(details);
    }

    let client = new_client()?;
    let id = fabric::profile_id(loader_version, &details.id);
    let path = dirs.versions().join(&id).join(format!("{id}.json"));
    let raw: Value = match read_json(&path).await? {
        Some(saved) => saved,
        None => {
            let fetched = fabric::fetch_profile(&client, loader_version, &details.id).await?;
            write_json(&path, &fetched).await?;
            fetched
        }
    };
    let profile: LoaderProfile = serde_json::from_value(raw)?;

    let libraries = profile.libraries()?;
    let tasks = libraries
        .iter()
        .map(|library| {
            let artifact = &library.downloads.artifact;
            DownloadTask {
                url: artifact.url.clone(),
                path: dirs.libraries().join(&artifact.path),
                sha1: artifact.sha1.clone(),
                size: artifact.size,
            }
        })
        .collect();
    // 載入器的檔案只有幾 MB，不單獨顯示進度。
    download_all(&client, tasks, concurrency, |_| {}).await?;

    Ok(profile.apply(details, libraries))
}
