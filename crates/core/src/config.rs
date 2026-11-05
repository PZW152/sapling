//! 啟動器設定：`config.json`。

use serde::{Deserialize, Serialize};

use crate::download::DEFAULT_CONCURRENCY;
use crate::platform::DataDirs;
use crate::storage::{StorageError, read_json, write_json};

const DEFAULT_THEME: &str = "sapling";
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
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            download_concurrency: DEFAULT_CONCURRENCY,
            selected_instance: String::new(),
            theme: DEFAULT_THEME.to_string(),
            disabled_features: Vec::new(),
            feature_switches_unlocked: false,
        }
    }
}

impl LauncherConfig {
    pub async fn load(dirs: &DataDirs) -> Result<Self, StorageError> {
        let config: Option<Self> = read_json(&dirs.config_file()).await?;
        Ok(config.unwrap_or_default().normalized())
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
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
