//! 啟動器自己的運行記錄，放在資料夾的 `launcher.log`，出問題時才查得到原因。
//!
//! 記錄可能被用戶貼到網路上，所以寫入前一律遮掉 token 和用戶名。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};

use crate::platform;

const LOG_FILE: &str = "launcher.log";
const OLD_LOG_FILE: &str = "launcher.old.log";
const MAX_LOG_BYTES: u64 = 1024 * 1024;
/// token 都很長。比這短的英數串不可能是 token。
const MIN_SECRET_LENGTH: usize = 40;
/// SHA 雜湊最長 128 個字元，是下載網址的一部分，留著才查得出問題。
const MAX_HASH_LENGTH: usize = 128;
const HIDDEN: &str = "[已隱藏]";

static FOLDER: OnceLock<Mutex<PathBuf>> = OnceLock::new();

/// 開始記錄。之後的 `record` 都寫到這個資料夾。
pub fn start(folder: &Path) {
    let _ = fs::create_dir_all(folder);
    let _ = FOLDER.set(Mutex::new(folder.to_path_buf()));
}

/// 寫一行記錄。寫入失敗不影響啟動器本身。
pub fn record(message: &str) {
    let Some(folder) = FOLDER.get() else {
        return;
    };
    let folder = folder.lock().unwrap_or_else(PoisonError::into_inner);
    let line = format!("{} {}\n", platform::local_time_text(), clean(message));
    let path = folder.join(LOG_FILE);
    if fs::metadata(&path).is_ok_and(|found| found.len() > MAX_LOG_BYTES) {
        let _ = fs::rename(&path, folder.join(OLD_LOG_FILE));
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// 最後幾行記錄。
pub fn recent(count: usize) -> Vec<String> {
    let Some(folder) = FOLDER.get() else {
        return Vec::new();
    };
    let path = folder
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .join(LOG_FILE);
    let text = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(count)..]
        .iter()
        .map(|line| (*line).to_string())
        .collect()
}

fn clean(message: &str) -> String {
    let single_line = message.replace(['\r', '\n'], " ");
    let hidden = hide_secrets(&single_line);
    match dirs::home_dir() {
        Some(home) => hide_home(&hidden, &home.to_string_lossy()),
        None => hidden,
    }
}

fn hide_home(text: &str, home: &str) -> String {
    if home.is_empty() {
        text.to_string()
    } else {
        text.replace(home, "~")
    }
}

fn is_secret_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '-' | '_' | '.' | '+' | '/' | '=' | '!' | '*' | '$'
        )
}

fn looks_like_hash(word: &str) -> bool {
    word.len() <= MAX_HASH_LENGTH && word.chars().all(|character| character.is_ascii_hexdigit())
}

/// 網址和路徑也會是長字串，但它們有分隔的斜線；token 沒有這麼多段。
fn looks_like_path(word: &str) -> bool {
    word.split('/')
        .all(|part| part.len() < MIN_SECRET_LENGTH || looks_like_hash(part))
}

/// 把看起來像 token 的長字串換掉。寧可多遮，不能漏。
fn hide_secrets(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut word = String::new();
    for character in text.chars().chain(std::iter::once(' ')) {
        if is_secret_character(character) {
            word.push(character);
            continue;
        }
        if word.len() >= MIN_SECRET_LENGTH && !looks_like_hash(&word) && !looks_like_path(&word) {
            result.push_str(HIDDEN);
        } else {
            result.push_str(&word);
        }
        word.clear();
        result.push(character);
    }
    result.pop();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_tokens() {
        let jwt =
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9P";
        let text = hide_secrets(&format!("登入失敗 {jwt} 請再試"));
        assert_eq!(text, format!("登入失敗 {HIDDEN} 請再試"));

        let refresh = "M.C512_BAY.0.U.-CkdPqZ7Rz!ZzAb*cD1eF2gH3iJ4kL5mN6oP7qR8sT9$uV";
        assert_eq!(hide_secrets(refresh), HIDDEN);
    }

    #[test]
    fn keeps_normal_text_and_links() {
        let link = "下載失敗：https://resources.download.minecraft.net/ab/ab3f1c0d9e8b7a6f5e4d3c2b1a0f9e8d7c6b5a49";
        assert_eq!(hide_secrets(link), link);
        let plain = "找不到檔案 mods/sodium-fabric-0.6.13+mc1.21.8.jar";
        assert_eq!(hide_secrets(plain), plain);
    }

    #[test]
    fn hides_the_user_name_in_paths() {
        assert_eq!(
            hide_home(
                r"無法寫入 C:\Users\小明\Sapling\config.json",
                r"C:\Users\小明"
            ),
            r"無法寫入 ~\Sapling\config.json"
        );
    }

    #[test]
    fn keeps_each_record_on_one_line() {
        assert_eq!(clean("第一行\r\n第二行"), "第一行  第二行");
    }
}
