//! 版本清單：從 Mojang 取得所有 Minecraft 版本，並篩選出 v1 支援的正式版。

pub mod details;
pub mod install;
pub mod rules;

use serde::Deserialize;
use thiserror::Error;

use crate::download::{DownloadError, fetch_text};
use details::VersionDetails;

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// v1 支援的最舊正式版。
///
/// Mojang 在 2026 年改用 `26.1` 這類版本號，所以不比較版本號數字，
/// 而是靠清單順序（最新的排最前）判斷新舊。
const OLDEST_SUPPORTED_RELEASE: &str = "1.19";

#[derive(Debug, Error)]
pub enum VersionsError {
    #[error("無法取得版本資料，請檢查網絡連線後再試一次")]
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
            { "id": "1.19", "type": "release", "url": "u", "sha1": "s", "complianceLevel": 1 },
            { "id": "1.18.2", "type": "release", "url": "u", "sha1": "s", "complianceLevel": 1 },
            { "id": "b1.7.3", "type": "old_beta", "url": "u", "sha1": "s", "complianceLevel": 0 }
        ]
    }"#;

    #[test]
    fn keeps_only_releases_from_newest_down_to_1_19() {
        let manifest = VersionManifest::parse(SAMPLE).unwrap();
        let ids: Vec<&str> = manifest
            .supported_releases()
            .unwrap()
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(ids, ["26.3", "1.21.11", "1.19"]);
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
