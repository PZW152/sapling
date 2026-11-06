//! 把電腦上的檔案加進實例：看內容判斷是模組、資源包還是光影，放進對應的資料夾。

use std::fs::File;
use std::path::Path;

use serde::Serialize;
use zip::ZipArchive;

use super::InstanceError;
use super::mods::mods_dir;
use super::packs::{PackKind, pack_dir};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AddedKind {
    Mod,
    ResourcePack,
    Shader,
}

/// 複製檔案進實例，回傳它被當成什麼。原始檔案不會被改動。
pub async fn add_file(
    game_dir: &Path,
    source: &Path,
    supports_mods: bool,
) -> Result<AddedKind, InstanceError> {
    let name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .ok_or(InstanceError::NotFound)?;
    let unknown = || InstanceError::UnknownFile { name: name.clone() };
    let path = source.to_path_buf();
    let kind = tokio::task::spawn_blocking(move || classify(&path))
        .await
        .ok()
        .flatten()
        .ok_or_else(unknown)?;
    if kind == AddedKind::Mod && !supports_mods {
        return Err(InstanceError::ModsNeedLoader);
    }
    let folder = match kind {
        AddedKind::Mod => mods_dir(game_dir),
        AddedKind::ResourcePack => pack_dir(game_dir, PackKind::ResourcePack),
        AddedKind::Shader => pack_dir(game_dir, PackKind::Shader),
    };
    let io_error = |source| InstanceError::Io {
        path: folder.clone(),
        source,
    };
    tokio::fs::create_dir_all(&folder).await.map_err(io_error)?;
    tokio::fs::copy(source, folder.join(&name))
        .await
        .map_err(io_error)?;
    Ok(kind)
}

fn classify(path: &Path) -> Option<AddedKind> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    if extension == "jar" {
        return Some(AddedKind::Mod);
    }
    if extension != "zip" {
        return None;
    }
    let archive = ZipArchive::new(File::open(path).ok()?).ok()?;
    let has = |prefix: &str| {
        archive
            .file_names()
            .flatten()
            .any(|name| name.starts_with(prefix))
    };
    if has("shaders/") {
        return Some(AddedKind::Shader);
    }
    // 只有 `data/` 的是資料包，要裝進世界，放資源包資料夾遊戲不會讀。
    (has("pack.mcmeta") && has("assets/")).then_some(AddedKind::ResourcePack)
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;

    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sapling-add-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn zip_with(path: &Path, entries: &[&str]) {
        let mut writer = ZipWriter::new(File::create(path).unwrap());
        for entry in entries {
            writer
                .start_file(*entry, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"x").unwrap();
        }
        writer.finish().unwrap();
    }

    #[tokio::test]
    async fn puts_each_kind_of_file_in_its_folder() {
        let dir = folder("kinds");
        let game = dir.join("game");
        let jar = dir.join("a.jar");
        let pack = dir.join("pack.zip");
        let shader = dir.join("shader.zip");
        std::fs::write(&jar, "x").unwrap();
        zip_with(&pack, &["pack.mcmeta", "assets/x.png"]);
        zip_with(&shader, &["shaders/world0/a.fsh"]);

        assert_eq!(add_file(&game, &jar, true).await.unwrap(), AddedKind::Mod);
        assert_eq!(
            add_file(&game, &pack, true).await.unwrap(),
            AddedKind::ResourcePack
        );
        assert_eq!(
            add_file(&game, &shader, true).await.unwrap(),
            AddedKind::Shader
        );

        assert!(game.join("mods/a.jar").exists());
        assert!(game.join("resourcepacks/pack.zip").exists());
        assert!(game.join("shaderpacks/shader.zip").exists());
        assert!(jar.exists());
    }

    #[tokio::test]
    async fn explains_what_cannot_be_added() {
        let dir = folder("refuse");
        let game = dir.join("game");
        let jar = dir.join("a.jar");
        let data_pack = dir.join("data.zip");
        let text = dir.join("notes.txt");
        std::fs::write(&jar, "x").unwrap();
        std::fs::write(&text, "x").unwrap();
        zip_with(&data_pack, &["pack.mcmeta", "data/x.json"]);

        assert!(matches!(
            add_file(&game, &jar, false).await,
            Err(InstanceError::ModsNeedLoader)
        ));
        assert!(matches!(
            add_file(&game, &data_pack, true).await,
            Err(InstanceError::UnknownFile { .. })
        ));
        assert!(matches!(
            add_file(&game, &text, true).await,
            Err(InstanceError::UnknownFile { .. })
        ));
    }
}
