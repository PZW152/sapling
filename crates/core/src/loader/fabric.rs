//! Fabric：所有和 Fabric 伺服器有關的網址和資料格式都在這裡。

use serde::Deserialize;
use serde_json::Value;

use crate::download::fetch_text;

use super::LoaderError;
use super::profile::maven_path;

const META_URL: &str = "https://meta.fabricmc.net/v2/versions/loader";
const NAME: &str = "Fabric";

#[derive(Debug, Deserialize)]
struct LoaderEntry {
    loader: LoaderVersion,
}

#[derive(Debug, Deserialize)]
struct LoaderVersion {
    version: String,
    stable: bool,
}

/// 本機保存設定檔用的名稱，和官方安裝程式相同。
pub fn profile_id(loader_version: &str, game_version: &str) -> String {
    format!("fabric-loader-{loader_version}-{game_version}")
}

pub async fn latest_version(
    client: &reqwest::Client,
    game_version: &str,
) -> Result<String, LoaderError> {
    let text = fetch_text(client, &format!("{META_URL}/{game_version}")).await?;
    let entries: Vec<LoaderEntry> = serde_json::from_str(&text)?;
    newest_stable(entries).ok_or_else(|| LoaderError::Unsupported {
        loader: NAME,
        version: game_version.to_string(),
    })
}

/// 清單由新到舊排列。
fn newest_stable(entries: Vec<LoaderEntry>) -> Option<String> {
    entries
        .into_iter()
        .find(|entry| entry.loader.stable)
        .map(|entry| entry.loader.version)
}

/// 下載設定檔，並補齊缺少的 SHA1。
pub async fn fetch_profile(
    client: &reqwest::Client,
    loader_version: &str,
    game_version: &str,
) -> Result<Value, LoaderError> {
    let url = format!("{META_URL}/{game_version}/{loader_version}/profile/json");
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
            profile_id("0.20.1", "1.21.8"),
            "fabric-loader-0.20.1-1.21.8"
        );
    }
}
