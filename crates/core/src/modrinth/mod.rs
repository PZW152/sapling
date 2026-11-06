//! Modrinth：搜尋和下載模組、資源包、光影和整合包。所有和 Modrinth 有關的網址和資料格式都在這裡。

pub mod export;
pub mod installed;
pub mod pack;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use reqwest::Url;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::download::{DownloadError, DownloadTask, download_all, fetch_text, file_sha1};
use crate::instance::InstanceError;
use crate::instance::mods::link_existing;
use crate::instance::worlds;
use crate::loader::LoaderKind;
use crate::versions::VersionsError;

const API_URL: &str = "https://api.modrinth.com/v2";
const SEARCH_LIMIT: usize = 20;
const REQUIRED: &str = "required";
const RELEASE: &str = "release";
/// 排序方式，其他值一律不接受。
const SORTS: [&str; 4] = ["relevance", "downloads", "newest", "updated"];
/// 整合包目前支援的載入器。
const PACK_LOADERS: [&str; 3] = ["fabric", "forge", "neoforge"];
/// 同一個專案可能同時有模組和資料包版本，要指定資料包。
const DATAPACK_LOADER: &str = "datapack";

#[derive(Debug, Error)]
pub enum ModrinthError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error("Modrinth 回傳的資料看不懂，可能是對方改了格式")]
    Parse(#[from] serde_json::Error),
    #[error("模組搜尋的網址有問題")]
    BadUrl,
    #[error("無法查詢模組資料，請檢查網路連線後再試一次")]
    Lookup,
    #[error("{project} 沒有適合這個遊戲版本和載入器的檔案")]
    NoCompatibleVersion { project: String },
    #[error("原版不能安裝模組，請新增一個使用載入器的實例")]
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
    #[error("請先選擇要安裝到哪個世界")]
    NoWorld,
    #[error("這個模組的檔案名稱有問題：{name}")]
    BadFileName { name: String },
}

/// 可以下載的資源種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectKind {
    Mod,
    Modpack,
    ResourcePack,
    Shader,
    DataPack,
}

