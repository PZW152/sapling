//! 複製資料夾，複製實例和備份世界共用。

use std::io;
use std::path::Path;

/// 遊戲運行時鎖著的檔案，複製了也沒有意義。
pub const LOCK_FILE: &str = "session.lock";

/// 把整個資料夾複製到新位置，略過 `skip` 裡的名稱。保留檔案的修改時間。
pub fn copy_dir(from: &Path, to: &Path, skip: &[&str]) -> io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if skip.iter().any(|skipped| name == *skipped) {
            continue;
        }
        let kind = entry.file_type()?;
        let target = to.join(&name);
        if kind.is_dir() {
            copy_dir(&entry.path(), &target, skip)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// 複製模組資料夾：能用硬連結就用，不支援時改成一般複製。
pub fn link_or_copy_files(from: &Path, to: &Path) -> io::Result<()> {
    if !from.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let target = to.join(entry.file_name());
        if std::fs::hard_link(entry.path(), &target).is_err() {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_nested_files_and_skips_named_ones() {
        let root = std::env::temp_dir().join(format!("sapling-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let from = root.join("from");
        std::fs::create_dir_all(from.join("region")).unwrap();
        std::fs::write(from.join("level.dat"), "a").unwrap();
        std::fs::write(from.join("region/r.mca"), "b").unwrap();
        std::fs::write(from.join(LOCK_FILE), "c").unwrap();

        let to = root.join("to");
        copy_dir(&from, &to, &[LOCK_FILE]).unwrap();

        assert_eq!(std::fs::read(to.join("region/r.mca")).unwrap(), b"b");
        assert!(to.join("level.dat").exists());
        assert!(!to.join(LOCK_FILE).exists());
    }
}
