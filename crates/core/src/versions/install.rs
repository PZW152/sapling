//! 安裝一個版本：下載遊戲本體、依賴庫和所有資源檔。

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::details::VersionDetails;
use super::{VersionEntry, VersionsError, fetch_manifest};
use crate::download::{
    DownloadError, DownloadProgress, DownloadTask, download_all, download_file, new_client,
};
use crate::java::{JavaError, install_runtime};
use crate::platform::{DataDirs, PlatformError};

const RESOURCES_URL: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Error)]
pub enum InstallError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error(transparent)]
    Versions(#[from] VersionsError),
    #[error(transparent)]
    Java(#[from] JavaError),
    #[error(transparent)]
    Platform(#[from] PlatformError),
    #[error("版本清單裡找不到 {id}")]
    UnknownVersion { id: String },
    #[error("無法讀取已下載的檔案：{}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// 資源索引：遊戲資源名稱對應到 hash。
#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexFile {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

/// 準備遊戲時正在進行的階段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PrepareStage {
    Game,
    Java,
}

/// 準備好某個版本需要的一切：遊戲檔案和對應的 Java。
pub async fn prepare_version(
    id: &str,
    concurrency: usize,
    mut on_progress: impl FnMut(PrepareStage, DownloadProgress) + Send,
) -> Result<(), InstallError> {
    let dirs = DataDirs::for_this_user()?;
    let client = new_client()?;
    let manifest = fetch_manifest().await?;
    let entry = manifest
        .versions
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| InstallError::UnknownVersion { id: id.to_string() })?;

    let details = install_version(&client, entry, &dirs, concurrency, |progress| {
        on_progress(PrepareStage::Game, progress)
    })
    .await?;
    install_runtime(
        &client,
        &details.java_version.component,
        &dirs,
        concurrency,
        |progress| on_progress(PrepareStage::Java, progress),
    )
    .await?;
    Ok(())
}

/// 下載一個版本需要的全部檔案。已存在且 SHA1 正確的檔案會跳過。
pub async fn install_version(
    client: &reqwest::Client,
    entry: &VersionEntry,
    dirs: &DataDirs,
    concurrency: usize,
    on_progress: impl FnMut(DownloadProgress),
) -> Result<VersionDetails, InstallError> {
    let details_path = dirs
        .versions()
        .join(&entry.id)
        .join(format!("{}.json", entry.id));
    let details_task = DownloadTask {
        url: entry.url.clone(),
        path: details_path.clone(),
        sha1: entry.sha1.clone(),
        size: 0,
    };
    download_file(client, &details_task).await?;
    let details = VersionDetails::parse(&read_text(&details_path).await?)?;

    let index = &details.asset_index;
    let index_path = dirs
        .assets()
        .join("indexes")
        .join(format!("{}.json", index.id));
    let index_task = DownloadTask {
        url: index.url.clone(),
        path: index_path.clone(),
        sha1: index.sha1.clone(),
        size: index.size,
    };
    download_file(client, &index_task).await?;
    let asset_index: AssetIndexFile =
        serde_json::from_str(&read_text(&index_path).await?).map_err(VersionsError::from)?;

    let tasks = game_file_tasks(&details, &asset_index, dirs);
    download_all(client, tasks, concurrency, on_progress).await?;
    Ok(details)
}

async fn read_text(path: &Path) -> Result<String, InstallError> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(|source| InstallError::Read {
            path: path.to_path_buf(),
            source,
        })
}

/// 列出遊戲本體、本機需要的依賴庫和所有資源檔，路徑不重複。
fn game_file_tasks(
    details: &VersionDetails,
    asset_index: &AssetIndexFile,
    dirs: &DataDirs,
) -> Vec<DownloadTask> {
    let client = &details.downloads.client;
    let client_task = DownloadTask {
        url: client.url.clone(),
        path: dirs
            .versions()
            .join(&details.id)
            .join(format!("{}.jar", details.id)),
        sha1: client.sha1.clone(),
        size: client.size,
    };

    let libraries_dir = dirs.libraries();
    let library_tasks = details
        .libraries_for_this_machine()
        .into_iter()
        .map(|library| {
            let artifact = &library.downloads.artifact;
            DownloadTask {
                url: artifact.url.clone(),
                path: libraries_dir.join(&artifact.path),
                sha1: artifact.sha1.clone(),
                size: artifact.size,
            }
        });

    let objects_dir = dirs.assets().join("objects");
    let asset_tasks = asset_index.objects.values().map(|object| {
        let prefix = object.hash.get(..2).unwrap_or(&object.hash);
        DownloadTask {
            url: format!("{RESOURCES_URL}/{prefix}/{}", object.hash),
            path: objects_dir.join(prefix).join(&object.hash),
            sha1: object.hash.clone(),
            size: object.size,
        }
    });

    let mut seen_paths = HashSet::new();
    std::iter::once(client_task)
        .chain(library_tasks)
        .chain(asset_tasks)
        .filter(|task| seen_paths.insert(task.path.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details() -> VersionDetails {
        VersionDetails::parse(
            r#"{
                "id": "26.3",
                "mainClass": "m",
                "assetIndex": { "id": "34", "sha1": "a", "size": 2, "totalSize": 3, "url": "u" },
                "downloads": { "client": { "sha1": "c", "size": 4, "url": "client-url" } },
                "javaVersion": { "component": "j", "majorVersion": 25 },
                "libraries": [
                    {
                        "name": "a:b:1",
                        "downloads": { "artifact": { "path": "a/b/1/b-1.jar", "sha1": "l", "size": 5, "url": "lib-url" } }
                    },
                    {
                        "name": "a:c:1:natives-linux",
                        "downloads": { "artifact": { "path": "a/c.jar", "sha1": "n", "size": 6, "url": "linux-url" } },
                        "rules": [{ "action": "allow", "os": { "name": "linux" } }]
                    }
                ]
            }"#,
        )
        .unwrap()
    }

    fn assets() -> AssetIndexFile {
        serde_json::from_str(
            r#"{ "objects": {
                "first.png": { "hash": "abcdef", "size": 7 },
                "same-content.png": { "hash": "abcdef", "size": 7 }
            } }"#,
        )
        .unwrap()
    }

    #[test]
    fn lists_client_needed_libraries_and_unique_assets() {
        let dirs = DataDirs::at("root");
        let tasks = game_file_tasks(&details(), &assets(), &dirs);

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].url, "client-url");
        assert_eq!(tasks[0].path, Path::new("root/versions/26.3/26.3.jar"));
        assert_eq!(tasks[1].path, Path::new("root/libraries/a/b/1/b-1.jar"));
        assert_eq!(
            tasks[2].url,
            "https://resources.download.minecraft.net/ab/abcdef"
        );
        assert_eq!(tasks[2].path, Path::new("root/assets/objects/ab/abcdef"));
    }
}
