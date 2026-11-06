//! 舊版遊戲的原生庫包在 jar 裡，啟動前要解開到資料夾。1.19 以後不需要。

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::platform::DataDirs;
use crate::versions::details::VersionDetails;

/// 把這個版本需要的原生庫解到 `natives_dir`。已經解過的檔案不重複處理。
pub async fn extract_all(
    details: &VersionDetails,
    dirs: &DataDirs,
    natives_dir: &Path,
) -> io::Result<()> {
    let jars: Vec<(PathBuf, Vec<String>)> = details
        .libraries_for_this_machine()
        .into_iter()
        .filter_map(|library| {
            let artifact = library.native_artifact()?;
            let excluded = library
                .extract
                .as_ref()
                .map(|extract| extract.exclude.clone())
                .unwrap_or_default();
            Some((dirs.libraries().join(&artifact.path), excluded))
        })
        .collect();
    let target = natives_dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        for (jar, excluded) in jars {
            extract_jar(&jar, &target, &excluded)?;
        }
        Ok(())
    })
    .await
    .map_err(io::Error::other)?
}

fn extract_jar(jar: &Path, target: &Path, excluded: &[String]) -> io::Result<()> {
    let mut archive = ZipArchive::new(File::open(jar)?).map_err(io::Error::other)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(io::Error::other)?;
        let name = entry.name().map_err(io::Error::other)?.to_string();
        let skipped = excluded.iter().any(|prefix| name.starts_with(prefix));
        if skipped || entry.is_dir() || name.contains("..") {
            continue;
        }
        let path = target.join(&name);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        io::copy(&mut entry, &mut File::create(path)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    #[test]
    fn extracts_libraries_and_skips_excluded_folders() {
        let root = std::env::temp_dir().join(format!("sapling-natives-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let jar = root.join("natives.jar");
        let mut writer = ZipWriter::new(File::create(&jar).unwrap());
        for (name, content) in [("lwjgl64.dll", "dll"), ("META-INF/MANIFEST.MF", "m")] {
            writer
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap();

        let target = root.join("out");
        extract_jar(&jar, &target, &["META-INF/".to_string()]).unwrap();

        assert_eq!(std::fs::read(target.join("lwjgl64.dll")).unwrap(), b"dll");
        assert!(!target.join("META-INF").exists());
    }
}
