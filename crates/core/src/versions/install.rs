//! 安裝一個版本：下載遊戲本體、依賴庫和所有資源檔。

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::details::{LoggingFile, VersionDetails};
use super::{VersionEntry, VersionsError, fetch_manifest};
use crate::download::{
    DownloadError, DownloadProgress, DownloadTask, download_all, download_file, new_client,
};
use crate::java::{JavaError, install_runtime, local_runtime};
use crate::platform::DataDirs;

const RESOURCES_URL: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Error)]
pub enum InstallError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error(transparent)]
    Versions(#[from] VersionsError),
    #[error(transparent)]
    Java(#[from] JavaError),
    #[error("版本清單裡找不到 {id}")]
    UnknownVersion { id: String },
    #[error("無法讀取已下載的檔案：{}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("無法記錄下載結果：{}", path.display())]
    Mark {
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

/// 已經準備好、可以啟動的版本。
#[derive(Debug, Clone)]
pub struct PreparedVersion {
    pub details: VersionDetails,
    pub java: PathBuf,
}

/// 準備好某個版本需要的一切：遊戲檔案和對應的 Java。
///
/// 已安裝的版本只確認檔案還在，不用網路也不計算 SHA1；有問題才走完整校驗。
pub async fn prepare_version(
    id: &str,
    dirs: &DataDirs,
    concurrency: usize,
    mut on_progress: impl FnMut(PrepareStage, DownloadProgress) + Send,
) -> Result<PreparedVersion, InstallError> {
    if let Some(prepared) = prepare_installed(id, dirs).await {
        return Ok(prepared);
    }

    let client = new_client()?;
    let manifest = fetch_manifest(&client).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| InstallError::UnknownVersion { id: id.to_string() })?;

    let details = install_version(&client, entry, dirs, concurrency, |progress| {
        on_progress(PrepareStage::Game, progress)
    })
    .await?;
    let java = install_runtime(
        &client,
        &details.java_version.component,
        dirs,
        concurrency,
        |progress| on_progress(PrepareStage::Java, progress),
    )
    .await?;

    let marker = installed_marker(dirs, id);
    tokio::fs::write(&marker, b"")
        .await
        .map_err(|source| InstallError::Mark {
            path: marker,
            source,
        })?;
    // 記號不存在時刪除會失敗，這是正常情況。
    let _ = tokio::fs::remove_file(recheck_marker(dirs, id)).await;
    Ok(PreparedVersion { details, java })
}

/// 用本機資料確認版本完整。任何一步不符合都回傳 `None`，交給完整校驗處理。
async fn prepare_installed(id: &str, dirs: &DataDirs) -> Option<PreparedVersion> {
    if !is_installed(dirs, id) || recheck_marker(dirs, id).exists() {
        return None;
    }
    let details_text = tokio::fs::read_to_string(details_path(dirs, id))
        .await
        .ok()?;
    let details = VersionDetails::parse(&details_text).ok()?;
    let index_path = index_path(dirs, &details.asset_index.id);
    let index_text = tokio::fs::read_to_string(index_path).await.ok()?;
    let asset_index: AssetIndexFile = serde_json::from_str(&index_text).ok()?;
    let (java, runtime_tasks) = local_runtime(&details.java_version.component, dirs).await?;

    let mut tasks = game_file_tasks(&details, &asset_index, dirs);
    tasks.extend(runtime_tasks);
    let complete = tokio::task::spawn_blocking(move || tasks.iter().all(has_expected_size))
        .await
        .ok()?;
    complete.then_some(PreparedVersion { details, java })
}

fn has_expected_size(task: &DownloadTask) -> bool {
    std::fs::metadata(&task.path).is_ok_and(|file| file.len() == task.size)
}

/// 下次啟動前完整校驗這個版本。遊戲異常結束時使用，檔案損壞可能是原因。
pub async fn request_full_check(dirs: &DataDirs, id: &str) -> io::Result<()> {
    tokio::fs::write(recheck_marker(dirs, id), b"").await
}

/// 這個版本是否已經完整下載。下載中斷的版本不算。
pub fn is_installed(dirs: &DataDirs, id: &str) -> bool {
    installed_marker(dirs, id).exists()
}

fn installed_marker(dirs: &DataDirs, id: &str) -> PathBuf {
    dirs.versions().join(id).join(".installed")
}

fn recheck_marker(dirs: &DataDirs, id: &str) -> PathBuf {
    dirs.versions().join(id).join(".recheck")
}

/// 有 Log4Shell 漏洞的版本才需要 Mojang 的修補設定。
fn log_config(details: &VersionDetails) -> Option<&LoggingFile> {
    if !details.needs_log4j_fix() {
        return None;
    }
    let client = details.logging.as_ref()?.client.as_ref()?;
    Some(&client.file)
}

/// 啟動這個版本要加上的修補參數。
pub fn log_config_argument(details: &VersionDetails, dirs: &DataDirs) -> Option<String> {
    let file = log_config(details)?;
    let client = details.logging.as_ref()?.client.as_ref()?;
    let path = log_config_path(dirs, &file.id);
    Some(client.argument.replace("${path}", &path.to_string_lossy()))
}

fn log_config_path(dirs: &DataDirs, id: &str) -> PathBuf {
    dirs.assets().join("log_configs").join(id)
}

fn details_path(dirs: &DataDirs, id: &str) -> PathBuf {
    dirs.versions().join(id).join(format!("{id}.json"))
}

fn index_path(dirs: &DataDirs, index_id: &str) -> PathBuf {
    dirs.assets()
        .join("indexes")
        .join(format!("{index_id}.json"))
}

/// 下載一個版本需要的全部檔案。已存在且 SHA1 正確的檔案會跳過。
pub async fn install_version(
    client: &reqwest::Client,
    entry: &VersionEntry,
    dirs: &DataDirs,
    concurrency: usize,
    on_progress: impl FnMut(DownloadProgress),
) -> Result<VersionDetails, InstallError> {
    let details_path = details_path(dirs, &entry.id);
    let details_task = DownloadTask {
        url: entry.url.clone(),
        path: details_path.clone(),
        sha1: entry.sha1.clone(),
        size: 0,
    };
    download_file(client, &details_task).await?;
    let details = VersionDetails::parse(&read_text(&details_path).await?)?;

    let index = &details.asset_index;
    let index_path = index_path(dirs, &index.id);
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
        .flat_map(|library| {
            library
                .downloads
                .artifact
                .iter()
                .chain(library.native_artifact())
        })
        .map(|artifact| DownloadTask {
            url: artifact.url.clone(),
            path: libraries_dir.join(&artifact.path),
            sha1: artifact.sha1.clone(),
            size: artifact.size,
        });
    let log_config_task = log_config(details).map(|file| DownloadTask {
        url: file.url.clone(),
        path: log_config_path(dirs, &file.id),
        sha1: file.sha1.clone(),
        size: file.size,
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
        .chain(log_config_task)
        .chain(asset_tasks)
        .filter(|task| seen_paths.insert(task.path.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DETAILS_JSON: &str = r#"{
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
            }"#;
    const ASSETS_JSON: &str = r#"{ "objects": {
                "first.png": { "hash": "abcdef", "size": 7 },
                "same-content.png": { "hash": "abcdef", "size": 7 }
            } }"#;
    const RUNTIME_JSON: &str = r#"{ "files": {
                "bin/java.exe": { "type": "file", "downloads": { "raw": { "sha1": "j", "size": 3, "url": "u" } } }
            } }"#;

    fn details() -> VersionDetails {
        VersionDetails::parse(DETAILS_JSON).unwrap()
    }

    fn assets() -> AssetIndexFile {
        serde_json::from_str(ASSETS_JSON).unwrap()
    }

    fn write(path: PathBuf, content: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    /// 一個所有檔案都齊全的已安裝版本。
    fn installed_version(name: &str) -> DataDirs {
        let root =
            std::env::temp_dir().join(format!("sapling-installed-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dirs = DataDirs::at(root);
        let version = dirs.versions().join("26.3");
        write(version.join("26.3.json"), DETAILS_JSON.as_bytes());
        write(version.join("26.3.jar"), b"1234");
        write(version.join(".installed"), b"");
        write(
            dirs.assets().join("indexes/34.json"),
            ASSETS_JSON.as_bytes(),
        );
        write(dirs.assets().join("objects/ab/abcdef"), b"1234567");
        write(dirs.libraries().join("a/b/1/b-1.jar"), b"12345");
        write(dirs.runtimes().join("j.json"), RUNTIME_JSON.as_bytes());
        write(dirs.runtimes().join("j/bin/java.exe"), b"123");
        dirs
    }

    #[tokio::test]
    async fn trusts_an_installed_version_with_all_files_present() {
        let dirs = installed_version("complete");

        let prepared = prepare_installed("26.3", &dirs).await.unwrap();

        assert_eq!(prepared.details.id, "26.3");
        assert!(prepared.java.starts_with(dirs.runtimes().join("j")));
        std::fs::remove_dir_all(dirs.root()).unwrap();
    }

    #[tokio::test]
    async fn needs_a_full_check_when_a_file_is_missing_or_changed() {
        let dirs = installed_version("broken");

        std::fs::write(dirs.libraries().join("a/b/1/b-1.jar"), b"1").unwrap();
        assert!(prepare_installed("26.3", &dirs).await.is_none());

        std::fs::remove_file(dirs.libraries().join("a/b/1/b-1.jar")).unwrap();
        assert!(prepare_installed("26.3", &dirs).await.is_none());
        std::fs::remove_dir_all(dirs.root()).unwrap();
    }

    #[tokio::test]
    async fn needs_a_full_check_after_the_game_crashed() {
        let dirs = installed_version("crashed");

        request_full_check(&dirs, "26.3").await.unwrap();

        assert!(prepare_installed("26.3", &dirs).await.is_none());
        std::fs::remove_dir_all(dirs.root()).unwrap();
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
