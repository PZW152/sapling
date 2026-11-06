//! 遊戲截圖：實例 `screenshots` 資料夾裡的 PNG。

use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use super::InstanceError;

const FOLDER: &str = "screenshots";
const SUFFIX: &str = ".png";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub name: String,
    pub path: PathBuf,
    pub taken_ms: u64,
}

pub fn folder(game_dir: &Path) -> PathBuf {
    game_dir.join(FOLDER)
}

/// 列出截圖，新的在前。
pub async fn list(game_dir: &Path) -> Result<Vec<Screenshot>, InstanceError> {
    let dir = folder(game_dir);
    let io_error = |source| InstanceError::Io {
        path: dir.clone(),
        source,
    };
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error(error)),
    };
    let mut found = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(io_error)? {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.to_lowercase().ends_with(SUFFIX) {
            continue;
        }
        let metadata = entry.metadata().await.map_err(io_error)?;
        let taken_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |elapsed| elapsed.as_millis() as u64);
        found.push(Screenshot {
            name,
            path: entry.path(),
            taken_ms,
        });
    }
    found.sort_by_key(|shot| std::cmp::Reverse(shot.taken_ms));
    Ok(found)
}

pub async fn delete(game_dir: &Path, name: &str) -> Result<(), InstanceError> {
    let mut parts = Path::new(name).components();
    let single_name = matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none();
    if !single_name || !name.to_lowercase().ends_with(SUFFIX) {
        return Err(InstanceError::NotFound);
    }
    let path = folder(game_dir).join(name);
    tokio::fs::remove_file(&path)
        .await
        .map_err(|source| InstanceError::Io { path, source })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sapling-shots-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(folder(&dir)).unwrap();
        dir
    }

    #[tokio::test]
    async fn lists_only_png_files() {
        let dir = game_dir("list");
        std::fs::write(folder(&dir).join("a.png"), "x").unwrap();
        std::fs::write(folder(&dir).join("notes.txt"), "x").unwrap();

        let found = list(&dir).await.unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "a.png");
    }

    #[tokio::test]
    async fn deletes_only_screenshots_in_the_folder() {
        let dir = game_dir("delete");
        std::fs::write(folder(&dir).join("a.png"), "x").unwrap();
        std::fs::write(dir.join("keep.png"), "x").unwrap();

        assert!(matches!(
            delete(&dir, "../keep.png").await,
            Err(InstanceError::NotFound)
        ));
        delete(&dir, "a.png").await.unwrap();

        assert!(list(&dir).await.unwrap().is_empty());
        assert!(dir.join("keep.png").exists());
    }
}
