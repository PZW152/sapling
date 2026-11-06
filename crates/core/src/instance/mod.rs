//! 實例：一個獨立遊戲資料夾 + 遊戲版本 + 載入器 + 設定。

mod copy;
pub mod import;
pub mod mods;
pub mod worlds;

use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::loader::LoaderKind;
use crate::platform::DataDirs;
use crate::storage::{StorageError, read_json, write_json};

/// `memory_mb` 為這個值時，按電腦記憶體自動分配。
pub const AUTO_MEMORY: u32 = 0;

const MIN_MEMORY_MB: u32 = 1024;
const MAX_MEMORY_MB: u32 = 65536;
const AUTO_MIN_MEMORY_MB: u32 = 2048;
const AUTO_MAX_MEMORY_MB: u32 = 8192;
const AUTO_MEMORY_STEP_MB: u32 = 512;
const AUTO_MEMORY_SHARE: u32 = 3;
/// 複製實例時略過的項目。
const DUPLICATE_SKIP: [&str; 4] = ["logs", "crash-reports", "mods", copy::LOCK_FILE];

#[derive(Debug, Error)]
pub enum InstanceError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("實例名稱不能是空的")]
    EmptyName,
    #[error("找不到這個實例")]
    NotFound,
    #[error("無法讀寫實例資料夾：{}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version: String,
    pub loader: LoaderKind,
    /// 載入器版本，原版是空的。舊實例沒有這個欄位。
    #[serde(default)]
    pub loader_version: String,
    /// 分配給遊戲的記憶體，`AUTO_MEMORY` 代表自動。
    pub memory_mb: u32,
    /// 額外的 Java 參數，用空白分隔。
    pub java_args: String,
}

impl Instance {
    /// 建立並保存一個新的原版實例。
    pub async fn create(dirs: &DataDirs, name: &str, version: &str) -> Result<Self, InstanceError> {
        Self::create_with_loader(dirs, name, version, LoaderKind::Vanilla, String::new()).await
    }

    /// 建立並保存一個使用指定載入器的新實例。
    pub async fn create_with_loader(
        dirs: &DataDirs,
        name: &str,
        version: &str,
        loader: LoaderKind,
        loader_version: String,
    ) -> Result<Self, InstanceError> {
        let name = clean_name(name)?;
        let instance = Self {
            id: next_id(dirs).await?,
            name,
            version: version.to_string(),
            loader,
            loader_version,
            memory_mb: AUTO_MEMORY,
            java_args: String::new(),
        };
        instance.save(dirs).await?;
        Ok(instance)
    }

    /// 所有實例，按建立順序排列。
    pub async fn list(dirs: &DataDirs) -> Result<Vec<Self>, InstanceError> {
        let root = dirs.instances();
        let io_error = |source| InstanceError::Io {
            path: root.clone(),
            source,
        };
        let mut entries = match tokio::fs::read_dir(&root).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(io_error(error)),
        };
        let mut instances = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(io_error)? {
            // 一個實例的設定損壞，不能讓其他實例都消失。
            let saved = read_json::<Self>(&entry.path().join("instance.json")).await;
            instances.extend(saved.ok().flatten());
        }
        instances.sort_by_key(|instance| creation_order(&instance.id));
        Ok(instances)
    }

    pub async fn load(dirs: &DataDirs, id: &str) -> Result<Self, InstanceError> {
        Self::list(dirs)
            .await?
            .into_iter()
            .find(|instance| instance.id == id)
            .ok_or(InstanceError::NotFound)
    }

    pub async fn save(&self, dirs: &DataDirs) -> Result<(), InstanceError> {
        let mut instance = self.clone();
        instance.name = clean_name(&self.name)?;
        if instance.memory_mb != AUTO_MEMORY {
            instance.memory_mb = instance.memory_mb.clamp(MIN_MEMORY_MB, MAX_MEMORY_MB);
        }
        write_json(&self.game_dir(dirs).join("instance.json"), &instance).await?;
        Ok(())
    }

    /// 複製出一個新實例，包括存檔、模組和設定。不複製日誌和崩潰報告。
    pub async fn duplicate(&self, dirs: &DataDirs) -> Result<Self, InstanceError> {
        let mut copy = self.clone();
        copy.id = next_id(dirs).await?;
        copy.name = format!("{} 副本", self.name);

        let from = self.game_dir(dirs);
        let to = copy.game_dir(dirs);
        if let Err(source) = copy_in_background(from, to.clone(), &DUPLICATE_SKIP).await {
            // 複製到一半的資料夾不完整，留著會變成損壞的實例。
            let _ = tokio::fs::remove_dir_all(&to).await;
            return Err(InstanceError::Io { path: to, source });
        }
        copy.save(dirs).await?;
        Ok(copy)
    }

    /// 刪除實例和它的整個遊戲資料夾，包括存檔。
    pub async fn delete(&self, dirs: &DataDirs) -> Result<(), InstanceError> {
        let path = self.game_dir(dirs);
        tokio::fs::remove_dir_all(&path)
            .await
            .map_err(|source| InstanceError::Io { path, source })
    }

    /// 這個實例的遊戲目錄（存檔、設定、日誌）。
    pub fn game_dir(&self, dirs: &DataDirs) -> PathBuf {
        dirs.instances().join(&self.id)
    }

    /// 實際分配的記憶體。
    pub fn effective_memory_mb(&self, total_memory_mb: u32) -> u32 {
        if self.memory_mb == AUTO_MEMORY {
            auto_memory_mb(total_memory_mb)
        } else {
            self.memory_mb
        }
    }
}

