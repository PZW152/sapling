//! 安裝 Modrinth 整合包（`.mrpack`）：建立實例、下載檔案、解開內含的設定。
//!
//! 格式說明：<https://support.modrinth.com/en/articles/8802351>

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use reqwest::Url;
use serde::Deserialize;
use zip::ZipArchive;

use crate::download::{DownloadProgress, DownloadTask, download_all, download_file, fetch_text};
use crate::instance::mods::{all_mods_dirs, link_existing};
use crate::instance::{Instance, icon};
use crate::loader::LoaderKind;
use crate::platform::DataDirs;
use crate::storage::safe_relative_path;
use crate::versions::fetch_manifest;

use super::{API_URL, FileHashes, ModVersion, ModrinthError, fetch_version, pick_version};

pub(super) const INDEX_FILE: &str = "modrinth.index.json";
pub(super) const FORMAT_VERSION: u32 = 1;
pub(super) const GAME: &str = "minecraft";
pub(super) const OVERRIDES: &str = "overrides";
const UNSUPPORTED: &str = "unsupported";
const SECURE_SCHEME: &str = "https://";
/// 依序解開，後面的覆蓋前面的。
const OVERRIDE_FOLDERS: [&str; 2] = ["overrides/", "client-overrides/"];
/// 整合包裡的載入器名稱。
pub(super) const LOADERS: [(&str, LoaderKind); 4] = [
    ("fabric-loader", LoaderKind::Fabric),
    ("quilt-loader", LoaderKind::Quilt),
    ("forge", LoaderKind::Forge),
    ("neoforge", LoaderKind::NeoForge),
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackIndex {
    #[serde(default)]
    name: String,
    format_version: u32,
    game: String,
    files: Vec<PackFile>,
    dependencies: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackFile {
    path: String,
    hashes: FileHashes,
    env: Option<PackEnv>,
    downloads: Vec<String>,
    file_size: u64,
}

#[derive(Debug, Deserialize)]
struct PackEnv {
    client: String,
}

/// 整合包需要的遊戲版本和載入器。
#[derive(Debug, PartialEq, Eq)]
struct PackTarget {
    game_version: String,
    loader: LoaderKind,
    loader_version: String,
}

/// 下載並安裝整合包，回傳新建立的實例。中途失敗不會留下半成品。
pub async fn install_pack(
    client: &reqwest::Client,
    dirs: &DataDirs,
    project_id: &str,
    version_id: Option<&str>,
    name: &str,
    concurrency: usize,
    on_progress: impl FnMut(DownloadProgress),
) -> Result<Instance, ModrinthError> {
    let version = match version_id {
        Some(id) => fetch_version(client, id).await?,
        None => latest_pack_version(client, project_id).await?,
    };
    let pack_path = dirs.root().join(format!("{}.mrpack.tmp", version.id));
    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .ok_or(ModrinthError::BadPack)?;
    download_file(
        client,
        &DownloadTask {
            url: file.url.clone(),
            path: pack_path.clone(),
            sha1: file.hashes.sha1.clone(),
            size: file.size,
        },
    )
    .await?;

    let result = install_from_file(client, dirs, &pack_path, name, concurrency, on_progress).await;
    // 清理失敗不影響結果，只是留下一個暫存檔。
    let _ = tokio::fs::remove_file(&pack_path).await;
    if let Ok(instance) = &result {
        save_pack_icon(client, project_id, &instance.game_dir(dirs)).await;
    }
    result
}

/// 圖示只是裝飾，抓不到就算了。
async fn save_pack_icon(client: &reqwest::Client, project_id: &str, game_dir: &Path) {
    #[derive(Deserialize)]
    struct Project {
        icon_url: Option<String>,
    }
    let url = format!("{API_URL}/project/{project_id}");
    let Ok(text) = fetch_text(client, &url).await else {
        return;
    };
    let Some(icon_url) = serde_json::from_str::<Project>(&text)
        .ok()
        .and_then(|found| found.icon_url)
    else {
        return;
    };
    if let Ok(response) = client.get(icon_url).send().await
        && let Ok(bytes) = response.bytes().await
    {
        let _ = icon::save(game_dir, &bytes).await;
    }
}

/// 安裝電腦上的 `.mrpack`。實例名稱用整合包自己的名稱，沒有才用檔名。
pub async fn install_local_pack(
    client: &reqwest::Client,
    dirs: &DataDirs,
    path: &Path,
    concurrency: usize,
    on_progress: impl FnMut(DownloadProgress),
) -> Result<Instance, ModrinthError> {
    let index = read_index(path.to_path_buf()).await?;
    let file_name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string());
    let name = Some(index.name)
        .filter(|name| !name.trim().is_empty())
        .or(file_name)
        .unwrap_or_else(|| "整合包".to_string());
    install_from_file(client, dirs, path, &name, concurrency, on_progress).await
}

async fn latest_pack_version(
    client: &reqwest::Client,
    project_id: &str,
) -> Result<ModVersion, ModrinthError> {
    let loaders = serde_json::json!(super::PACK_LOADERS).to_string();
    let url = Url::parse_with_params(
        &format!("{API_URL}/project/{project_id}/version"),
        [("loaders", loaders.as_str())],
    )
    .map_err(|_| ModrinthError::BadUrl)?;
    let versions: Vec<ModVersion> = serde_json::from_str(&fetch_text(client, url.as_str()).await?)?;
    pick_version(versions).ok_or_else(|| ModrinthError::NoCompatibleVersion {
        project: project_id.to_string(),
    })
}

async fn install_from_file(
    client: &reqwest::Client,
    dirs: &DataDirs,
    pack_path: &Path,
    name: &str,
    concurrency: usize,
    on_progress: impl FnMut(DownloadProgress),
) -> Result<Instance, ModrinthError> {
    let index = read_index(pack_path.to_path_buf()).await?;
    let target = pack_target(&index)?;
    let manifest = fetch_manifest(client).await?;
    let supported = manifest
        .supported_releases()?
        .iter()
        .any(|entry| entry.id == target.game_version);
    if !supported {
        return Err(ModrinthError::OldGameVersion {
            version: target.game_version,
        });
    }

    let instance = Instance::create_with_loader(
        dirs,
        name,
        &target.game_version,
        target.loader,
        target.loader_version,
    )
    .await?;
    let game_dir = instance.game_dir(dirs);
    let sources = all_mods_dirs(dirs).await?;
    let filled = fill_game_dir(
        client,
        index,
        &sources,
        pack_path.to_path_buf(),
        game_dir,
        concurrency,
        on_progress,
    )
    .await;
    if let Err(error) = filled {
        // 裝到一半的實例不能用，不要留在清單裡。
        let _ = instance.delete(dirs).await;
        return Err(error);
    }
    Ok(instance)
}

async fn read_index(pack_path: PathBuf) -> Result<PackIndex, ModrinthError> {
    tokio::task::spawn_blocking(move || {
        let mut archive = open_archive(&pack_path)?;
        let mut entry = archive
            .by_name(INDEX_FILE)
            .map_err(|_| ModrinthError::BadPack)?;
        let mut text = String::new();
        entry
            .read_to_string(&mut text)
            .map_err(|_| ModrinthError::BadPack)?;
        Ok::<_, ModrinthError>(serde_json::from_str(&text)?)
    })
    .await
    .map_err(|_| ModrinthError::BadPack)?
}

fn open_archive(path: &Path) -> Result<ZipArchive<File>, ModrinthError> {
    let file = File::open(path).map_err(|_| ModrinthError::BadPack)?;
    ZipArchive::new(file).map_err(|_| ModrinthError::BadPack)
}

fn pack_target(index: &PackIndex) -> Result<PackTarget, ModrinthError> {
    if index.format_version != FORMAT_VERSION || index.game != GAME {
        return Err(ModrinthError::BadPack);
    }
    let game_version = index
        .dependencies
        .get("minecraft")
        .ok_or(ModrinthError::BadPack)?
        .clone();
    let (loader, loader_version) = LOADERS
        .iter()
        .find_map(|(id, kind)| Some((*kind, index.dependencies.get(*id)?.clone())))
        .unwrap_or((LoaderKind::Vanilla, String::new()));
    Ok(PackTarget {
        game_version,
        loader,
        loader_version,
    })
}

async fn fill_game_dir(
    client: &reqwest::Client,
    index: PackIndex,
    sources: &[PathBuf],
    pack_path: PathBuf,
    game_dir: PathBuf,
    concurrency: usize,
    on_progress: impl FnMut(DownloadProgress),
) -> Result<(), ModrinthError> {
    let tasks = client_tasks(index.files, &game_dir)?;
    let tasks = link_existing(tasks, sources).await;
    download_all(client, tasks, concurrency, on_progress).await?;
    tokio::task::spawn_blocking(move || extract_overrides(&pack_path, &game_dir))
        .await
        .map_err(|_| ModrinthError::BadPack)?
}

/// 客戶端要下載的檔案。標記為不支援客戶端的不裝，可選項目一律安裝。
fn client_tasks(files: Vec<PackFile>, game_dir: &Path) -> Result<Vec<DownloadTask>, ModrinthError> {
    files
        .into_iter()
        .filter(|file| {
            file.env
                .as_ref()
                .is_none_or(|env| env.client != UNSUPPORTED)
        })
        .map(|file| {
            let path = safe_relative_path(&file.path).ok_or_else(|| ModrinthError::UnsafePath {
                path: file.path.clone(),
            })?;
            let url = file
                .downloads
                .into_iter()
                .find(|url| url.starts_with(SECURE_SCHEME))
                .ok_or(ModrinthError::BadPack)?;
            Ok(DownloadTask {
                url,
                path: game_dir.join(path),
                sha1: file.hashes.sha1,
                size: file.file_size,
            })
        })
        .collect()
}

fn extract_overrides(pack_path: &Path, game_dir: &Path) -> Result<(), ModrinthError> {
    let mut archive = open_archive(pack_path)?;
    for folder in OVERRIDE_FOLDERS {
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|_| ModrinthError::BadPack)?;
            let full_name = entry
                .name()
                .map_err(|_| ModrinthError::BadPack)?
                .to_string();
            let Some(name) = full_name.strip_prefix(folder).map(str::to_string) else {
                continue;
            };
            if entry.is_dir() || name.is_empty() {
                continue;
            }
            let relative = safe_relative_path(&name)
                .ok_or(ModrinthError::UnsafePath { path: name.clone() })?;
            let target = game_dir.join(relative);
            let io_error = |source| ModrinthError::Write {
                path: target.clone(),
                source,
            };
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(io_error)?;
            }
            let mut output = File::create(&target).map_err(io_error)?;
            io::copy(&mut entry, &mut output).map_err(io_error)?;
        }
    }
    Ok(())
}

