//! Forge 和 NeoForge：版本查詢和下載網址都在這裡，安裝流程共用 `installer`。

use std::collections::HashMap;

use serde::Deserialize;

use crate::download::fetch_text;

use super::LoaderError;
use super::installer::InstallRequest;

const FORGE_NAME: &str = "Forge";
const FORGE_PROMOTIONS: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const NEOFORGE_NAME: &str = "NeoForge";
const NEOFORGE_VERSIONS: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const BETA_MARK: &str = "beta";

#[derive(Debug, Deserialize)]
struct Promotions {
    promos: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct NeoForgeVersions {
    versions: Vec<String>,
}

pub async fn latest_forge(
    client: &reqwest::Client,
    game_version: &str,
) -> Result<String, LoaderError> {
    let promotions: Promotions =
        serde_json::from_str(&fetch_text(client, FORGE_PROMOTIONS).await?)?;
    pick_forge(&promotions.promos, game_version).ok_or_else(|| LoaderError::Unsupported {
        loader: FORGE_NAME,
        version: game_version.to_string(),
    })
}

/// 優先用官方推薦的版本，沒有才用最新版。
fn pick_forge(promotions: &HashMap<String, String>, game_version: &str) -> Option<String> {
    promotions
        .get(&format!("{game_version}-recommended"))
        .or_else(|| promotions.get(&format!("{game_version}-latest")))
        .cloned()
}

pub async fn latest_neoforge(
    client: &reqwest::Client,
    game_version: &str,
) -> Result<String, LoaderError> {
    let list: NeoForgeVersions =
        serde_json::from_str(&fetch_text(client, NEOFORGE_VERSIONS).await?)?;
    pick_neoforge(&list.versions, game_version).ok_or_else(|| LoaderError::Unsupported {
        loader: NEOFORGE_NAME,
        version: game_version.to_string(),
    })
}

/// 清單由舊到新排列。優先用正式版，新版本剛出時可能只有測試版。
fn pick_neoforge(versions: &[String], game_version: &str) -> Option<String> {
    let prefix = neoforge_prefix(game_version);
    let matching = versions
        .iter()
        .filter(|version| version.starts_with(&prefix));
    let newest_stable = matching
        .clone()
        .rfind(|version| !version.contains(BETA_MARK));
    newest_stable.or_else(|| matching.clone().next_back()).cloned()
}

/// NeoForge 的版本號開頭對應遊戲版本：`1.21.8` 是 `21.8.`，`1.21` 是 `21.0.`，`26.3` 是 `26.3.`。
fn neoforge_prefix(game_version: &str) -> String {
    match game_version.strip_prefix("1.") {
        Some(rest) if rest.contains('.') => format!("{rest}."),
        Some(rest) => format!("{rest}.0."),
        None => format!("{game_version}."),
    }
}

pub fn forge_request(loader_version: &str, game_version: &str) -> InstallRequest {
    let full = format!("{game_version}-{loader_version}");
    InstallRequest {
        id: format!("{game_version}-forge-{loader_version}"),
        installer_url: format!("{FORGE_MAVEN}/{full}/forge-{full}-installer.jar"),
        game_version: game_version.to_string(),
    }
}

pub fn neoforge_request(loader_version: &str, game_version: &str) -> InstallRequest {
    InstallRequest {
        id: format!("neoforge-{loader_version}"),
        installer_url: format!(
            "{NEOFORGE_MAVEN}/{loader_version}/neoforge-{loader_version}-installer.jar"
        ),
        game_version: game_version.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn versions(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn prefers_the_recommended_forge() {
        let promotions = HashMap::from([
            ("26.3-latest".to_string(), "66.1.5".to_string()),
            ("26.3-recommended".to_string(), "66.1.0".to_string()),
            ("26.2-latest".to_string(), "65.1.6".to_string()),
        ]);

        assert_eq!(pick_forge(&promotions, "26.3").as_deref(), Some("66.1.0"));
        assert_eq!(pick_forge(&promotions, "26.2").as_deref(), Some("65.1.6"));
        assert_eq!(pick_forge(&promotions, "1.19"), None);
    }

    #[test]
    fn maps_game_versions_to_neoforge_prefixes() {
        assert_eq!(neoforge_prefix("1.21.8"), "21.8.");
        assert_eq!(neoforge_prefix("1.21"), "21.0.");
        assert_eq!(neoforge_prefix("26.3"), "26.3.");
    }

    #[test]
    fn picks_the_newest_stable_neoforge() {
        let list = versions(&[
            "21.8.1-beta",
            "21.8.54",
            "21.8.57",
            "21.8.58-beta",
            "21.9.1",
        ]);

        assert_eq!(pick_neoforge(&list, "1.21.8").as_deref(), Some("21.8.57"));
    }

    #[test]
    fn falls_back_to_a_beta_for_a_brand_new_version() {
        let list = versions(&["26.3.0.1-beta", "26.3.0.2-beta"]);

        assert_eq!(
            pick_neoforge(&list, "26.3").as_deref(),
            Some("26.3.0.2-beta")
        );
    }

    #[test]
    fn does_not_confuse_similar_version_numbers() {
        let list = versions(&["21.10.3", "21.1.5"]);

        assert_eq!(pick_neoforge(&list, "1.21.1").as_deref(), Some("21.1.5"));
    }

    #[test]
    fn builds_the_official_installer_addresses() {
        let forge = forge_request("66.1.5", "26.3");
        assert_eq!(forge.id, "26.3-forge-66.1.5");
        assert_eq!(
            forge.installer_url,
            "https://maven.minecraftforge.net/net/minecraftforge/forge/26.3-66.1.5/forge-26.3-66.1.5-installer.jar"
        );

        let neoforge = neoforge_request("26.3.0.101", "26.3");
        assert_eq!(neoforge.id, "neoforge-26.3.0.101");
        assert!(
            neoforge
                .installer_url
                .ends_with("neoforge-26.3.0.101-installer.jar")
        );
    }
}
