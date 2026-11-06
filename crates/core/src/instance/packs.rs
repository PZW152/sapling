//! 實例裡的資源包和光影。可以是 `.zip`，也可以是解開的資料夾。

use std::io;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::InstanceError;

const ARCHIVE_SUFFIX: &str = ".zip";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackKind {
    ResourcePack,
    Shader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    pub name: String,
    pub size: u64,
}

pub fn pack_dir(game_dir: &Path, kind: PackKind) -> PathBuf {
    game_dir.join(match kind {
        PackKind::ResourcePack => "resourcepacks",
        PackKind::Shader => "shaderpacks",
    })
}

pub async fn list(game_dir: &Path, kind: PackKind) -> Result<Vec<PackFile>, InstanceError> {
    let dir = pack_dir(game_dir, kind);
    let io_error = |source| InstanceError::Io {
        path: dir.clone(),
        source,
    };
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io_error(error)),
    };
    let mut packs = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(io_error)? {
        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata().await.map_err(io_error)?;
        if metadata.is_dir() || name.to_lowercase().ends_with(ARCHIVE_SUFFIX) {
            packs.push(PackFile {
                name,
                // 資料夾沒有現成的大小，不值得為了顯示走訪全部檔案。
                size: if metadata.is_dir() { 0 } else { metadata.len() },
            });
        }
    }
    packs.sort_by_key(|pack| pack.name.to_lowercase());
    Ok(packs)
}

pub async fn delete(game_dir: &Path, kind: PackKind, name: &str) -> Result<(), InstanceError> {
    let mut parts = Path::new(name).components();
    let single_name = matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none();
    if !single_name {
        return Err(InstanceError::NotFound);
    }
    let path = pack_dir(game_dir, kind).join(name);
    let removed = if path.is_dir() {
        tokio::fs::remove_dir_all(&path).await
    } else {
        tokio::fs::remove_file(&path).await
    };
    removed.map_err(|source| InstanceError::Io { path, source })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sapling-packs-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(pack_dir(&dir, PackKind::ResourcePack)).unwrap();
        dir
    }

    #[tokio::test]
    async fn lists_archives_and_folders_but_not_other_files() {
        let dir = game_dir("list");
        let packs = pack_dir(&dir, PackKind::ResourcePack);
        std::fs::write(packs.join("b.zip"), "12345").unwrap();
        std::fs::write(packs.join("notes.txt"), "x").unwrap();
        std::fs::create_dir(packs.join("a")).unwrap();

        let found = list(&dir, PackKind::ResourcePack).await.unwrap();

        assert_eq!(
            found,
            [
                PackFile {
                    name: "a".to_string(),
                    size: 0
                },
                PackFile {
                    name: "b.zip".to_string(),
                    size: 5
                },
            ]
        );
    }

    #[tokio::test]
    async fn a_missing_folder_is_an_empty_list() {
        let dir = game_dir("missing");

        assert!(list(&dir, PackKind::Shader).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn deletes_archives_and_folders() {
        let dir = game_dir("delete");
        let packs = pack_dir(&dir, PackKind::ResourcePack);
        std::fs::write(packs.join("b.zip"), "x").unwrap();
        std::fs::create_dir(packs.join("a")).unwrap();
        std::fs::write(packs.join("a").join("pack.mcmeta"), "x").unwrap();

        delete(&dir, PackKind::ResourcePack, "b.zip").await.unwrap();
        delete(&dir, PackKind::ResourcePack, "a").await.unwrap();

        assert!(list(&dir, PackKind::ResourcePack).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn refuses_names_outside_the_folder() {
        let dir = game_dir("escape");
        std::fs::write(dir.join("keep.zip"), "x").unwrap();

        let outcome = delete(&dir, PackKind::ResourcePack, "../keep.zip").await;

        assert!(matches!(outcome, Err(InstanceError::NotFound)));
        assert!(dir.join("keep.zip").exists());
    }
}
