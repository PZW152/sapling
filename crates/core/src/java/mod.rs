//! Java runtime：按版本 JSON 的 `javaVersion.component`，從 Mojang 下載對應的 Java。

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

use crate::download::{
    DownloadError, DownloadProgress, DownloadTask, download_all, download_file, fetch_text,
};
use crate::platform::{DataDirs, JAVA_EXE_PATH, MOJANG_RUNTIME_PLATFORM};

const RUNTIME_INDEX_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Debug, Error)]
pub enum JavaError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error("Java 清單的格式看不懂，可能是 Mojang 改了格式")]
    Parse(#[from] serde_json::Error),
    #[error("Mojang 沒有提供這台電腦可用的 Java：{component}")]
    NotAvailable { component: String },
    #[error("無法讀取已下載的 Java 清單：{}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// 系統平台 → Java 種類 → 可用的 runtime。
type RuntimeIndex = HashMap<String, HashMap<String, Vec<RuntimeEntry>>>;

#[derive(Debug, Clone, Deserialize)]
struct RuntimeEntry {
    manifest: RuntimeManifestRef,
}

#[derive(Debug, Clone, Deserialize)]
struct RuntimeManifestRef {
    sha1: String,
    size: u64,
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RuntimeManifest {
    files: HashMap<String, RuntimeFile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RuntimeFile {
    File {
        downloads: RuntimeFileDownloads,
    },
    /// 資料夾和捷徑：資料夾會在下載檔案時自動建立，Windows 的清單裡沒有捷徑。
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
struct RuntimeFileDownloads {
    raw: RawFile,
}

#[derive(Debug, Clone, Deserialize)]
struct RawFile {
    sha1: String,
    size: u64,
    url: String,
}

/// 確保 `component` 對應的 Java 已安裝，回傳 `java.exe` 的位置。
pub async fn install_runtime(
    client: &reqwest::Client,
    component: &str,
    dirs: &DataDirs,
    concurrency: usize,
    on_progress: impl FnMut(DownloadProgress),
) -> Result<PathBuf, JavaError> {
    let index: RuntimeIndex = serde_json::from_str(&fetch_text(client, RUNTIME_INDEX_URL).await?)?;
    let manifest_ref = find_manifest(&index, component)?;

    let manifest_path = dirs.runtimes().join(format!("{component}.json"));
    let manifest_task = DownloadTask {
        url: manifest_ref.url.clone(),
        path: manifest_path.clone(),
        sha1: manifest_ref.sha1.clone(),
        size: manifest_ref.size,
    };
    download_file(client, &manifest_task).await?;
    let manifest_text =
        tokio::fs::read_to_string(&manifest_path)
            .await
            .map_err(|source| JavaError::Read {
                path: manifest_path.clone(),
                source,
            })?;
    let manifest: RuntimeManifest = serde_json::from_str(&manifest_text)?;

    let runtime_dir = dirs.runtimes().join(component);
    let tasks = runtime_file_tasks(&manifest, &runtime_dir);
    download_all(client, tasks, concurrency, on_progress).await?;
    Ok(runtime_dir.join(JAVA_EXE_PATH))
}

fn find_manifest<'a>(
    index: &'a RuntimeIndex,
    component: &str,
) -> Result<&'a RuntimeManifestRef, JavaError> {
    index
        .get(MOJANG_RUNTIME_PLATFORM)
        .and_then(|components| components.get(component))
        .and_then(|entries| entries.first())
        .map(|entry| &entry.manifest)
        .ok_or_else(|| JavaError::NotAvailable {
            component: component.to_string(),
        })
}

fn runtime_file_tasks(manifest: &RuntimeManifest, runtime_dir: &std::path::Path) -> Vec<DownloadTask> {
    manifest
        .files
        .iter()
        .filter_map(|(path, file)| match file {
            RuntimeFile::File { downloads } => Some(DownloadTask {
                url: downloads.raw.url.clone(),
                path: runtime_dir.join(path),
                sha1: downloads.raw.sha1.clone(),
                size: downloads.raw.size,
            }),
            RuntimeFile::Other => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn finds_the_manifest_for_windows_x64() {
        let index: RuntimeIndex = serde_json::from_str(
            r#"{
                "linux": { "java-runtime-epsilon": [] },
                "windows-x64": {
                    "java-runtime-delta": [],
                    "java-runtime-epsilon": [{
                        "manifest": { "sha1": "s", "size": 1, "url": "manifest-url" },
                        "version": { "name": "25.0.1" }
                    }]
                }
            }"#,
        )
        .unwrap();

        let found = find_manifest(&index, "java-runtime-epsilon").unwrap();
        assert_eq!(found.url, "manifest-url");
        assert!(matches!(
            find_manifest(&index, "java-runtime-delta"),
            Err(JavaError::NotAvailable { .. })
        ));
        assert!(matches!(
            find_manifest(&index, "java-runtime-unknown"),
            Err(JavaError::NotAvailable { .. })
        ));
    }

    #[test]
    fn lists_files_and_skips_directories() {
        let manifest: RuntimeManifest = serde_json::from_str(
            r#"{ "files": {
                "bin": { "type": "directory" },
                "bin/java.exe": {
                    "type": "file",
                    "executable": true,
                    "downloads": {
                        "lzma": { "sha1": "z", "size": 1, "url": "lzma-url" },
                        "raw": { "sha1": "r", "size": 2, "url": "raw-url" }
                    }
                }
            } }"#,
        )
        .unwrap();

        let tasks = runtime_file_tasks(&manifest, Path::new("runtime"));

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].url, "raw-url");
        assert_eq!(tasks[0].sha1, "r");
        assert_eq!(tasks[0].path, Path::new("runtime/bin/java.exe"));
    }
}
