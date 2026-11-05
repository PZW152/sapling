//! 啟動器設定：`config.json`。

use serde::{Deserialize, Serialize};

use crate::download::DEFAULT_CONCURRENCY;
use crate::platform::DataDirs;
use crate::storage::{StorageError, read_json, write_json};

const MIN_CONCURRENCY: usize = 1;
const MAX_CONCURRENCY: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LauncherConfig {
    pub download_concurrency: usize,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            download_concurrency: DEFAULT_CONCURRENCY,
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
        };
        assert_eq!(too_high.normalized().download_concurrency, MAX_CONCURRENCY);
    }
}