/// 測試用：讀回匯出的整合包，確認安裝流程看得懂。
#[cfg(test)]
pub(super) fn read_exported(path: &Path) -> (String, String, LoaderKind, Vec<String>) {
    let mut archive = open_archive(path).unwrap();
    let names = archive
        .file_names()
        .flatten()
        .map(|name| name.to_string())
        .collect();
    let mut text = String::new();
    archive
        .by_name(INDEX_FILE)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    let index: PackIndex = serde_json::from_str(&text).unwrap();
    let target = pack_target(&index).unwrap();
    (index.name, target.game_version, target.loader, names)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    fn index(dependencies: &str) -> PackIndex {
        serde_json::from_str(&format!(
            r#"{{ "formatVersion": 1, "game": "minecraft", "versionId": "1", "name": "p",
                 "files": [], "dependencies": {dependencies} }}"#
        ))
        .unwrap()
    }

    fn pack_file(path: &str, client: Option<&str>, url: &str) -> PackFile {
        PackFile {
            path: path.to_string(),
            hashes: FileHashes {
                sha1: "a".to_string(),
                sha512: String::new(),
            },
            env: client.map(|client| PackEnv {
                client: client.to_string(),
            }),
            downloads: vec![url.to_string()],
            file_size: 1,
        }
    }

    #[test]
    fn reads_the_game_and_fabric_versions() {
        let target = pack_target(&index(
            r#"{ "minecraft": "26.3", "fabric-loader": "0.20.1" }"#,
        ))
        .unwrap();

        assert_eq!(
            target,
            PackTarget {
                game_version: "26.3".to_string(),
                loader: LoaderKind::Fabric,
                loader_version: "0.20.1".to_string(),
            }
        );
    }

    #[test]
    fn reads_quilt_packs() {
        let target = pack_target(&index(
            r#"{ "minecraft": "1.20.1", "quilt-loader": "0.24.0" }"#,
        ));

        assert_eq!(target.unwrap().loader, LoaderKind::Quilt);
    }

    #[test]
    fn reads_forge_and_neoforge_packs() {
        let forge = pack_target(&index(r#"{ "minecraft": "1.20.1", "forge": "47.4.0" }"#)).unwrap();
        let neo =
            pack_target(&index(r#"{ "minecraft": "26.3", "neoforge": "26.3.0.1" }"#)).unwrap();

        assert_eq!(forge.loader, LoaderKind::Forge);
        assert_eq!(forge.loader_version, "47.4.0");
        assert_eq!(neo.loader, LoaderKind::NeoForge);
    }

    #[test]
    fn ignores_dependencies_it_does_not_know() {
        let target = pack_target(&index(r#"{ "minecraft": "26.3", "something-new": "1" }"#));

        assert_eq!(target.unwrap().loader, LoaderKind::Vanilla);
    }

    #[test]
    fn skips_files_the_client_must_not_install() {
        let tasks = client_tasks(
            vec![
                pack_file("mods/both.jar", None, "https://x/a"),
                pack_file("mods/optional.jar", Some("optional"), "https://x/b"),
                pack_file("mods/server.jar", Some("unsupported"), "https://x/c"),
            ],
            Path::new("game"),
        )
        .unwrap();

        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn refuses_insecure_downloads() {
        let result = client_tasks(
            vec![pack_file("mods/a.jar", None, "http://x/a")],
            Path::new("game"),
        );

        assert!(matches!(result, Err(ModrinthError::BadPack)));
    }

    fn write_pack(name: &str, entries: &[(&str, &str)]) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("sapling-pack-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("test.mrpack");
        let mut writer = ZipWriter::new(File::create(&path).unwrap());
        for (entry, content) in entries {
            writer
                .start_file(*entry, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
        (path, root.join("game"))
    }

    #[test]
    fn client_overrides_win_over_shared_overrides() {
        let (pack, game) = write_pack(
            "overrides",
            &[
                ("overrides/config/a.txt", "shared"),
                ("overrides/options.txt", "keep"),
                ("client-overrides/config/a.txt", "client"),
                ("server-overrides/config/a.txt", "server"),
                ("modrinth.index.json", "{}"),
            ],
        );

        extract_overrides(&pack, &game).unwrap();

        assert_eq!(
            std::fs::read_to_string(game.join("config/a.txt")).unwrap(),
            "client"
        );
        assert!(game.join("options.txt").exists());
        assert!(!game.join("modrinth.index.json").exists());
    }

    #[test]
    fn stops_on_an_override_that_leaves_the_instance() {
        let (pack, game) = write_pack("escape", &[("overrides/../../evil.txt", "x")]);

        assert!(matches!(
            extract_overrides(&pack, &game),
            Err(ModrinthError::UnsafePath { .. })
        ));
    }
}
