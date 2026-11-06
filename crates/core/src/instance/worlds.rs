//! 實例裡的單人世界：讀取最近遊玩的世界，讓首頁可以直接回到剛才的進度。

use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::InstanceError;

const LEVEL_FILE: &str = "level.dat";
const ICON_FILE: &str = "icon.png";
/// 遊戲的世界縮圖很小，超過這個大小就不是正常的縮圖。
const MAX_ICON_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    /// 世界的資料夾名稱，直接進入世界時要用它。
    pub folder: String,
    /// 上次保存的時間，從 1970 年起算的毫秒數。
    pub last_played_ms: u64,
    /// `icon.png` 的內容。沒有玩過或舊世界可能沒有。
    pub icon: Option<Vec<u8>>,
}

fn saves_dir(game_dir: &Path) -> PathBuf {
    game_dir.join("saves")
}

/// 最近遊玩的世界，新的在前。沒有 `saves` 資料夾就是空清單。
pub async fn recent(game_dir: &Path, limit: usize) -> Result<Vec<World>, InstanceError> {
    let dir = saves_dir(game_dir);
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
        // 沒有 level.dat 的資料夾不是世界，例如玩家自己放的備份。
        let Ok(level) = tokio::fs::metadata(entry.path().join(LEVEL_FILE)).await else {
            continue;
        };
        let modified = level.modified().unwrap_or(UNIX_EPOCH);
        found.push((modified, entry));
    }
    found.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));

    let mut worlds = Vec::new();
    for (modified, entry) in found.into_iter().take(limit) {
        worlds.push(World {
            folder: entry.file_name().to_string_lossy().to_string(),
            last_played_ms: millis_since_epoch(modified),
            icon: read_icon(&entry.path().join(ICON_FILE)).await,
        });
    }
    Ok(worlds)
}

/// 確認這個名稱真的是實例裡的世界，不會指到 `saves` 以外的地方。
pub fn exists(game_dir: &Path, folder: &str) -> bool {
    let mut parts = Path::new(folder).components();
    let single_name = matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none();
    single_name && saves_dir(game_dir).join(folder).join(LEVEL_FILE).is_file()
}

/// 縮圖只是裝飾，讀不到就當作沒有。
async fn read_icon(path: &Path) -> Option<Vec<u8>> {
    let metadata = tokio::fs::metadata(path).await.ok()?;
    if metadata.len() > MAX_ICON_BYTES {
        return None;
    }
    tokio::fs::read(path).await.ok()
}

fn millis_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::time::Duration;

    use super::*;

    fn game_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sapling-worlds-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(saves_dir(&dir)).unwrap();
        dir
    }

    fn add_world(dir: &Path, name: &str, seconds: u64) {
        let world = saves_dir(dir).join(name);
        std::fs::create_dir_all(&world).unwrap();
        let level = File::create(world.join(LEVEL_FILE)).unwrap();
        level
            .set_modified(UNIX_EPOCH + Duration::from_secs(seconds))
            .unwrap();
    }

    #[tokio::test]
    async fn newest_world_comes_first_and_limit_applies() {
        let dir = game_dir("order");
        add_world(&dir, "old", 100);
        add_world(&dir, "new", 300);
        add_world(&dir, "middle", 200);

        let worlds = recent(&dir, 2).await.unwrap();

        let names: Vec<_> = worlds.iter().map(|world| world.folder.as_str()).collect();
        assert_eq!(names, ["new", "middle"]);
        assert_eq!(worlds[0].last_played_ms, 300_000);
    }

    #[tokio::test]
    async fn skips_folders_without_level_data() {
        let dir = game_dir("skip");
        add_world(&dir, "real", 100);
        std::fs::create_dir_all(saves_dir(&dir).join("backup")).unwrap();

        let worlds = recent(&dir, 5).await.unwrap();

        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].folder, "real");
    }

    #[tokio::test]
    async fn no_saves_folder_means_no_worlds() {
        let dir = game_dir("empty");
        std::fs::remove_dir_all(saves_dir(&dir)).unwrap();

        assert!(recent(&dir, 5).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn reads_the_icon_and_ignores_an_oversized_one() {
        let dir = game_dir("icon");
        add_world(&dir, "small", 200);
        add_world(&dir, "huge", 100);
        let icon = |name: &str| saves_dir(&dir).join(name).join(ICON_FILE);
        std::fs::write(icon("small"), [1, 2, 3]).unwrap();
        std::fs::write(icon("huge"), vec![0; MAX_ICON_BYTES as usize + 1]).unwrap();

        let worlds = recent(&dir, 5).await.unwrap();

        assert_eq!(worlds[0].icon, Some(vec![1, 2, 3]));
        assert_eq!(worlds[1].icon, None);
    }

    #[test]
    fn only_a_plain_folder_name_counts_as_a_world() {
        let dir = game_dir("exists");
        add_world(&dir, "real", 100);

        assert!(exists(&dir, "real"));
        assert!(!exists(&dir, "missing"));
        assert!(!exists(&dir, "../real"));
        assert!(!exists(&dir, "real/../real"));
        assert!(!exists(&dir, ""));
    }
}
