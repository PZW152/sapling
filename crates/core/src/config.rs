//! 啟動器設定：`config.json`。

use serde::{Deserialize, Serialize};

use crate::download::DEFAULT_CONCURRENCY;
use crate::platform::DataDirs;
use crate::storage::{StorageError, read_json, write_json};

const DEFAULT_THEME: &str = "sapling";
const RETIRED_THEME: &str = "zhongshan";
const MIN_CONCURRENCY: usize = 1;
const MAX_CONCURRENCY: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LauncherConfig {
    pub download_concurrency: usize,
    /// 目前選中的實例，空字串代表還沒選。
    pub selected_instance: String,
    /// 預設綠色主題的名稱。
    pub theme: String,
    /// 用戶關閉的可選功能模塊。
    pub disabled_features: Vec<String>,
    /// 功能模塊開關平時隱藏，在關於頁連按版本號才顯示。
    pub feature_switches_unlocked: bool,
    /// 植樹節種過樹的年份。
    pub planted_years: Vec<i32>,
    /// 下次啟動是否用離線模式。
    pub offline_enabled: bool,
    pub offline_name: String,
    /// 收藏的資源，內容由介面決定。
    pub favorites: Vec<serde_json::Value>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            download_concurrency: DEFAULT_CONCURRENCY,
            selected_instance: String::new(),
            theme: DEFAULT_THEME.to_string(),
            disabled_features: Vec::new(),
            feature_switches_unlocked: false,
            planted_years: Vec::new(),
            offline_enabled: false,
            offline_name: String::new(),
            favorites: Vec::new(),
        }
    }
}

impl LauncherConfig {
    pub async fn load(dirs: &DataDirs) -> Result<Self, StorageError> {
        let path = dirs.config_file();
        match read_json::<Self>(&path).await {
            Ok(config) => Ok(config.unwrap_or_default().normalized()),
            // 損壞的設定不能讓啟動器和遊戲都打不開。留一份副本，用預設值繼續。
            Err(StorageError::Corrupted { .. }) => {
                let _ = tokio::fs::rename(&path, path.with_extension("json.bad")).await;
                Ok(Self::default())
            }
            Err(error) => Err(error),
        }
    }

    pub async fn save(&self, dirs: &DataDirs) -> Result<(), StorageError> {
        write_json(&dirs.config_file(), &self.clone().normalized()).await
    }

    pub fn is_feature_enabled(&self, id: &str) -> bool {
        !self.disabled_features.iter().any(|disabled| disabled == id)
    }

    fn normalized(mut self) -> Self {
        self.download_concurrency = self
            .download_concurrency
            .clamp(MIN_CONCURRENCY, MAX_CONCURRENCY);
        // 中山主題改成植樹節限定後，舊設定裡的值要換回預設主題。
        if self.theme == RETIRED_THEME {
            self.theme = DEFAULT_THEME.to_string();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dirs(name: &str) -> DataDirs {
        let root =
            std::env::temp_dir().join(format!("sapling-config-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        DataDirs::at(root)
    }

    #[tokio::test]
    async fn a_corrupted_config_is_kept_aside_and_defaults_are_used() {
        let dirs = temp_dirs("corrupted");
        std::fs::write(dirs.config_file(), "{ not json").unwrap();

        let config = LauncherConfig::load(&dirs).await.unwrap();

        assert_eq!(config, LauncherConfig::default());
        assert_eq!(
            std::fs::read_to_string(dirs.config_file().with_extension("json.bad")).unwrap(),
            "{ not json"
        );
    }

    #[tokio::test]
    async fn the_retired_theme_falls_back_to_the_default() {
        let dirs = temp_dirs("theme");
        std::fs::write(dirs.config_file(), r#"{ "theme": "zhongshan" }"#).unwrap();

        assert_eq!(
            LauncherConfig::load(&dirs).await.unwrap().theme,
            DEFAULT_THEME
        );
    }

    #[test]
    fn missing_fields_use_the_default() {
        let config: LauncherConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.download_concurrency, DEFAULT_CONCURRENCY);
    }

    #[test]
    fn concurrency_stays_in_a_safe_range() {
        let too_high = LauncherConfig {
            download_concurrency: 9999,
            ..LauncherConfig::default()
        };
        assert_eq!(too_high.normalized().download_concurrency, MAX_CONCURRENCY);
    }
}
