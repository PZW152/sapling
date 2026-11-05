//! 實例：一個獨立遊戲資料夾 + 遊戲版本 + 載入器 + 設定。
//! v1 介面只用一個實例，但資料按多實例存放。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::loader::LoaderKind;
use crate::platform::DataDirs;
use crate::storage::{StorageError, read_json, write_json};

pub const DEFAULT_INSTANCE_ID: &str = "default";

const DEFAULT_MEMORY_MB: u32 = 4096;
const MIN_MEMORY_MB: u32 = 1024;
const MAX_MEMORY_MB: u32 = 65536;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    /// 遊戲版本，空字串代表還沒選過。
    pub version: String,
    pub loader: LoaderKind,
    pub memory_mb: u32,
    /// 額外的 Java 參數，用空白分隔。
    pub java_args: String,
}

impl Instance {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: "我的世界".to_string(),
            version: String::new(),
            loader: LoaderKind::Vanilla,
            memory_mb: DEFAULT_MEMORY_MB,
            java_args: String::new(),
        }
    }

    /// 讀取實例；不存在時回傳一個尚未保存的新實例。
    pub async fn load_or_new(dirs: &DataDirs, id: &str) -> Result<Self, StorageError> {
        let saved: Option<Self> = read_json(&Self::file(dirs, id)).await?;
        Ok(saved.unwrap_or_else(|| Self::new(id)))
    }

    pub async fn save(&self, dirs: &DataDirs) -> Result<(), StorageError> {
        let mut instance = self.clone();
        instance.memory_mb = instance.memory_mb.clamp(MIN_MEMORY_MB, MAX_MEMORY_MB);
        write_json(&Self::file(dirs, &self.id), &instance).await
    }

    /// 這個實例的遊戲目錄（存檔、設定、日誌）。
    pub fn game_dir(&self, dirs: &DataDirs) -> PathBuf {
        dirs.instances().join(&self.id)
    }

    fn file(dirs: &DataDirs, id: &str) -> PathBuf {
        dirs.instances().join(id).join("instance.json")
    }
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
    async fn saves_and_loads_settings() {
        let dirs = temp_dirs("instance");
        let mut instance = Instance::load_or_new(&dirs, DEFAULT_INSTANCE_ID).await.unwrap();
        assert_eq!(instance.memory_mb, DEFAULT_MEMORY_MB);

        instance.version = "26.3".to_string();
        instance.memory_mb = 8192;
        instance.save(&dirs).await.unwrap();

        let loaded = Instance::load_or_new(&dirs, DEFAULT_INSTANCE_ID).await.unwrap();
        assert_eq!(loaded, instance);
    }

    #[tokio::test]
    async fn memory_is_kept_in_a_safe_range() {
        let dirs = temp_dirs("memory");
        let mut instance = Instance::new(DEFAULT_INSTANCE_ID);
        instance.memory_mb = 1;
        instance.save(&dirs).await.unwrap();

        let loaded = Instance::load_or_new(&dirs, DEFAULT_INSTANCE_ID).await.unwrap();
        assert_eq!(loaded.memory_mb, MIN_MEMORY_MB);
    }
}
