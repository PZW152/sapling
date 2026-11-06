//! 本地模組管理：實例 `mods` 資料夾裡的 `.jar`，停用時改名為 `.jar.disabled`。

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{Instance, InstanceError};
use crate::download::{DownloadTask, has_expected_sha1};
use crate::platform::DataDirs;

const ENABLED_SUFFIX: &str = ".jar";
const DISABLED_SUFFIX: &str = ".jar.disabled";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    /// 不含停用後綴的檔案名。
    pub name: String,
    pub enabled: bool,
    pub size: u64,
}

pub fn mods_dir(game_dir: &Path) -> PathBuf {
    game_dir.join("mods")
}

/// 所有實例的模組資料夾。
pub async fn all_mods_dirs(dirs: &DataDirs) -> Result<Vec<PathBuf>, InstanceError> {
    let instances = Instance::list(dirs).await?;
    Ok(instances
        .iter()
        .map(|instance| mods_dir(&instance.game_dir(dirs)))
        .collect())
}

/// 其他實例已經有完全相同的模組時，用硬連結接過來，回傳還需要下載的工作。
///
/// 硬連結只佔一份空間，刪掉任何一邊都不影響另一邊。連結失敗就照常下載，
/// 例如不同磁碟或隨身碟的 exFAT 格式都不支援。只用於模組：存檔和設定會被修改，
/// 共用會互相影響。
pub async fn link_existing(tasks: Vec<DownloadTask>, sources: &[PathBuf]) -> Vec<DownloadTask> {
    let mut remaining = Vec::new();
    for task in tasks {
        if !try_link(&task, sources).await {
            remaining.push(task);
        }
    }
    remaining
}

async fn try_link(task: &DownloadTask, sources: &[PathBuf]) -> bool {
    let Some(name) = task.path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if !name.ends_with(ENABLED_SUFFIX) || task.path.exists() {
        return false;
    }
    let Some(parent) = task.path.parent() else {
        return false;
    };
    if tokio::fs::create_dir_all(parent).await.is_err() {
        return false;
    }
    for source in sources {
        let disabled = format!("{}{DISABLED_SUFFIX}", name.trim_end_matches(ENABLED_SUFFIX));
        for candidate in [source.join(name), source.join(disabled)] {
            let same = has_expected_sha1(&candidate, &task.sha1)
                .await
                .unwrap_or(false);
            if same && tokio::fs::hard_link(&candidate, &task.path).await.is_ok() {
                return true;
            }
        }
    }
    false
}

/// 列出所有模組，按名稱排序。沒有 `mods` 資料夾就是空清單。
pub async fn list(game_dir: &Path) -> Result<Vec<ModFile>, InstanceError> {
    let dir = mods_dir(game_dir);
    let io_error = |source| InstanceError::Io {
        path: dir.clone(),
        source,
    };
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error(error)),
    };
    let mut mods = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(io_error)? {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Some((name, enabled)) = parse_name(&file_name) else {
            continue;
        };
        let size = entry.metadata().await.map_err(io_error)?.len();
        mods.push(ModFile {
            name,
            enabled,
            size,
        });
    }
    mods.sort_by_key(|file| file.name.to_lowercase());
    Ok(mods)
}

pub async fn set_enabled(game_dir: &Path, name: &str, enabled: bool) -> Result<(), InstanceError> {
    let (from, to) = if enabled {
        (
            disabled_path(game_dir, name)?,
            enabled_path(game_dir, name)?,
        )
    } else {
        (
            enabled_path(game_dir, name)?,
            disabled_path(game_dir, name)?,
        )
    };
    tokio::fs::rename(&from, &to)
        .await
        .map_err(|source| InstanceError::Io { path: from, source })
}

pub async fn delete(game_dir: &Path, name: &str, enabled: bool) -> Result<(), InstanceError> {
    let path = if enabled {
        enabled_path(game_dir, name)?
    } else {
        disabled_path(game_dir, name)?
    };
    tokio::fs::remove_file(&path)
        .await
        .map_err(|source| InstanceError::Io { path, source })
}

fn parse_name(file_name: &str) -> Option<(String, bool)> {
    if file_name.ends_with(DISABLED_SUFFIX) {
        let name = file_name.strip_suffix(".disabled")?;
        return Some((name.to_string(), false));
    }
    file_name
        .ends_with(ENABLED_SUFFIX)
        .then(|| (file_name.to_string(), true))
}

