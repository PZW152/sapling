//! 實例圖示：存在實例資料夾裡，複製實例時跟著走。

use std::io;
use std::path::{Path, PathBuf};

use super::InstanceError;

const ICON_FILE: &str = "icon";
/// 圖示只顯示在清單裡，太大的圖片沒有意義，也會拖慢介面。
const MAX_BYTES: usize = 2 * 1024 * 1024;

fn path(game_dir: &Path) -> PathBuf {
    game_dir.join(ICON_FILE)
}

/// 看開頭判斷是不是圖片。副檔名可能是錯的。
fn is_image(bytes: &[u8]) -> bool {
    let png = bytes.starts_with(&[0x89, b'P', b'N', b'G']);
    let jpeg = bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
    let webp = bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP";
    let gif = bytes.starts_with(b"GIF8");
    png || jpeg || webp || gif
}

pub async fn read(game_dir: &Path) -> Option<Vec<u8>> {
    tokio::fs::read(path(game_dir)).await.ok()
}

/// 保存圖示。不是圖片或太大時回傳錯誤。
pub async fn save(game_dir: &Path, bytes: &[u8]) -> Result<(), InstanceError> {
    if !is_image(bytes) || bytes.len() > MAX_BYTES {
        return Err(InstanceError::BadIcon);
    }
    let target = path(game_dir);
    tokio::fs::write(&target, bytes)
        .await
        .map_err(|source| InstanceError::Io {
            path: target,
            source,
        })
}

pub async fn remove(game_dir: &Path) -> Result<(), InstanceError> {
    let target = path(game_dir);
    match tokio::fs::remove_file(&target).await {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(InstanceError::Io {
            path: target,
            source: error,
        }),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sapling-icon-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn saves_reads_and_removes_an_image() {
        let dir = game_dir("round");
        let png = [0x89, b'P', b'N', b'G', 1, 2, 3];

        save(&dir, &png).await.unwrap();
        assert_eq!(read(&dir).await.unwrap(), png);

        remove(&dir).await.unwrap();
        assert_eq!(read(&dir).await, None);
        remove(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn refuses_files_that_are_not_images_or_too_big() {
        let dir = game_dir("refuse");
        let mut big = vec![0xFF, 0xD8, 0xFF];
        big.resize(MAX_BYTES + 1, 0);

        assert!(matches!(
            save(&dir, b"text").await,
            Err(InstanceError::BadIcon)
        ));
        assert!(matches!(
            save(&dir, &big).await,
            Err(InstanceError::BadIcon)
        ));
    }
}