impl ProjectKind {
    fn facet(self) -> &'static str {
        match self {
            ProjectKind::Mod => "mod",
            ProjectKind::Modpack => "modpack",
            ProjectKind::ResourcePack => "resourcepack",
            ProjectKind::Shader => "shader",
            ProjectKind::DataPack => "datapack",
        }
    }

    /// 這種資源放在實例的哪裡。資料包屬於單一世界，整合包會建立新實例。
    fn folder(self, game_dir: &Path, world: Option<&str>) -> Result<PathBuf, ModrinthError> {
        Ok(match self {
            ProjectKind::Mod | ProjectKind::Modpack => game_dir.join("mods"),
            ProjectKind::ResourcePack => game_dir.join("resourcepacks"),
            ProjectKind::Shader => game_dir.join("shaderpacks"),
            ProjectKind::DataPack => {
                let world = world
                    .filter(|folder| worlds::exists(game_dir, folder))
                    .ok_or(ModrinthError::NoWorld)?;
                game_dir.join("saves").join(world).join("datapacks")
            }
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub kind: ProjectKind,
    pub game_version: Option<String>,
    pub loader: Option<LoaderKind>,
    pub category: Option<String>,
    pub sort: Option<String>,
    #[serde(default)]
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct ModHit {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub date_modified: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchPage {
    pub hits: Vec<ModHit>,
    #[serde(rename(deserialize = "total_hits"))]
    pub total: u64,
}

/// 顯示在詳情頁的一個版本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectVersion {
    pub id: String,
    pub name: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub version_type: String,
    pub date_published: String,
    pub file_name: String,
    pub size: u64,
}

/// 要安裝哪一份檔案。
pub enum Wanted {
    /// 這個專案裡最適合的版本。
    Project(String),
    /// 指定的版本。
    Version(String),
}

/// 裝到哪裡。
pub struct InstallTarget<'a> {
    pub game_version: &'a str,
    pub loader: LoaderKind,
    pub game_dir: &'a Path,
    /// 資料包要裝進的世界。
    pub world: Option<&'a str>,
    pub other_mods_dirs: &'a [PathBuf],
    pub concurrency: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct ModVersion {
    id: String,
    project_id: String,
    version_type: String,
    files: Vec<ModVersionFile>,
    #[serde(default)]
    dependencies: Vec<Dependency>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    date_published: String,
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
    #[serde(default)]
    sha512: String,
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
        LoaderKind::Forge => Ok("forge"),
        LoaderKind::NeoForge => Ok("neoforge"),
    }
}

/// 搜尋資源。沒有關鍵字時列出最熱門的。
pub async fn search(
    client: &reqwest::Client,
    request: &SearchRequest,
) -> Result<SearchPage, ModrinthError> {
    let facets = facets(request)?;
    let limit = SEARCH_LIMIT.to_string();
    let offset = request.offset.to_string();
    let url = Url::parse_with_params(
        &format!("{API_URL}/search"),
        [
            ("query", request.query.as_str()),
            ("facets", facets.as_str()),
            ("index", sort_index(request)),
            ("limit", limit.as_str()),
            ("offset", offset.as_str()),
        ],
    )
    .map_err(|_| ModrinthError::BadUrl)?;
    Ok(serde_json::from_str(
        &fetch_text(client, url.as_str()).await?,
    )?)
}

/// 同一組裡的條件是「或」，組和組之間是「且」。
fn facets(request: &SearchRequest) -> Result<String, ModrinthError> {
    let mut groups = vec![vec![format!("project_type:{}", request.kind.facet())]];
    if let Some(version) = &request.game_version {
        groups.push(vec![format!("versions:{version}")]);
    }
    match (request.kind, request.loader) {
        (ProjectKind::Mod, Some(loader)) => {
            groups.push(vec![format!("categories:{}", loader_name(loader)?)]);
        }
        (ProjectKind::Modpack, _) => {
            groups.push(
                PACK_LOADERS
                    .iter()
                    .map(|loader| format!("categories:{loader}"))
                    .collect(),
            );
        }
        _ => {}
    }
    if let Some(category) = &request.category {
        groups.push(vec![format!("categories:{category}")]);
    }
    Ok(serde_json::to_string(&groups)?)
}

fn sort_index(request: &SearchRequest) -> &str {
    let chosen = request.sort.as_deref().filter(|sort| SORTS.contains(sort));
    chosen.unwrap_or(if request.query.trim().is_empty() {
        "downloads"
    } else {
        "relevance"
    })
}

/// 一個專案的所有版本，新的在前。
pub async fn project_versions(
    client: &reqwest::Client,
    project_id: &str,
) -> Result<Vec<ProjectVersion>, ModrinthError> {
    let url = format!("{API_URL}/project/{project_id}/version");
    let versions: Vec<ModVersion> = serde_json::from_str(&fetch_text(client, &url).await?)?;
    Ok(versions.iter().filter_map(summary).collect())
}

fn summary(version: &ModVersion) -> Option<ProjectVersion> {
    let file = primary_file(version).ok()?;
    Some(ProjectVersion {
        id: version.id.clone(),
        name: version.name.clone(),
        game_versions: version.game_versions.clone(),
        loaders: version.loaders.clone(),
        version_type: version.version_type.clone(),
        date_published: version.date_published.clone(),
        file_name: file.filename,
        size: file.size,
    })
}

/// 下載資源到實例，回傳下載的檔案名稱。模組會連同必需的前置模組一起裝，
/// 全部找到相容版本後才開始下載，不會只裝一半。
pub async fn install(
    client: &reqwest::Client,
    kind: ProjectKind,
    wanted: Wanted,
    target: &InstallTarget<'_>,
) -> Result<Vec<String>, ModrinthError> {
    let loader = match kind {
        ProjectKind::Mod => Some(loader_name(target.loader)?),
        ProjectKind::DataPack => Some(DATAPACK_LOADER),
        _ => None,
    };
    let folder = kind.folder(target.game_dir, target.world)?;
    let mut pending = vec![wanted];
    let mut seen_projects = HashSet::new();
    let mut files = Vec::new();
    let mut installed = Vec::new();

    while let Some(wanted) = pending.pop() {
        let version = match wanted {
            Wanted::Project(project) => {
                if !seen_projects.insert(project.clone()) {
                    continue;
                }
                compatible_version(client, &project, target.game_version, loader).await?
            }
            Wanted::Version(id) => {
                let version = fetch_version(client, &id).await?;
                if !seen_projects.insert(version.project_id.clone()) {
                    continue;
                }
                version
            }
        };
        if kind == ProjectKind::Mod {
            pending.extend(required_dependencies(&version));
        }
        let file = primary_file(&version)?;
        installed.push((version.project_id.clone(), file.filename.clone()));
        files.push(file);
    }

    let names = files.iter().map(|file| file.filename.clone()).collect();
    let tasks = files
        .into_iter()
        .map(|file| {
            Ok(DownloadTask {
                path: folder.join(safe_file_name(&file.filename)?),
                url: file.url,
                sha1: file.hashes.sha1,
                size: file.size,
            })
        })
        .collect::<Result<Vec<_>, ModrinthError>>()?;
    let tasks = link_existing(tasks, target.other_mods_dirs).await;
    download_all(client, tasks, target.concurrency, |_| {}).await?;
    if kind == ProjectKind::Mod {
        remove_replaced_mods(client, &folder, &installed).await;
    }
    Ok(names)
}

/// 同一個模組留著兩個版本時，載入器會拒絕啟動遊戲，所以裝好新版後要移除舊版。
/// 模組可能是手動放進來的，只能靠檔案的 SHA1 問 Modrinth 它屬於哪個專案。
/// 查不到就保留：移除失敗頂多要手動處理，誤刪才是問題。
async fn remove_replaced_mods(
    client: &reqwest::Client,
    folder: &Path,
    installed: &[(String, String)],
) {
    let Ok(mut entries) = tokio::fs::read_dir(folder).await else {
        return;
    };
    let mut local = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Ok(Some(hash)) = file_sha1(&entry.path()).await {
            local.push((name, hash));
        }
    }
    let hashes: Vec<&str> = local.iter().map(|(_, hash)| hash.as_str()).collect();
    let Ok(projects) = projects_of(client, &hashes).await else {
        return;
    };
    for name in replaced_files(&local, &projects, installed) {
        // 刪不掉時保留，用戶可以在實例頁手動處理。
        let _ = tokio::fs::remove_file(folder.join(name)).await;
    }
}

/// 用 SHA1 查檔案屬於哪個專案。
async fn projects_of(
    client: &reqwest::Client,
    hashes: &[&str],
) -> Result<HashMap<String, String>, ModrinthError> {
    let versions: HashMap<String, ProjectOfFile> = installed::post(
        client,
        "version_files",
        &serde_json::json!({ "hashes": hashes, "algorithm": "sha1" }),
    )
    .await?;
    Ok(versions
        .into_iter()
        .map(|(hash, version)| (hash, version.project_id))
        .collect())
}

#[derive(Debug, Deserialize)]
struct ProjectOfFile {
    project_id: String,
}

/// 屬於剛裝好的專案、但不是剛裝的那個檔案，就是要移除的舊版本。
fn replaced_files<'a>(
    local: &'a [(String, String)],
    projects: &HashMap<String, String>,
    installed: &[(String, String)],
) -> Vec<&'a str> {
    local
        .iter()
        .filter(|(name, hash)| {
            projects.get(hash).is_some_and(|project| {
                installed
                    .iter()
                    .any(|(id, file)| id == project && file != name)
            })
        })
        .map(|(name, _)| name.as_str())
        .collect()
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
    loader: Option<&str>,
) -> Result<ModVersion, ModrinthError> {
    let game_versions = serde_json::json!([game_version]).to_string();
    let mut params = vec![("game_versions", game_versions)];
    if let Some(loader) = loader {
        params.push(("loaders", serde_json::json!([loader]).to_string()));
    }
    let url = Url::parse_with_params(&format!("{API_URL}/project/{project}/version"), params)
        .map_err(|_| ModrinthError::BadUrl)?;
    let versions: Vec<ModVersion> = serde_json::from_str(&fetch_text(client, url.as_str()).await?)?;
    match pick_version(versions) {
        Some(version) => Ok(version),
        None => Err(ModrinthError::NoCompatibleVersion {
            project: project_title(client, project).await,
        }),
    }
}