/// 大量檔案複製放到背景執行緒，不佔用非同步工作。
async fn copy_in_background(
    from: PathBuf,
    to: PathBuf,
    skip: &'static [&'static str],
) -> io::Result<()> {
    tokio::task::spawn_blocking(move || {
        copy::copy_dir(&from, &to, skip)?;
        copy::link_or_copy_files(&mods::mods_dir(&from), &mods::mods_dir(&to))
    })
    .await
    .map_err(io::Error::other)?
}

fn clean_name(name: &str) -> Result<String, InstanceError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(InstanceError::EmptyName);
    }
    Ok(name.to_string())
}

/// 第一個沒被用過的 `instance-N`。看資料夾而不是實例清單：
/// 設定損壞的實例不在清單裡，但它的資料夾不能被新實例佔用。
async fn next_id(dirs: &DataDirs) -> Result<String, InstanceError> {
    let root = dirs.instances();
    let io_error = |source| InstanceError::Io {
        path: root.clone(),
        source,
    };
    let mut highest = 0;
    let mut entries = match tokio::fs::read_dir(&root).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok("instance-1".to_string());
        }
        Err(error) => return Err(io_error(error)),
    };
    while let Some(entry) = entries.next_entry().await.map_err(io_error)? {
        highest = highest.max(creation_order(&entry.file_name().to_string_lossy()));
    }
    Ok(format!("instance-{}", highest + 1))
}

fn creation_order(id: &str) -> u32 {
    id.rsplit('-')
        .next()
        .and_then(|number| number.parse().ok())
        .unwrap_or(0)
}

/// 用電腦記憶體的三分之一，並限制在 2 到 8 GB。
fn auto_memory_mb(total_memory_mb: u32) -> u32 {
    let share = (total_memory_mb / AUTO_MEMORY_SHARE).clamp(AUTO_MIN_MEMORY_MB, AUTO_MAX_MEMORY_MB);
    share / AUTO_MEMORY_STEP_MB * AUTO_MEMORY_STEP_MB
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dirs(name: &str) -> DataDirs {
        let root = std::env::temp_dir().join(format!("sapling-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        DataDirs::at(root)
    }

    #[tokio::test]
    async fn a_corrupted_instance_does_not_hide_the_others() {
        let dirs = temp_dirs("corrupted");
        let good = Instance::create(&dirs, "生存", "26.3").await.unwrap();
        let bad = Instance::create(&dirs, "創造", "26.3").await.unwrap();
        std::fs::write(bad.game_dir(&dirs).join("instance.json"), "{ not json").unwrap();

        let listed = Instance::list(&dirs).await.unwrap();
        assert_eq!(listed, [good]);

        let next = Instance::create(&dirs, "新的", "26.3").await.unwrap();
        assert_ne!(next.id, bad.id);
    }

    #[tokio::test]
    async fn duplicate_keeps_saves_but_not_logs() {
        let dirs = temp_dirs("duplicate");
        let source = Instance::create(&dirs, "生存", "26.3").await.unwrap();
        let game_dir = source.game_dir(&dirs);
        std::fs::create_dir_all(game_dir.join("saves/世界")).unwrap();
        std::fs::write(game_dir.join("saves/世界/level.dat"), "x").unwrap();
        std::fs::create_dir_all(game_dir.join("logs")).unwrap();
        std::fs::write(game_dir.join("logs/latest.log"), "x").unwrap();
        std::fs::create_dir_all(game_dir.join("mods")).unwrap();
        std::fs::write(game_dir.join("mods/a.jar"), "m").unwrap();

        let copy = source.duplicate(&dirs).await.unwrap();

        assert_ne!(copy.id, source.id);
        assert_eq!(copy.name, "生存 副本");
        let copied = copy.game_dir(&dirs);
        assert!(copied.join("saves/世界/level.dat").exists());
        assert!(!copied.join("logs").exists());
        assert_eq!(std::fs::read(copied.join("mods/a.jar")).unwrap(), b"m");
        assert_eq!(Instance::load(&dirs, &copy.id).await.unwrap(), copy);
    }

    #[tokio::test]
    async fn creates_lists_and_deletes_instances() {
        let dirs = temp_dirs("instances");
        let first = Instance::create(&dirs, "生存", "26.3").await.unwrap();
        let second = Instance::create(&dirs, "創造", "1.20.1").await.unwrap();
        assert_ne!(first.id, second.id);

        let ids: Vec<String> = Instance::list(&dirs)
            .await
            .unwrap()
            .into_iter()
            .map(|instance| instance.id)
            .collect();
        assert_eq!(ids, [first.id.clone(), second.id.clone()]);

        first.delete(&dirs).await.unwrap();
        assert_eq!(Instance::list(&dirs).await.unwrap(), [second]);
    }

    #[tokio::test]
    async fn saves_settings_and_keeps_memory_in_range() {
        let dirs = temp_dirs("settings");
        let mut instance = Instance::create(&dirs, "生存", "26.3").await.unwrap();
        instance.name = "  新名字  ".to_string();
        instance.memory_mb = 1;
        instance.save(&dirs).await.unwrap();

        let loaded = Instance::load(&dirs, &instance.id).await.unwrap();
        assert_eq!(loaded.name, "新名字");
        assert_eq!(loaded.memory_mb, MIN_MEMORY_MB);
    }

    #[tokio::test]
    async fn rejects_an_empty_name() {
        let dirs = temp_dirs("empty-name");
        assert!(matches!(
            Instance::create(&dirs, "   ", "26.3").await,
            Err(InstanceError::EmptyName)
        ));
    }

    #[test]
    fn automatic_memory_follows_the_computer() {
        assert_eq!(auto_memory_mb(4096), 2048);
        assert_eq!(auto_memory_mb(16384), 5120);
        assert_eq!(auto_memory_mb(65536), 8192);
    }
}
