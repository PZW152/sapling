//! 已經裝好的模組：用檔案的 SHA1 問 Modrinth 它是什麼、有沒有新版。
//! 手動放進來的模組查不到，介面就照常顯示檔名。

use std::collections::HashMap;
use std::path::Path;

use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use super::{API_URL, ModVersion, ModrinthError, RELEASE, loader_names};
use crate::download::{fetch_text, file_sha1};
use crate::instance::mods::{self, ModFile};
use crate::loader::LoaderKind;

/// 一次最多查幾個專案，網址才不會太長。
const PROJECTS_PER_REQUEST: usize = 100;
const HASH_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    /// 不含停用後綴的檔案名。
    pub file_name: String,
    pub project_id: String,
    pub title: String,
    pub slug: String,
    pub icon_url: Option<String>,
    pub update: Option<ModUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModUpdate {
    pub version_id: String,
    pub version_name: String,
}

#[derive(Debug, Deserialize)]
struct Project {
    id: String,
    slug: String,
    title: String,
    icon_url: Option<String>,
}

/// 查出實例裡每個模組的名稱和新版。查不到的模組不會出現在結果裡。
pub async fn inspect(
    client: &reqwest::Client,
    game_dir: &Path,
    game_version: &str,
    loader: LoaderKind,
) -> Result<Vec<InstalledMod>, ModrinthError> {
    let loaders = loader_names(loader)?;
    let files = mods::list(game_dir).await?;
    let hashed = hash_all(game_dir, files).await;
    if hashed.is_empty() {
        return Ok(Vec::new());
    }
    let hashes: Vec<&str> = hashed.iter().map(|(_, hash)| hash.as_str()).collect();

    let current: HashMap<String, ModVersion> = post(
        client,
        "version_files",
        &serde_json::json!({ "hashes": hashes, "algorithm": "sha1" }),
    )
    .await?;
    let latest: HashMap<String, ModVersion> = post(
        client,
        "version_files/update",
        &serde_json::json!({
            "hashes": hashes,
            "algorithm": "sha1",
            "loaders": loaders,
            "game_versions": [game_version],
        }),
    )
    .await?;
    let mut ids: Vec<String> = current
        .values()
        .map(|version| version.project_id.clone())
        .collect();
    ids.sort();
    ids.dedup();
    let projects = fetch_projects(client, &ids).await?;

    Ok(hashed
        .iter()
        .filter_map(|(file, hash)| {
            let version = current.get(hash)?;
            let project = projects.get(&version.project_id)?;
            Some(InstalledMod {
                file_name: file.name.clone(),
                project_id: project.id.clone(),
                title: project.title.clone(),
                slug: project.slug.clone(),
                icon_url: project.icon_url.clone(),
                // 停用的模組是用戶刻意留著的，不提示更新。
                update: latest
                    .get(hash)
                    .filter(|_| file.enabled)
                    .and_then(|newest| available_update(version, newest)),
            })
        })
        .collect())
}

/// 同時算幾個模組的雜湊。模組多的整合包，一個一個算要等很久。
async fn hash_all(game_dir: &Path, files: Vec<ModFile>) -> Vec<(ModFile, String)> {
    let folder = mods::mods_dir(game_dir);
    let mut pending = files.into_iter();
    let mut running = JoinSet::new();
    let mut hashed = Vec::new();
    loop {
        while running.len() < HASH_CONCURRENCY {
            let Some(file) = pending.next() else {
                break;
            };
            let name = if file.enabled {
                file.name.clone()
            } else {
                format!("{}.disabled", file.name)
            };
            let path = folder.join(name);
            running.spawn(async move { (file, file_sha1(&path).await) });
        }
        let Some(finished) = running.join_next().await else {
            // 完成的順序不固定，排回去讓結果穩定。
            hashed.sort_by(|a: &(ModFile, String), b| a.0.name.cmp(&b.0.name));
            return hashed;
        };
        if let Ok((file, Ok(Some(hash)))) = finished {
            hashed.push((file, hash));
        }
    }
}

/// 只提示真的更新的版本：用戶裝的是正式版時不換成測試版，也不換成比較舊的版本。
fn available_update(current: &ModVersion, newest: &ModVersion) -> Option<ModUpdate> {
    let same = newest.id == current.id;
    let older = newest.date_published <= current.date_published;
    let less_stable = newest.version_type != RELEASE && current.version_type == RELEASE;
    (!same && !older && !less_stable).then(|| ModUpdate {
        version_id: newest.id.clone(),
        version_name: newest.name.clone(),
    })
}

async fn fetch_projects(
    client: &reqwest::Client,
    ids: &[String],
) -> Result<HashMap<String, Project>, ModrinthError> {
    let mut projects = HashMap::new();
    for chunk in ids.chunks(PROJECTS_PER_REQUEST) {
        let list = serde_json::json!(chunk).to_string();
        let url = Url::parse_with_params(&format!("{API_URL}/projects"), [("ids", list)])
            .map_err(|_| ModrinthError::BadUrl)?;
        let found: Vec<Project> = serde_json::from_str(&fetch_text(client, url.as_str()).await?)?;
        projects.extend(
            found
                .into_iter()
                .map(|project| (project.id.clone(), project)),
        );
    }
    Ok(projects)
}

pub(super) async fn post<T: DeserializeOwned>(
    client: &reqwest::Client,
    path: &str,
    body: &serde_json::Value,
) -> Result<T, ModrinthError> {
    let response = client
        .post(format!("{API_URL}/{path}"))
        .json(body)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| ModrinthError::Lookup)?;
    response.json().await.map_err(|_| ModrinthError::Lookup)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(id: &str, version_type: &str, date: &str) -> ModVersion {
        ModVersion {
            id: id.to_string(),
            project_id: "p".to_string(),
            version_type: version_type.to_string(),
            files: Vec::new(),
            dependencies: Vec::new(),
            name: id.to_string(),
            game_versions: Vec::new(),
            loaders: Vec::new(),
            date_published: date.to_string(),
        }
    }

    #[test]
    fn offers_a_newer_release() {
        let current = version("a", "release", "2026-01-01T00:00:00Z");
        let newest = version("b", "release", "2026-02-01T00:00:00Z");

        let update = available_update(&current, &newest).unwrap();

        assert_eq!(update.version_id, "b");
    }

    #[test]
    fn stays_on_the_same_version() {
        let current = version("a", "release", "2026-01-01T00:00:00Z");

        assert_eq!(available_update(&current, &current), None);
    }

    #[test]
    fn never_moves_a_release_to_a_beta() {
        let current = version("a", "release", "2026-01-01T00:00:00Z");
        let newest = version("b", "beta", "2026-02-01T00:00:00Z");

        assert_eq!(available_update(&current, &newest), None);
    }

    #[test]
    fn a_beta_can_move_to_a_newer_beta() {
        let current = version("a", "beta", "2026-01-01T00:00:00Z");
        let newest = version("b", "beta", "2026-02-01T00:00:00Z");

        assert!(available_update(&current, &newest).is_some());
    }

    #[test]
    fn never_downgrades() {
        let current = version("a", "release", "2026-03-01T00:00:00Z");
        let newest = version("b", "release", "2026-02-01T00:00:00Z");

        assert_eq!(available_update(&current, &newest), None);
    }
}
