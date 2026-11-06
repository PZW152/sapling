//! Modrinth：搜尋和下載模組。所有和 Modrinth 有關的網址和資料格式都在這裡。

pub mod pack;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use reqwest::Url;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::download::{DownloadError, DownloadTask, download_all, fetch_text};
use crate::instance::InstanceError;
use crate::loader::LoaderKind;
use crate::versions::VersionsError;

const API_URL: &str = "https://api.modrinth.com/v2";
const SEARCH_LIMIT: usize = 20;
const REQUIRED: &str = "required";
const RELEASE: &str = "release";

#[derive(Debug, Error)]
pub enum ModrinthError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error("Modrinth 回傳的資料看不懂，可能是對方改了格式")]
    Parse(#[from] serde_json::Error),
    #[error("模組搜尋的網址有問題")]
    BadUrl,
    #[error("{project} 沒有適合這個遊戲版本和載入器的檔案")]
    NoCompatibleVersion { project: String },
    #[error("原版不能安裝模組，請新增一個 Fabric 實例")]
    NoLoader,
    #[error("這個整合包的內容看不懂，可能已損壞")]
    BadPack,
    #[error("這個整合包使用 {loader}，Sapling 還不支援")]
    UnsupportedLoader { loader: &'static str },
    #[error("這個整合包使用 Minecraft {version}，Sapling 還不支援這麼舊的版本")]
    OldGameVersion { version: String },
    #[error("整合包裡有不安全的路徑，已停止安裝：{path}")]
    UnsafePath { path: String },
    #[error("無法寫入檔案：{}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Instance(#[from] InstanceError),
    #[error(transparent)]
    Versions(#[from] VersionsError),
    #[error("這個模組的檔案名稱有問題：{name}")]
    BadFileName { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct ModHit {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    hits: Vec<ModHit>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModVersion {
    id: String,
    project_id: String,
    version_type: String,
    files: Vec<ModVersionFile>,
    #[serde(default)]
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModVersionFile {
    url: String,
    filename: String,
    size: u64,
    #[serde(default)]
    primary: bool,
    hashes: FileHashes,
}

#[derive(Debug, Clone, Deserialize)]
struct FileHashes {
    sha1: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Dependency {
    project_id: Option<String>,
    version_id: Option<String>,
    dependency_type: String,
}

fn loader_name(loader: LoaderKind) -> Result<&'static str, ModrinthError> {
    match loader {
        LoaderKind::Vanilla => Err(ModrinthError::NoLoader),
        LoaderKind::Fabric => Ok("fabric"),
    }
}

/// 搜尋適合這個遊戲版本和載入器的模組。沒有關鍵字時列出最熱門的。
pub async fn search(
    client: &reqwest::Client,
    query: &str,
    game_version: &str,
    loader: LoaderKind,
) -> Result<Vec<ModHit>, ModrinthError> {
    let facets = serde_json::json!([
        ["project_type:mod"],
        [format!("versions:{game_version}")],
        [format!("categories:{}", loader_name(loader)?)],
    ])
    .to_string();
    search_hits(client, query, &facets).await
}

/// 用 Modrinth 的篩選條件搜尋。
async fn search_hits(
    client: &reqwest::Client,
    query: &str,
    facets: &str,
) -> Result<Vec<ModHit>, ModrinthError> {
    let limit = SEARCH_LIMIT.to_string();
    let index = if query.trim().is_empty() {
        "downloads"
    } else {
        "relevance"
    };
    let url = Url::parse_with_params(
        &format!("{API_URL}/search"),
        [
            ("query", query),
            ("facets", facets),
            ("index", index),
            ("limit", limit.as_str()),
        ],
    )
    .map_err(|_| ModrinthError::BadUrl)?;
    let result: SearchResult = serde_json::from_str(&fetch_text(client, url.as_str()).await?)?;
    Ok(result.hits)
}

/// 下載模組和它必需的前置模組到 `mods_dir`，回傳下載的檔案名稱。
/// 全部找到相容版本後才開始下載，不會只裝一半。
pub async fn install(
    client: &reqwest::Client,
    project_id: &str,
    game_version: &str,
    loader: LoaderKind,
    mods_dir: &Path,
    concurrency: usize,
) -> Result<Vec<String>, ModrinthError> {
    let loader = loader_name(loader)?;
    let mut pending = vec![Wanted::Project(project_id.to_string())];
    let mut seen_projects = HashSet::new();
    let mut files = Vec::new();

    while let Some(wanted) = pending.pop() {
        let version = match wanted {
            Wanted::Project(project) => {
                if !seen_projects.insert(project.clone()) {
                    continue;
                }
                compatible_version(client, &project, game_version, loader).await?
            }
            Wanted::Version(id) => {
                let version = fetch_version(client, &id).await?;
                if !seen_projects.insert(version.project_id.clone()) {
                    continue;
                }
                version
            }
        };
        pending.extend(required_dependencies(&version));
        files.push(primary_file(&version)?);
    }

    let names = files.iter().map(|file| file.filename.clone()).collect();
    let tasks = files
        .into_iter()
        .map(|file| {
            Ok(DownloadTask {
                path: mods_dir.join(safe_file_name(&file.filename)?),
                url: file.url,
                sha1: file.hashes.sha1,
                size: file.size,
            })
        })
        .collect::<Result<Vec<_>, ModrinthError>>()?;
    download_all(client, tasks, concurrency, |_| {}).await?;
    Ok(names)
}

enum Wanted {
    Project(String),
    Version(String),
}

fn required_dependencies(version: &ModVersion) -> Vec<Wanted> {
    version
        .dependencies
        .iter()
        .filter(|dependency| dependency.dependency_type == REQUIRED)
        .filter_map(|dependency| {
            // 指定版本時要用那個版本，否則另外找相容的版本。
            dependency
                .version_id
                .clone()
                .map(Wanted::Version)
                .or_else(|| dependency.project_id.clone().map(Wanted::Project))
        })
        .collect()
}

async fn compatible_version(
    client: &reqwest::Client,
    project: &str,
    game_version: &str,
    loader: &str,
) -> Result<ModVersion, ModrinthError> {
    let game_versions = serde_json::json!([game_version]).to_string();
    let loaders = serde_json::json!([loader]).to_string();
    let url = Url::parse_with_params(
        &format!("{API_URL}/project/{project}/version"),
        [
            ("game_versions", game_versions.as_str()),
            ("loaders", loaders.as_str()),
        ],
    )
    .map_err(|_| ModrinthError::BadUrl)?;
    let versions: Vec<ModVersion> = serde_json::from_str(&fetch_text(client, url.as_str()).await?)?;
    pick_version(versions).ok_or_else(|| ModrinthError::NoCompatibleVersion {
        project: project.to_string(),
    })
}

async fn fetch_version(client: &reqwest::Client, id: &str) -> Result<ModVersion, ModrinthError> {
    let url = format!("{API_URL}/version/{id}");
    Ok(serde_json::from_str(&fetch_text(client, &url).await?)?)
}

/// 清單由新到舊排列。優先用正式版，沒有才用測試版。
fn pick_version(versions: Vec<ModVersion>) -> Option<ModVersion> {
    let release = versions
        .iter()
        .find(|version| version.version_type == RELEASE)
        .cloned();
    release.or_else(|| versions.into_iter().next())
}

fn primary_file(version: &ModVersion) -> Result<ModVersionFile, ModrinthError> {
    version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .cloned()
        .ok_or_else(|| ModrinthError::NoCompatibleVersion {
            project: version.id.clone(),
        })
}

/// 檔案名稱來自網路，不能讓它指到模組資料夾以外的地方。
fn safe_file_name(name: &str) -> Result<&str, ModrinthError> {
    let safe = name.ends_with(".jar") && !name.contains(['/', '\\']) && !name.contains("..");
    if safe {
        Ok(name)
    } else {
        Err(ModrinthError::BadFileName {
            name: name.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(kind: &str, id: &str) -> ModVersion {
        serde_json::from_str(&format!(
            r#"{{
                "id": "{id}", "project_id": "p", "version_type": "{kind}",
                "files": [
                    {{ "url": "u1", "filename": "extra.jar", "size": 1, "primary": false, "hashes": {{ "sha1": "a" }} }},
                    {{ "url": "u2", "filename": "main.jar", "size": 2, "primary": true, "hashes": {{ "sha1": "b" }} }}
                ],
                "dependencies": [
                    {{ "project_id": "need", "version_id": null, "dependency_type": "required" }},
                    {{ "project_id": "pinned", "version_id": "v9", "dependency_type": "required" }},
                    {{ "project_id": "nice", "version_id": null, "dependency_type": "optional" }}
                ]
            }}"#
        ))
        .unwrap()
    }

    #[test]
    fn reads_the_fields_of_a_real_search_hit() {
        let result: SearchResult = serde_json::from_str(
            r#"{ "hits": [{ "project_id": "AANobbMI", "slug": "sodium", "title": "Sodium",
                 "description": "快", "downloads": 5, "icon_url": null, "client_side": "required" }] }"#,
        )
        .unwrap();

        assert_eq!(result.hits[0].project_id, "AANobbMI");
        assert_eq!(result.hits[0].icon_url, None);
    }

    #[test]
    fn prefers_a_release_over_a_newer_beta() {
        let picked = pick_version(vec![version("beta", "new"), version("release", "old")]);

        assert_eq!(picked.unwrap().id, "old");
    }

    #[test]
    fn falls_back_to_a_beta_when_nothing_else_exists() {
        let picked = pick_version(vec![version("beta", "only")]);

        assert_eq!(picked.unwrap().id, "only");
    }

    #[test]
    fn uses_the_primary_file() {
        assert_eq!(
            primary_file(&version("release", "x")).unwrap().filename,
            "main.jar"
        );
    }

    #[test]
    fn follows_only_required_dependencies() {
        let wanted = required_dependencies(&version("release", "x"));

        assert_eq!(wanted.len(), 2);
        assert!(matches!(&wanted[0], Wanted::Project(id) if id == "need"));
        assert!(matches!(&wanted[1], Wanted::Version(id) if id == "v9"));
    }

    #[test]
    fn vanilla_cannot_use_mods() {
        assert!(matches!(
            loader_name(LoaderKind::Vanilla),
            Err(ModrinthError::NoLoader)
        ));
    }

    #[test]
    fn refuses_file_names_that_leave_the_mods_folder() {
        assert!(safe_file_name("sodium-0.7+mc1.21.jar").is_ok());
        assert!(safe_file_name("../evil.jar").is_err());
        assert!(safe_file_name("a/b.jar").is_err());
        assert!(safe_file_name("a\\b.jar").is_err());
        assert!(safe_file_name("readme.txt").is_err());
    }
}
