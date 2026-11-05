//! 讀寫設定用的 JSON 檔案。

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("無法讀寫設定檔案：{}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("設定檔案已損壞：{}", path.display())]
    Corrupted {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

/// 讀取 JSON 檔案。檔案不存在時回傳 `None`。
pub async fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, StorageError> {
    let text = match tokio::fs::read_to_string(path).await {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(StorageError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|source| StorageError::Corrupted {
            path: path.to_path_buf(),
            source,
        })
}

pub async fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    let corrupted = |source| StorageError::Corrupted {
        path: path.to_path_buf(),
        source,
    };
    let io_error = |source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    };
    let text = serde_json::to_string_pretty(value).map_err(corrupted)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(io_error)?;
    }
    // 先寫到暫存檔再換名，中途斷電或崩潰時舊檔案仍然完整。
    let temporary = path.with_extension("json.tmp");
    tokio::fs::write(&temporary, text).await.map_err(io_error)?;
    tokio::fs::rename(&temporary, path).await.map_err(io_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn replaces_the_file_and_leaves_no_temporary_file() {
        let dir = std::env::temp_dir().join(format!("sapling-storage-{}", std::process::id()));
        let path = dir.join("config.json");

        write_json(&path, &1).await.unwrap();
        write_json(&path, &2).await.unwrap();

        assert_eq!(read_json::<i32>(&path).await.unwrap(), Some(2));
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
