//! 從其他地方匯入世界：官方啟動器的 `.minecraft`，以及舊版 Sapling 的資料夾。

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::copy::{LOCK_FILE, copy_dir};
use super::worlds::{self, World};
use super::{Instance, InstanceError};
use crate::platform::{DataDirs, legacy_data_dir, official_game_dir};

/// 每個來源最多列出幾個世界。
const WORLD_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSource {
    /// 顯示給用戶看的名稱。
    pub label: String,
    /// 這個來源的遊戲資料夾。
    pub path: PathBuf,
    pub worlds: Vec<World>,
}

/// 找出這台電腦上有世界可以匯入的地方。
pub async fn find_sources(dirs: &DataDirs) -> Vec<ImportSource> {
    let mut candidates = Vec::new();
    if let Some(path) = official_game_dir() {
        candidates.push(("官方啟動器".to_string(), path));
    }
    if let Some(legacy) = legacy_data_dir().filter(|path| path != dirs.root()) {
        let legacy = DataDirs::at(legacy);
        for instance in Instance::list(&legacy).await.unwrap_or_default() {
            let label = format!("舊版 Sapling：{}", instance.name);
            candidates.push((label, instance.game_dir(&legacy)));
        }
    }

    let mut sources = Vec::new();
    for (label, path) in candidates {
        let found = worlds::recent(&path, WORLD_LIMIT).await.unwrap_or_default();
        if !found.is_empty() {
            sources.push(ImportSource {
                label,
                path,
                worlds: found,
            });
        }
    }
    sources
}

/// 把世界複製到實例，回傳新的資料夾名稱。來源不會被改動。
pub async fn import_world(
    source: &Path,
    folder: &str,
    game_dir: &Path,
) -> Result<String, InstanceError> {
    if !worlds::exists(source, folder) {
        return Err(InstanceError::NotFound);
    }
    let saves = game_dir.join("saves");
    let name = free_name(&saves, folder);
    let from = source.join("saves").join(folder);
    let to = saves.join(&name);
    let target = to.clone();
    let copied = tokio::task::spawn_blocking(move || copy_dir(&from, &target, &[LOCK_FILE]))
        .await
        .map_err(std::io::Error::other)
        .and_then(|result| result);
    if let Err(source) = copied {
        let _ = tokio::fs::remove_dir_all(&to).await;
        return Err(InstanceError::Io { path: to, source });
    }
    Ok(name)
}

/// 實例裡已經有同名世界時，不要覆蓋它。
fn free_name(saves: &Path, folder: &str) -> String {
    if !saves.join(folder).exists() {
        return folder.to_string();
    }
    (2..)
        .map(|number| format!("{folder} ({number})"))
        .find(|name| !saves.join(name).exists())
        .unwrap_or_else(|| format!("{folder} (匯入)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sapling-import-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn add_world(game_dir: &Path, name: &str) {
        let world = game_dir.join("saves").join(name);
        std::fs::create_dir_all(&world).unwrap();
        std::fs::write(world.join("level.dat"), "x").unwrap();
    }

    #[tokio::test]
    async fn copies_the_world_and_leaves_the_source_alone() {
        let source = folder("copy-source");
        let target = folder("copy-target");
        add_world(&source, "生存");

        let name = import_world(&source, "生存", &target).await.unwrap();

        assert_eq!(name, "生存");
        assert!(target.join("saves/生存/level.dat").exists());
        assert!(source.join("saves/生存/level.dat").exists());
    }

    #[tokio::test]
    async fn never_overwrites_a_world_with_the_same_name() {
        let source = folder("same-source");
        let target = folder("same-target");
        add_world(&source, "生存");
        add_world(&target, "生存");

        let name = import_world(&source, "生存", &target).await.unwrap();

        assert_eq!(name, "生存 (2)");
    }

    #[tokio::test]
    async fn refuses_a_folder_outside_saves() {
        let source = folder("escape-source");
        let target = folder("escape-target");

        assert!(matches!(
            import_world(&source, "../secret", &target).await,
            Err(InstanceError::NotFound)
        ));
    }
}