fn enabled_path(game_dir: &Path, name: &str) -> Result<PathBuf, InstanceError> {
    Ok(mods_dir(game_dir).join(checked_name(name)?))
}

fn disabled_path(game_dir: &Path, name: &str) -> Result<PathBuf, InstanceError> {
    Ok(mods_dir(game_dir).join(format!("{}.disabled", checked_name(name)?)))
}

/// 只接受單純的檔案名，避免操作到 `mods` 資料夾以外的檔案。
fn checked_name(name: &str) -> Result<&str, InstanceError> {
    let is_plain = Path::new(name).file_name().is_some_and(|file| file == name);
    if is_plain && name.ends_with(ENABLED_SUFFIX) {
        Ok(name)
    } else {
        Err(InstanceError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sapling-mods-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(mods_dir(&dir)).unwrap();
        dir
    }

    /// "hello" 的 SHA1。
    const HELLO_SHA1: &str = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";

    fn hello_task(dir: &Path) -> DownloadTask {
        DownloadTask {
            url: "https://example.invalid/a.jar".to_string(),
            path: mods_dir(dir).join("a.jar"),
            sha1: HELLO_SHA1.to_string(),
            size: 5,
        }
    }

    #[tokio::test]
    async fn links_an_identical_mod_instead_of_downloading() {
        let source = game_dir("link-source");
        let target = game_dir("link-target");
        std::fs::write(mods_dir(&source).join("a.jar"), "hello").unwrap();

        let remaining = link_existing(vec![hello_task(&target)], &[mods_dir(&source)]).await;

        assert!(remaining.is_empty());
        std::fs::remove_file(mods_dir(&source).join("a.jar")).unwrap();
        assert_eq!(
            std::fs::read(mods_dir(&target).join("a.jar")).unwrap(),
            b"hello"
        );
    }

    #[tokio::test]
    async fn finds_a_disabled_copy_too() {
        let source = game_dir("link-disabled");
        let target = game_dir("link-disabled-target");
        std::fs::write(mods_dir(&source).join("a.jar.disabled"), "hello").unwrap();

        let remaining = link_existing(vec![hello_task(&target)], &[mods_dir(&source)]).await;

        assert!(remaining.is_empty());
        assert!(mods_dir(&target).join("a.jar").exists());
    }

    #[tokio::test]
    async fn downloads_when_the_same_name_has_different_content() {
        let source = game_dir("link-different");
        let target = game_dir("link-different-target");
        std::fs::write(mods_dir(&source).join("a.jar"), "other").unwrap();

        let remaining = link_existing(vec![hello_task(&target)], &[mods_dir(&source)]).await;

        assert_eq!(remaining.len(), 1);
        assert!(!mods_dir(&target).join("a.jar").exists());
    }

    #[tokio::test]
    async fn lists_only_mod_files() {
        let dir = game_dir("list");
        std::fs::write(mods_dir(&dir).join("b.jar"), "x").unwrap();
        std::fs::write(mods_dir(&dir).join("a.jar.disabled"), "xx").unwrap();
        std::fs::write(mods_dir(&dir).join("notes.txt"), "x").unwrap();

        let mods = list(&dir).await.unwrap();

        assert_eq!(mods.len(), 2);
        assert_eq!((mods[0].name.as_str(), mods[0].enabled), ("a.jar", false));
        assert_eq!((mods[1].name.as_str(), mods[1].enabled), ("b.jar", true));
    }

    #[tokio::test]
    async fn disables_enables_and_deletes() {
        let dir = game_dir("toggle");
        std::fs::write(mods_dir(&dir).join("a.jar"), "x").unwrap();

        set_enabled(&dir, "a.jar", false).await.unwrap();
        assert!(mods_dir(&dir).join("a.jar.disabled").exists());
        set_enabled(&dir, "a.jar", true).await.unwrap();
        assert!(mods_dir(&dir).join("a.jar").exists());
        delete(&dir, "a.jar", true).await.unwrap();
        assert!(list(&dir).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn refuses_paths_outside_the_mods_folder() {
        let dir = game_dir("outside");
        assert!(delete(&dir, "../instance.jar", true).await.is_err());
    }

    #[tokio::test]
    async fn missing_folder_means_no_mods() {
        let dir = game_dir("missing");
        std::fs::remove_dir_all(mods_dir(&dir)).unwrap();
        assert!(list(&dir).await.unwrap().is_empty());
    }
}
