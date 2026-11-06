//! Fabric 和 Quilt：兩邊的伺服器格式相同，只有網址和名稱不一樣。

use serde::Deserialize;
use serde_json::Value;

use crate::download::fetch_text;

use super::LoaderError;
use super::profile::maven_path;

/// 一種 Fabric 系的載入器。
#[derive(Debug, Clone, Copy)]
pub struct Flavor {
    meta_url: &'static str,
    name: &'static str,
    id_prefix: &'static str,
}

pub const FABRIC: Flavor = Flavor {
    meta_url: "https://meta.fabricmc.net/v2/versions/loader",
    name: "Fabric",
    id_prefix: "fabric-loader",
};

pub const QUILT: Flavor = Flavor {
    meta_url: "https://meta.quiltmc.org/v3/versions/loader",
    name: "Quilt",
    id_prefix: "quilt-loader",
};

#[derive(Debug, Deserialize)]
struct LoaderEntry {
    loader: LoaderVersion,
}

#[derive(Debug, Deserialize)]
struct LoaderVersion {
    version: String,
    /// Quilt 沒有這個欄位，測試版的版本號裡有 `-`。
    stable: Option<bool>,
}

impl LoaderVersion {
    fn is_stable(&self) -> bool {
        self.stable.unwrap_or(!self.version.contains('-'))
    }

    /// 用來比較新舊。Quilt 的清單不是照版本排序的。
    fn order(&self) -> Vec<u32> {
        self.version
            .split('.')
            .map(|part| part.parse().unwrap_or(0))
            .collect()
    }
}

/// 本機保存設定檔用的名稱，和官方安裝程式相同。
pub fn profile_id(flavor: Flavor, loader_version: &str, game_version: &str) -> String {
    format!("{}-{loader_version}-{game_version}", flavor.id_prefix)
}

pub async fn latest_version(
    client: &reqwest::Client,
    flavor: Flavor,
    game_version: &str,
) -> Result<String, LoaderError> {
    let text = fetch_text(client, &format!("{}/{game_version}", flavor.meta_url)).await?;
    let entries: Vec<LoaderEntry> = serde_json::from_str(&text)?;
    newest_stable(entries).ok_or_else(|| LoaderError::Unsupported {
        loader: flavor.name,
        version: game_version.to_string(),
    })
}

fn newest_stable(entries: Vec<LoaderEntry>) -> Option<String> {
    entries
        .into_iter()
        .map(|entry| entry.loader)
        .filter(LoaderVersion::is_stable)
        .max_by_key(LoaderVersion::order)
        .map(|loader| loader.version)
}

/// 下載設定檔，並補齊缺少的 SHA1。
pub async fn fetch_profile(
    client: &reqwest::Client,
    flavor: Flavor,
    loader_version: &str,
    game_version: &str,
) -> Result<Value, LoaderError> {
    let url = format!(
        "{}/{game_version}/{loader_version}/profile/json",
        flavor.meta_url
    );
    let mut profile: Value = serde_json::from_str(&fetch_text(client, &url).await?)?;
    if let Some(libraries) = profile["libraries"].as_array_mut() {
        for library in libraries {
            if library.get("sha1").is_none() {
                let sha1 = fetch_sha1(client, library).await?;
                library["sha1"] = Value::String(sha1);
            }
        }
    }
    Ok(profile)
}

/// `fabric-loader` 和 `intermediary` 的設定檔沒有 SHA1，要到 Maven 倉庫的 `.sha1` 檔案拿。
async fn fetch_sha1(client: &reqwest::Client, library: &Value) -> Result<String, LoaderError> {
    let name = library["name"].as_str().unwrap_or_default();
    let bad = || LoaderError::BadLibrary {
        name: name.to_string(),
    };
    let path = maven_path(name).ok_or_else(bad)?;
    let base = library["url"].as_str().ok_or_else(bad)?;
    let url = format!("{}/{path}.sha1", base.trim_end_matches('/'));
    let text = fetch_text(client, &url).await?;
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(json: &str) -> Vec<LoaderEntry> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn quilt_has_no_stable_flag_and_is_not_sorted() {
        let list = entries(
            r#"[{ "loader": { "version": "0.20.0-beta.9" } },
                { "loader": { "version": "0.24.0" } },
                { "loader": { "version": "0.29.1" } },
                { "loader": { "version": "0.3.0" } }]"#,
        );

        assert_eq!(newest_stable(list).as_deref(), Some("0.29.1"));
    }

    #[test]
    fn picks_the_newest_stable_loader() {
        let list = entries(
            r#"[
                { "loader": { "version": "0.21.0-beta", "stable": false } },
                { "loader": { "version": "0.20.1", "stable": true } },
                { "loader": { "version": "0.20.0", "stable": true } }
            ]"#,
        );

        assert_eq!(newest_stable(list).as_deref(), Some("0.20.1"));
    }

    #[test]
    fn an_unsupported_game_version_has_no_loader() {
        assert_eq!(newest_stable(entries("[]")), None);
    }

    #[test]
    fn profile_name_matches_the_official_installer() {
        assert_eq!(
            profile_id(FABRIC, "0.20.1", "1.21.8"),
            "fabric-loader-0.20.1-1.21.8"
        );
    }
}
