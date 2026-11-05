//! 實例：一個獨立遊戲資料夾 + 遊戲版本 + 載入器 + 設定。

pub mod mods;

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
    /// 分配給遊戲的記憶體，`AUTO_MEMORY` 代表自動。
    pub memory_mb: u32,
    /// 額外的 Java 參數，用空白分隔。
    pub java_args: String,
}

impl Instance {
    /// 建立並保存一個新實例。
    pub async fn create(dirs: &DataDirs, name: &str, version: &str) -> Result<Self, InstanceError> {
        let name = clean_name(name)?;
        let instance = Self {
            id: next_id(&Self::list(dirs).await?),
            name,
            version: version.to_string(),
            loader: LoaderKind::Vanilla,
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
            let saved: Option<Self> = read_json(&entry.path().join("instance.json")).await?;
            instances.extend(saved);
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

fn clean_name(name: &str) -> Result<String, InstanceError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(InstanceError::EmptyName);
    }
    Ok(name.to_string())
}

/// 第一個沒被用過的 `instance-N`。名稱不用於資料夾，所以可以隨時改名。
fn next_id(instances: &[Instance]) -> String {
    let next = instances
        .iter()
        .map(|instance| creation_order(&instance.id))
        .max()
        .map_or(1, |highest| highest + 1);
    format!("instance-{next}")
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
