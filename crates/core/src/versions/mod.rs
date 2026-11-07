//! 版本清單：從 Mojang 取得所有 Minecraft 版本，並篩選出 v1 支援的正式版。

pub mod details;
pub mod install;
pub mod rules;

use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

use crate::download::{DownloadError, fetch_text};
use details::VersionDetails;

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// v1 支援的最舊正式版。
///
/// Mojang 在 2026 年改用 `26.1` 這類版本號，所以不比較版本號數字，
/// 而是靠清單順序（最新的排最前）判斷新舊。
const OLDEST_SUPPORTED_RELEASE: &str = "1.7.10";

#[derive(Debug, Error)]
pub enum VersionsError {
    #[error("無法取得版本資料，請檢查網路連線後再試一次")]
    Network(#[from] DownloadError),
    #[error("版本資料的格式看不懂，可能是 Mojang 改了格式")]
    Parse(#[from] serde_json::Error),
    #[error("版本清單裡找不到 {OLDEST_SUPPORTED_RELEASE}，無法判斷哪些版本受支援")]
    MissingOldestSupported,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: VersionKind,
    /// 這個版本的版本 JSON 下載地址。
    pub url: String,
    /// 版本 JSON 的 SHA1，下載後用來校驗。
    pub sha1: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionKind {
    Release,
    Snapshot,
    /// old_alpha、old_beta，以及 Mojang 日後可能新增的類型。
    #[serde(other)]
    Other,
}

impl VersionManifest {
    pub fn parse(json: &str) -> Result<Self, VersionsError> {
        Ok(serde_json::from_str(json)?)
    }

    /// 最近的快照版，最新的排最前。
    pub fn recent_snapshots(&self, limit: usize) -> Vec<&VersionEntry> {
        self.versions
            .iter()
            .filter(|entry| entry.kind == VersionKind::Snapshot)
            .take(limit)
            .collect()
    }

    /// v1 支援的正式版，最新的排最前。
    pub fn supported_releases(&self) -> Result<Vec<&VersionEntry>, VersionsError> {
        let releases: Vec<&VersionEntry> = self
            .versions
            .iter()
            .filter(|entry| entry.kind == VersionKind::Release)
            .collect();
        let oldest_index = releases
            .iter()
            .position(|entry| entry.id == OLDEST_SUPPORTED_RELEASE)
            .ok_or(VersionsError::MissingOldestSupported)?;
        Ok(releases[..=oldest_index].to_vec())
    }
}

/// 從 Mojang 下載並解析版本清單。
pub async fn fetch_manifest(client: &reqwest::Client) -> Result<VersionManifest, VersionsError> {
    VersionManifest::parse(&fetch_text(client, MANIFEST_URL).await?)
}

/// 取得版本清單並存在本機。沒有網路時用上次存的，已安裝的版本和新增實例才能繼續用。
pub async fn fetch_manifest_cached(
    client: &reqwest::Client,
    cache: &Path,
) -> Result<VersionManifest, VersionsError> {
    let fetched = fetch_text(client, MANIFEST_URL).await;
    manifest_or_cache(fetched.map_err(VersionsError::from), cache).await
}

/// 連上要先登入的 Wi-Fi 時，收到的是登入網頁，所以看不懂內容時也要改用快取。
/// 快取也不能用時回報原本的問題，那才是用戶要解決的事。
async fn manifest_or_cache(
    fetched: Result<String, VersionsError>,
    cache: &Path,
) -> Result<VersionManifest, VersionsError> {
    let parsed = fetched.and_then(|text| Ok((VersionManifest::parse(&text)?, text)));
    match parsed {
        Ok((manifest, text)) => {
            // 只是快取，寫不進去不影響這次使用。
            let _ = tokio::fs::write(cache, &text).await;
            Ok(manifest)
        }
        Err(error) => tokio::fs::read_to_string(cache)
            .await
            .ok()
            .and_then(|text| VersionManifest::parse(&text).ok())
            .ok_or(error),
    }
}

/// 下載並解析單個版本的版本 JSON。
pub async fn fetch_details(
    client: &reqwest::Client,
    entry: &VersionEntry,
) -> Result<VersionDetails, VersionsError> {
    VersionDetails::parse(&fetch_text(client, &entry.url).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "latest": { "release": "26.3", "snapshot": "26.4-snapshot-7" },
        "versions": [
            { "id": "26.4-snapshot-7", "type": "snapshot", "url": "u", "sha1": "s", "complianceLevel": 1 },
            { "id": "26.3", "type": "release", "url": "u", "sha1": "s", "complianceLevel": 1 },
            { "id": "1.21.11", "type": "release", "url": "u", "sha1": "s", "complianceLevel": 1 },
            { "id": "1.7.10", "type": "release", "url": "u", "sha1": "s", "complianceLevel": 1 },
            { "id": "1.18.2", "type": "release", "url": "u", "sha1": "s", "complianceLevel": 1 },
            { "id": "b1.7.3", "type": "old_beta", "url": "u", "sha1": "s", "complianceLevel": 0 }
        ]
    }"#;

    fn cache_file(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "sapling-manifest-{}-{name}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[tokio::test]
    async fn saves_a_good_manifest_for_later() {
        let cache = cache_file("good");

        let manifest = manifest_or_cache(Ok(SAMPLE.to_string()), &cache).await;

        assert_eq!(manifest.unwrap().latest.release, "26.3");
        assert_eq!(std::fs::read_to_string(&cache).unwrap(), SAMPLE);
    }

    #[tokio::test]
    async fn a_wifi_login_page_falls_back_to_the_cache() {
        let cache = cache_file("login-page");
        std::fs::write(&cache, SAMPLE).unwrap();

        let manifest = manifest_or_cache(Ok("<html>請先登入</html>".to_string()), &cache).await;

        assert_eq!(manifest.unwrap().latest.release, "26.3");
        assert_eq!(std::fs::read_to_string(&cache).unwrap(), SAMPLE);
    }

    #[tokio::test]
    async fn a_broken_cache_reports_the_original_problem() {
        let cache = cache_file("broken");
        std::fs::write(&cache, "{ 寫到一半").unwrap();

        let result = manifest_or_cache(Err(VersionsError::MissingOldestSupported), &cache).await;

        assert!(matches!(result, Err(VersionsError::MissingOldestSupported)));
    }

    #[test]
    fn lists_recent_snapshots_only() {
        let manifest = VersionManifest::parse(SAMPLE).unwrap();

        let ids: Vec<&str> = manifest
            .recent_snapshots(5)
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();

        assert_eq!(ids, ["26.4-snapshot-7"]);
    }

    #[test]
    fn keeps_only_releases_down_to_the_oldest_supported() {
        let manifest = VersionManifest::parse(SAMPLE).unwrap();
        let ids: Vec<&str> = manifest
            .supported_releases()
            .unwrap()
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(ids, ["26.3", "1.21.11", "1.7.10"]);
    }

    #[test]
    fn unknown_version_types_do_not_break_parsing() {
        let manifest = VersionManifest::parse(SAMPLE).unwrap();
        assert_eq!(manifest.versions[5].kind, VersionKind::Other);
    }

    #[test]
    fn reports_when_oldest_supported_is_missing() {
        let json = r#"{
            "latest": { "release": "26.3", "snapshot": "26.3" },
            "versions": [{ "id": "26.3", "type": "release", "url": "u", "sha1": "s" }]
        }"#;
        let manifest = VersionManifest::parse(json).unwrap();
        assert!(matches!(
            manifest.supported_releases(),
            Err(VersionsError::MissingOldestSupported)
        ));
    }
}
