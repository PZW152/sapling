//! 清理沒有實例在用的共用檔案。刪掉實例時不會動這些檔案，因為別的版本可能還在用。
//!
//! 這裡會刪資料，所以寧可少刪：看不懂任何一份版本資料就整個放棄，
//! 也不碰存檔和模組，那些都在實例資料夾裡，不在清理範圍。

use std::collections::HashSet;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

use crate::instance::{Instance, InstanceError};
use crate::loader;
use crate::platform::DataDirs;
use crate::versions::details::VersionDetails;
use crate::versions::install::AssetIndexFile;

#[derive(Debug, Error)]
pub enum CleanupError {
    #[error(transparent)]
    Instance(#[from] InstanceError),
    #[error("有版本資料看不懂，為了安全沒有清理任何檔案：{}", path.display())]
    Unreadable { path: PathBuf },
    #[error("無法讀取資料夾：{}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// 還需要保留的東西。
#[derive(Debug, Default)]
struct Keep {
    versions: HashSet<String>,
    runtimes: HashSet<String>,
    asset_indexes: HashSet<String>,
    assets: HashSet<String>,
    /// `None` 代表無法確定，不清理函式庫：Forge 系列有安裝時才產生的檔案。
    libraries: Option<HashSet<PathBuf>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupSummary {
    pub bytes: u64,
    pub items: usize,
}

#[derive(Debug)]
struct Plan {
    paths: Vec<PathBuf>,
    bytes: u64,
}

/// 看看可以清出多少空間，不刪任何東西。
pub async fn preview(dirs: &DataDirs) -> Result<CleanupSummary, CleanupError> {
    let plan = make_plan(dirs).await?;
    Ok(CleanupSummary {
        bytes: plan.bytes,
        items: plan.paths.len(),
    })
}

/// 刪除沒有在用的檔案，回傳實際清出的大小。
/// 刪不掉的檔案直接略過，例如正在運行的遊戲鎖著的檔案。
pub async fn run(dirs: &DataDirs) -> Result<u64, CleanupError> {
    let plan = make_plan(dirs).await?;
    tokio::task::spawn_blocking(move || plan.paths.iter().map(|path| remove(path)).sum())
        .await
        .map_err(|source| CleanupError::Io {
            path: PathBuf::new(),
            source: io::Error::other(source),
        })
}

fn remove(path: &Path) -> u64 {
    let size = size_of(path);
    let removed = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    if removed.is_ok() { size } else { 0 }
}

async fn make_plan(dirs: &DataDirs) -> Result<Plan, CleanupError> {
    let instances = Instance::list(dirs).await?;
    // 一個實例都沒有時多半是資料讀錯了，不要把全部檔案當成沒人用。
    if instances.is_empty() {
        return Ok(Plan {
            paths: Vec::new(),
            bytes: 0,
        });
    }
    let keep = find_keep(&instances, dirs).await?;
    let dirs = dirs.clone();
    tokio::task::spawn_blocking(move || unused_paths(&keep, &dirs))
        .await
        .map_err(|source| CleanupError::Io {
            path: PathBuf::new(),
            source: io::Error::other(source),
        })?
}

async fn find_keep(instances: &[Instance], dirs: &DataDirs) -> Result<Keep, CleanupError> {
    let mut keep = Keep {
        libraries: Some(HashSet::new()),
        ..Keep::default()
    };
    for instance in instances {
        keep.versions.insert(instance.version.clone());
        if let Some(folder) =
            loader::folder_id(instance.loader, &instance.loader_version, &instance.version)
        {
            keep.versions.insert(folder);
        }
        match loader::known_libraries(instance, dirs).await {
            Some(paths) => {
                if let Some(libraries) = &mut keep.libraries {
                    libraries.extend(paths.iter().map(|path| library_key(path)));
                }
            }
            None => keep.libraries = None,
        }

        let path = dirs
            .versions()
            .join(&instance.version)
            .join(format!("{}.json", instance.version));
        // 建立後還沒下載過的實例沒有版本資料，也就沒有要保留的東西。
        let Ok(text) = tokio::fs::read_to_string(&path).await else {
            continue;
        };
        let details = VersionDetails::parse(&text)
            .map_err(|_| CleanupError::Unreadable { path: path.clone() })?;
        keep_version_files(&details, &mut keep, dirs).await?;
    }
    Ok(keep)
}

async fn keep_version_files(
    details: &VersionDetails,
    keep: &mut Keep,
    dirs: &DataDirs,
) -> Result<(), CleanupError> {
    keep.runtimes.insert(details.java_version.component.clone());
    keep.asset_indexes.insert(details.asset_index.id.clone());
    if let Some(libraries) = &mut keep.libraries {
        for library in details.libraries_for_this_machine() {
            let artifacts = library
                .downloads
                .artifact
                .iter()
                .chain(library.native_artifact());
            libraries.extend(artifacts.map(|artifact| library_key(Path::new(&artifact.path))));
        }
    }

    let path = dirs
        .assets()
        .join("indexes")
        .join(format!("{}.json", details.asset_index.id));
    let Ok(text) = tokio::fs::read_to_string(&path).await else {
        return Ok(());
    };
    let index: AssetIndexFile =
        serde_json::from_str(&text).map_err(|_| CleanupError::Unreadable { path: path.clone() })?;
    keep.assets
        .extend(index.objects.into_values().map(|object| object.hash));
    Ok(())
}

/// 同一個路徑可能用 `/` 或 `\` 寫，比較前先統一。
fn library_key(path: &Path) -> PathBuf {
    path.components()
        .filter(|part| matches!(part, Component::Normal(_)))
        .collect()
}

fn unused_paths(keep: &Keep, dirs: &DataDirs) -> Result<Plan, CleanupError> {
    let mut paths = Vec::new();

    for entry in entries(&dirs.versions())? {
        if !keep.versions.contains(&file_name(&entry)) {
            paths.push(entry);
        }
    }
    for entry in entries(&dirs.runtimes())? {
        let name = file_name(&entry);
        let component = name.strip_suffix(".json").unwrap_or(&name);
        if !keep.runtimes.contains(component) {
            paths.push(entry);
        }
    }
    for entry in entries(&dirs.assets().join("indexes"))? {
        let name = file_name(&entry);
        let id = name.strip_suffix(".json").unwrap_or(&name);
        if !keep.asset_indexes.contains(id) {
            paths.push(entry);
        }
    }
    for folder in entries(&dirs.assets().join("objects"))? {
        for object in entries(&folder)? {
            if !keep.assets.contains(&file_name(&object)) {
                paths.push(object);
            }
        }
    }
    if let Some(libraries) = &keep.libraries {
        unused_libraries(&dirs.libraries(), &dirs.libraries(), libraries, &mut paths)?;
    }

    let bytes = paths.iter().map(|path| size_of(path)).sum();
    Ok(Plan { paths, bytes })
}

fn unused_libraries(
    root: &Path,
    folder: &Path,
    keep: &HashSet<PathBuf>,
    paths: &mut Vec<PathBuf>,
) -> Result<(), CleanupError> {
    for entry in entries(folder)? {
        if entry.is_dir() {
            unused_libraries(root, &entry, keep, paths)?;
        } else if let Ok(relative) = entry.strip_prefix(root)
            && !keep.contains(&library_key(relative))
        {
            paths.push(entry);
        }
    }
    Ok(())
}

/// 資料夾裡的項目。資料夾不存在就是空的。
fn entries(folder: &Path) -> Result<Vec<PathBuf>, CleanupError> {
    let io_error = |source| CleanupError::Io {
        path: folder.to_path_buf(),
        source,
    };
    let list = match std::fs::read_dir(folder) {
        Ok(list) => list,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error(error)),
    };
    list.map(|entry| entry.map(|entry| entry.path()).map_err(io_error))
        .collect()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn size_of(path: &Path) -> u64 {
    if path.is_dir() {
        std::fs::read_dir(path)
            .map(|list| list.flatten().map(|entry| size_of(&entry.path())).sum())
            .unwrap_or(0)
    } else {
        std::fs::metadata(path).map_or(0, |metadata| metadata.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::LoaderKind;

    fn temp_dirs(name: &str) -> DataDirs {
        let root =
            std::env::temp_dir().join(format!("sapling-cleanup-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        DataDirs::at(root)
    }

    fn write(path: PathBuf, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    /// 寫一個版本：一個函式庫、一個資源、一份 Java。
    fn add_version(dirs: &DataDirs, id: &str, library: &str, asset: &str, java: &str) {
        write(
            dirs.versions().join(id).join(format!("{id}.json")),
            &format!(
                r#"{{
                    "id": "{id}", "mainClass": "m",
                    "assetIndex": {{ "id": "{id}", "sha1": "a", "size": 2, "totalSize": 3, "url": "u" }},
                    "downloads": {{ "client": {{ "sha1": "c", "size": 4, "url": "u" }} }},
                    "javaVersion": {{ "component": "{java}", "majorVersion": 21 }},
                    "libraries": [{{ "name": "a:b:1", "downloads": {{ "artifact":
                        {{ "path": "{library}", "sha1": "l", "size": 5, "url": "u" }} }} }}]
                }}"#
            ),
        );
        write(dirs.versions().join(id).join(format!("{id}.jar")), "jar");
        write(dirs.libraries().join(library), "lib");
        write(
            dirs.assets().join("indexes").join(format!("{id}.json")),
            &format!(r#"{{ "objects": {{ "a": {{ "hash": "{asset}", "size": 1 }} }} }}"#),
        );
        write(
            dirs.assets().join("objects").join(&asset[..2]).join(asset),
            "asset",
        );
        write(dirs.runtimes().join(java).join("bin/java.exe"), "java");
        write(dirs.runtimes().join(format!("{java}.json")), "{}");
    }

    #[tokio::test]
    async fn removes_only_what_no_instance_uses() {
        let dirs = temp_dirs("basic");
        add_version(&dirs, "26.3", "keep/lib.jar", "aa11", "java-new");
        add_version(&dirs, "1.8.9", "old/lib.jar", "bb22", "java-old");
        let instance = Instance::create(&dirs, "生存", "26.3").await.unwrap();
        write(instance.game_dir(&dirs).join("saves/世界/level.dat"), "x");

        let summary = preview(&dirs).await.unwrap();
        assert!(summary.bytes > 0);
        assert!(dirs.versions().join("1.8.9").exists(), "預覽不能刪東西");

        run(&dirs).await.unwrap();

        assert!(dirs.versions().join("26.3/26.3.jar").exists());
        assert!(dirs.libraries().join("keep/lib.jar").exists());
        assert!(dirs.assets().join("objects/aa/aa11").exists());
        assert!(dirs.runtimes().join("java-new/bin/java.exe").exists());
        assert!(
            instance
                .game_dir(&dirs)
                .join("saves/世界/level.dat")
                .exists()
        );

        assert!(!dirs.versions().join("1.8.9").exists());
        assert!(!dirs.libraries().join("old/lib.jar").exists());
        assert!(!dirs.assets().join("objects/bb/bb22").exists());
        assert!(!dirs.runtimes().join("java-old").exists());
        assert!(!dirs.runtimes().join("java-old.json").exists());
    }

    #[tokio::test]
    async fn keeps_files_shared_with_a_version_still_in_use() {
        let dirs = temp_dirs("shared");
        add_version(&dirs, "26.3", "same/lib.jar", "cc33", "java-new");
        add_version(&dirs, "26.2", "same/lib.jar", "cc33", "java-new");
        Instance::create(&dirs, "生存", "26.3").await.unwrap();

        run(&dirs).await.unwrap();

        assert!(!dirs.versions().join("26.2").exists());
        assert!(dirs.libraries().join("same/lib.jar").exists());
        assert!(dirs.assets().join("objects/cc/cc33").exists());
    }

    #[tokio::test]
    async fn does_nothing_when_there_are_no_instances() {
        let dirs = temp_dirs("empty");
        add_version(&dirs, "26.3", "a/lib.jar", "dd44", "java-new");

        assert_eq!(run(&dirs).await.unwrap(), 0);
        assert!(dirs.versions().join("26.3").exists());
    }

    #[tokio::test]
    async fn gives_up_entirely_on_unreadable_version_data() {
        let dirs = temp_dirs("broken");
        add_version(&dirs, "1.8.9", "old/lib.jar", "ee55", "java-old");
        write(dirs.versions().join("26.3/26.3.json"), "not json");
        Instance::create(&dirs, "生存", "26.3").await.unwrap();

        assert!(matches!(
            run(&dirs).await,
            Err(CleanupError::Unreadable { .. })
        ));
        assert!(dirs.versions().join("1.8.9").exists());
    }

    #[tokio::test]
    async fn leaves_libraries_alone_when_forge_is_in_use() {
        let dirs = temp_dirs("forge");
        add_version(&dirs, "26.3", "keep/lib.jar", "ff66", "java-new");
        write(
            dirs.libraries().join("generated/patched.jar"),
            "made by forge",
        );
        Instance::create_with_loader(
            &dirs,
            "模組",
            "26.3",
            LoaderKind::Forge,
            "66.1.0".to_string(),
        )
        .await
        .unwrap();

        run(&dirs).await.unwrap();

        assert!(dirs.libraries().join("generated/patched.jar").exists());
    }
}