/// 前置模組只有 id，錯誤訊息要用看得懂的名稱。查不到才顯示 id。
async fn project_title(client: &reqwest::Client, project: &str) -> String {
    #[derive(Deserialize)]
    struct Project {
        title: String,
    }
    let url = format!("{API_URL}/project/{project}");
    let text = fetch_text(client, &url).await.unwrap_or_default();
    serde_json::from_str::<Project>(&text).map_or_else(|_| project.to_string(), |found| found.title)
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

/// 檔案名稱來自網路，不能讓它指到實例資料夾以外的地方。
fn safe_file_name(name: &str) -> Result<&str, ModrinthError> {
    let allowed = name.ends_with(".jar") || name.ends_with(".zip");
    let safe = allowed && !name.contains(['/', '\\']) && !name.contains("..");
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
        let result: SearchPage = serde_json::from_str(
            r#"{ "total_hits": 35, "hits": [{ "project_id": "AANobbMI", "slug": "sodium", "title": "Sodium",
                 "description": "快", "downloads": 5, "icon_url": null, "client_side": "required",
                 "author": "jellysquid3", "categories": ["optimization"], "date_modified": "2026-01-01" }] }"#,
        )
        .unwrap();

        assert_eq!(result.hits[0].project_id, "AANobbMI");
        assert_eq!(result.hits[0].icon_url, None);
        assert_eq!(result.total, 35);
        assert_eq!(result.hits[0].categories, ["optimization"]);
    }

    fn request(kind: ProjectKind) -> SearchRequest {
        SearchRequest {
            query: String::new(),
            kind,
            game_version: None,
            loader: None,
            category: None,
            sort: None,
            offset: 0,
        }
    }

    #[test]
    fn filters_mods_by_version_loader_and_category() {
        let search = SearchRequest {
            game_version: Some("1.21.8".to_string()),
            loader: Some(LoaderKind::NeoForge),
            category: Some("optimization".to_string()),
            ..request(ProjectKind::Mod)
        };

        assert_eq!(
            facets(&search).unwrap(),
            r#"[["project_type:mod"],["versions:1.21.8"],["categories:neoforge"],["categories:optimization"]]"#
        );
    }

    #[test]
    fn modpacks_accept_any_supported_loader() {
        assert_eq!(
            facets(&request(ProjectKind::Modpack)).unwrap(),
            r#"[["project_type:modpack"],["categories:fabric","categories:forge","categories:neoforge"]]"#
        );
    }

    #[test]
    fn shaders_ignore_the_instance_loader() {
        let search = SearchRequest {
            loader: Some(LoaderKind::Vanilla),
            ..request(ProjectKind::Shader)
        };

        assert_eq!(facets(&search).unwrap(), r#"[["project_type:shader"]]"#);
    }

    #[test]
    fn sort_falls_back_when_empty_or_unknown() {
        let hot = request(ProjectKind::Mod);
        let typed = SearchRequest {
            query: "sodium".to_string(),
            ..request(ProjectKind::Mod)
        };
        let newest = SearchRequest {
            sort: Some("newest".to_string()),
            ..request(ProjectKind::Mod)
        };
        let unknown = SearchRequest {
            sort: Some("drop table".to_string()),
            ..request(ProjectKind::Mod)
        };

        assert_eq!(sort_index(&hot), "downloads");
        assert_eq!(sort_index(&typed), "relevance");
        assert_eq!(sort_index(&newest), "newest");
        assert_eq!(sort_index(&unknown), "downloads");
    }

    #[test]
    fn resources_go_to_their_own_folders() {
        let game = Path::new("game");

        assert_eq!(
            ProjectKind::Mod.folder(game, None).unwrap(),
            game.join("mods")
        );
        assert_eq!(
            ProjectKind::ResourcePack.folder(game, None).unwrap(),
            game.join("resourcepacks")
        );
        assert_eq!(
            ProjectKind::Shader.folder(game, None).unwrap(),
            game.join("shaderpacks")
        );
    }

    #[test]
    fn data_packs_go_into_an_existing_world() {
        let game = std::env::temp_dir().join(format!("sapling-datapack-{}", std::process::id()));
        std::fs::create_dir_all(game.join("saves/生存")).unwrap();
        std::fs::write(game.join("saves/生存/level.dat"), "x").unwrap();

        assert_eq!(
            ProjectKind::DataPack.folder(&game, Some("生存")).unwrap(),
            game.join("saves/生存/datapacks")
        );
        assert!(matches!(
            ProjectKind::DataPack.folder(&game, None),
            Err(ModrinthError::NoWorld)
        ));
        assert!(matches!(
            ProjectKind::DataPack.folder(&game, Some("../mods")),
            Err(ModrinthError::NoWorld)
        ));
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
    fn finds_older_versions_of_what_was_just_installed() {
        let local = vec![
            ("sodium-old.jar".to_string(), "h1".to_string()),
            ("sodium-new.jar".to_string(), "h2".to_string()),
            ("iris.jar.disabled".to_string(), "h3".to_string()),
            ("handmade.jar".to_string(), "h4".to_string()),
        ];
        let projects = HashMap::from([
            ("h1".to_string(), "sodium".to_string()),
            ("h2".to_string(), "sodium".to_string()),
            ("h3".to_string(), "iris".to_string()),
        ]);
        let installed = [("sodium".to_string(), "sodium-new.jar".to_string())];

        assert_eq!(
            replaced_files(&local, &projects, &installed),
            ["sodium-old.jar"]
        );
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
        assert!(safe_file_name("BSL_v10.1.8.zip").is_ok());
        assert!(safe_file_name("../evil.jar").is_err());
        assert!(safe_file_name("a/b.jar").is_err());
        assert!(safe_file_name("a\\b.jar").is_err());
        assert!(safe_file_name("readme.txt").is_err());
    }
}
