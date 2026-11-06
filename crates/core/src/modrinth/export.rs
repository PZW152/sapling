//! 把實例匯出成 `.mrpack`，別的啟動器也能打開。
//!
//! Modrinth 上有的模組只寫下載網址，其餘的檔案放進壓縮檔。存檔和日誌不匯出：
//! 整合包是一套玩法，不是個人備份。

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::pack::{FORMAT_VERSION, GAME, INDEX_FILE, LOADERS, OVERRIDES};
use super::{ModVersion, ModrinthError, installed};
use crate::download::file_sha1;
use crate::instance::Instance;
use crate::instance::mods;
use crate::platform::DataDirs;

/// 跟著整合包走的設定和資源。
const SHARED_ITEMS: [&str; 4] = ["config", "resourcepacks", "shaderpacks", "options.txt"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    /// 寫成下載網址的模組數。
    pub linked_mods: usize,
    /// 查不到來源、放進壓縮檔的模組數。
    pub bundled_mods: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportIndex {
    format_version: u32,
    game: &'static str,
    version_id: String,
    name: String,
    files: Vec<ExportFile>,
    dependencies: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportFile {
    path: String,
    hashes: ExportHashes,
    downloads: Vec<String>,
    file_size: u64,
}

#[derive(Debug, Serialize)]
struct ExportHashes {
    sha1: String,
    sha512: String,
}

/// 匯出實例。沒有網路時所有模組都放進壓縮檔，一樣能用，只是檔案比較大。
pub async fn export_pack(
    client: &reqwest::Client,
    dirs: &DataDirs,
    instance: &Instance,
    target: &Path,
) -> Result<ExportSummary, ModrinthError> {
    let game_dir = instance.game_dir(dirs);
    let local = enabled_mods(&game_dir).await?;
    let hashes: Vec<&str> = local.iter().map(|(_, hash)| hash.as_str()).collect();
    let known: HashMap<String, ModVersion> = if hashes.is_empty() {
        HashMap::new()
    } else {
        installed::post(
            client,
            "version_files",
            &serde_json::json!({ "hashes": hashes, "algorithm": "sha1" }),
        )
        .await
        .unwrap_or_default()
    };

    let mut files = Vec::new();
    let mut bundled = Vec::new();
    for (name, hash) in &local {
        match known
            .get(hash)
            .and_then(|version| linked_file(version, name, hash))
        {
            Some(file) => files.push(file),
            None => bundled.push(name.clone()),
        }
    }
    let summary = ExportSummary {
        linked_mods: files.len(),
        bundled_mods: bundled.len(),
    };
    let index = ExportIndex {
        format_version: FORMAT_VERSION,
        game: GAME,
        version_id: "1.0.0".to_string(),
        name: instance.name.clone(),
        files,
        dependencies: dependencies(instance),
    };
    let json = serde_json::to_vec_pretty(&index)?;
    let target = target.to_path_buf();
    tokio::task::spawn_blocking(move || write_archive(&target, &json, &game_dir, &bundled))
        .await
        .map_err(io::Error::other)
        .and_then(|written| written)
        .map_err(|source| ModrinthError::Write {
            path: PathBuf::new(),
            source,
        })?;
    Ok(summary)
}

async fn enabled_mods(game_dir: &Path) -> Result<Vec<(String, String)>, ModrinthError> {
    let mut found = Vec::new();
    for file in mods::list(game_dir).await? {
        if file.enabled {
            let path = mods::mods_dir(game_dir).join(&file.name);
            if let Ok(Some(hash)) = file_sha1(&path).await {
                found.push((file.name, hash));
            }
        }
    }
    Ok(found)
}

/// 來源資料齊全才寫成網址。格式要求 SHA512，少了別的啟動器會拒絕。
fn linked_file(version: &ModVersion, name: &str, hash: &str) -> Option<ExportFile> {
    let file = version.files.iter().find(|file| file.hashes.sha1 == hash)?;
    (!file.hashes.sha512.is_empty()).then(|| ExportFile {
        path: format!("mods/{name}"),
        hashes: ExportHashes {
            sha1: file.hashes.sha1.clone(),
            sha512: file.hashes.sha512.clone(),
        },
        downloads: vec![file.url.clone()],
        file_size: file.size,
    })
}

fn dependencies(instance: &Instance) -> HashMap<String, String> {
    let mut dependencies = HashMap::from([("minecraft".to_string(), instance.version.clone())]);
    if let Some((id, _)) = LOADERS.iter().find(|(_, kind)| *kind == instance.loader) {
        dependencies.insert((*id).to_string(), instance.loader_version.clone());
    }
    dependencies
}

/// 先寫到暫存檔，中途失敗不會留下壞掉的整合包。
fn write_archive(
    target: &Path,
    index: &[u8],
    game_dir: &Path,
    bundled: &[String],
) -> io::Result<()> {
    let temporary = target.with_extension("mrpack.tmp");
    let written = write_entries(&temporary, index, game_dir, bundled);
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
        return written;
    }
    std::fs::rename(&temporary, target)
}

fn write_entries(path: &Path, index: &[u8], game_dir: &Path, bundled: &[String]) -> io::Result<()> {
    let mut writer = ZipWriter::new(File::create(path)?);
    let options = SimpleFileOptions::default();
    writer
        .start_file(INDEX_FILE, options)
        .map_err(io::Error::other)?;
    io::Write::write_all(&mut writer, index)?;
    for name in bundled {
        let entry = format!("{OVERRIDES}/mods/{name}");
        add_file(&mut writer, &mods::mods_dir(game_dir).join(name), &entry)?;
    }
    for item in SHARED_ITEMS {
        add_item(
            &mut writer,
            &game_dir.join(item),
            &format!("{OVERRIDES}/{item}"),
        )?;
    }
    writer.finish().map_err(io::Error::other)?;
    Ok(())
}

fn add_item(writer: &mut ZipWriter<File>, path: &Path, entry: &str) -> io::Result<()> {
    if path.is_file() {
        return add_file(writer, path, entry);
    }
    if !path.is_dir() {
        return Ok(());
    }
    for child in std::fs::read_dir(path)? {
        let child = child?;
        let name = child.file_name().to_string_lossy().to_string();
        add_item(writer, &child.path(), &format!("{entry}/{name}"))?;
    }
    Ok(())
}

fn add_file(writer: &mut ZipWriter<File>, path: &Path, entry: &str) -> io::Result<()> {
    writer
        .start_file(entry, SimpleFileOptions::default())
        .map_err(io::Error::other)?;
    io::copy(&mut File::open(path)?, writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::pack::read_exported;
    use super::*;
    use crate::loader::LoaderKind;

    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sapling-export-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn an_exported_pack_can_be_read_back() {
        let game_dir = folder("round-trip");
        std::fs::create_dir_all(game_dir.join("mods")).unwrap();
        std::fs::create_dir_all(game_dir.join("config/sub")).unwrap();
        std::fs::create_dir_all(game_dir.join("saves/world")).unwrap();
        std::fs::write(game_dir.join("mods/handmade.jar"), "jar").unwrap();
        std::fs::write(game_dir.join("config/sub/a.toml"), "a").unwrap();
        std::fs::write(game_dir.join("options.txt"), "fov:1").unwrap();
        std::fs::write(game_dir.join("saves/world/level.dat"), "secret").unwrap();
        let instance = Instance {
            id: "x".to_string(),
            name: "我的包".to_string(),
            version: "26.3".to_string(),
            loader: LoaderKind::Fabric,
            loader_version: "0.20.1".to_string(),
            memory_mb: 0,
            java_args: String::new(),
            ..Instance::blank()
        };
        let index = ExportIndex {
            format_version: FORMAT_VERSION,
            game: GAME,
            version_id: "1.0.0".to_string(),
            name: instance.name.clone(),
            files: Vec::new(),
            dependencies: dependencies(&instance),
        };
        let target = game_dir.join("out.mrpack");

        write_archive(
            &target,
            &serde_json::to_vec(&index).unwrap(),
            &game_dir,
            &["handmade.jar".to_string()],
        )
        .unwrap();

        let (name, game, loader, entries) = read_exported(&target);
        assert_eq!(name, "我的包");
        assert_eq!(game, "26.3");
        assert_eq!(loader, LoaderKind::Fabric);
        assert!(entries.contains(&"overrides/mods/handmade.jar".to_string()));
        assert!(entries.contains(&"overrides/config/sub/a.toml".to_string()));
        assert!(entries.contains(&"overrides/options.txt".to_string()));
        assert!(!entries.iter().any(|entry| entry.contains("saves")));
    }
}
