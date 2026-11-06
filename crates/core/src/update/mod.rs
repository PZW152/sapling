//! 自動更新：檢查 GitHub 最新版本，下載免安裝版並替換目前的程式。

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/PZW152/sapling/releases/latest";
const USER_AGENT: &str = concat!("Sapling-Launcher/", env!("CARGO_PKG_VERSION"));
const PORTABLE_SUFFIX: &str = "_portable.exe";
const DIGEST_PREFIX: &str = "sha256:";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("無法檢查更新，請檢查網路連線後再試一次")]
    Network(#[from] reqwest::Error),
    #[error("新版本下載後校驗不通過，已取消更新")]
    ChecksumMismatch,
    #[error("無法替換程式檔案：{}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    body: Option<String>,
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

/// 可以安裝的新版本。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub url: String,
    pub sha256: String,
}

/// 檢查有沒有比目前新的版本。
pub async fn check(client: &reqwest::Client) -> Result<Option<UpdateInfo>, UpdateError> {
    let release: Release = client
        .get(LATEST_RELEASE_URL)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(update_from(release, CURRENT_VERSION))
}

fn update_from(release: Release, current: &str) -> Option<UpdateInfo> {
    let version = release.tag_name.trim_start_matches('v').to_string();
    if parse_version(&version)? <= parse_version(current)? {
        return None;
    }
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name.ends_with(PORTABLE_SUFFIX))?;
    let sha256 = asset.digest?.strip_prefix(DIGEST_PREFIX)?.to_string();
    Some(UpdateInfo {
        version,
        notes: release.body.unwrap_or_default(),
        url: asset.browser_download_url,
        sha256,
    })
}

fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version.split('.').map(|part| part.parse().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

/// 下載新版本並替換目前的程式。完成後呼叫方要重新啟動。
///
/// Windows 不能覆蓋執行中的程式，但可以改名，所以舊檔先改名為 `.old`。
pub async fn install(
    client: &reqwest::Client,
    update: &UpdateInfo,
) -> Result<PathBuf, UpdateError> {
    let exe = std::env::current_exe().map_err(|source| UpdateError::Io {
        path: PathBuf::from("sapling.exe"),
        source,
    })?;
    let new_exe = with_suffix(&exe, ".new");
    download_verified(client, update, &new_exe).await?;

    let old_exe = with_suffix(&exe, ".old");
    let io_error = |source| UpdateError::Io {
        path: exe.clone(),
        source,
    };
    match tokio::fs::remove_file(&old_exe).await {
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(io_error(error)),
        _ => {}
    }
    tokio::fs::rename(&exe, &old_exe).await.map_err(io_error)?;
    if let Err(error) = tokio::fs::rename(&new_exe, &exe).await {
        // 換不上新版時放回舊版，避免程式檔案消失。
        let _ = tokio::fs::rename(&old_exe, &exe).await;
        return Err(io_error(error));
    }
    Ok(exe)
}

async fn download_verified(
    client: &reqwest::Client,
    update: &UpdateInfo,
    path: &Path,
) -> Result<(), UpdateError> {
    let io_error = |source| UpdateError::Io {
        path: path.to_path_buf(),
        source,
    };
    let mut response = client
        .get(&update.url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?
        .error_for_status()?;
    let mut file = tokio::fs::File::create(path).await.map_err(io_error)?;
    let mut hasher = Sha256::new();
    while let Some(chunk) = response.chunk().await? {
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(io_error)?;
    }
    file.flush().await.map_err(io_error)?;
    drop(file);

    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if digest.eq_ignore_ascii_case(&update.sha256) {
        Ok(())
    } else {
        let _ = tokio::fs::remove_file(path).await;
        Err(UpdateError::ChecksumMismatch)
    }
}

/// 啟動時清理上次更新留下的舊程式。失敗不影響使用，下次再清。
pub fn remove_old_version() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(with_suffix(&exe, ".old"));
    }
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, digest: Option<&str>) -> Release {
        Release {
            tag_name: tag.to_string(),
            body: Some("notes".to_string()),
            assets: vec![
                ReleaseAsset {
                    name: "Sapling_setup.exe".to_string(),
                    browser_download_url: "setup".to_string(),
                    digest: Some("sha256:bad".to_string()),
                },
                ReleaseAsset {
                    name: "Sapling_x64_portable.exe".to_string(),
                    browser_download_url: "portable".to_string(),
                    digest: digest.map(str::to_string),
                },
            ],
        }
    }

    #[test]
    fn finds_a_newer_portable_build() {
        let update = update_from(release("v0.10.0", Some("sha256:abc")), "0.9.9").unwrap();
        assert_eq!(update.version, "0.10.0");
        assert_eq!(update.url, "portable");
        assert_eq!(update.sha256, "abc");
    }

    #[test]
    fn ignores_the_same_or_older_version() {
        assert!(update_from(release("v0.2.0", Some("sha256:abc")), "0.2.0").is_none());
        assert!(update_from(release("v0.1.9", Some("sha256:abc")), "0.2.0").is_none());
    }

    #[test]
    fn refuses_a_build_without_a_checksum() {
        assert!(update_from(release("v9.0.0", None), "0.2.0").is_none());
    }
}
