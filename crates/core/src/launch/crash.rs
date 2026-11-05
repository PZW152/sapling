//! 崩潰分析：從遊戲日誌和崩潰報告找出常見原因，翻譯成白話。

use std::path::{Path, PathBuf};

const LOG_TAIL_BYTES: usize = 200_000;
const UNKNOWN_CAUSE: &str = "沒有找到明確原因，可以查看問題記錄，或把崩潰報告發給別人幫忙。";

struct Rule {
    markers: &'static [&'static str],
    advice: &'static str,
}

/// 由最具體到最籠統，只回報第一個符合的原因。
const RULES: [Rule; 8] = [
    Rule {
        markers: &["OutOfMemoryError", "Could not reserve enough space"],
        advice: "遊戲記憶體不足。請到設定頁調高記憶體，或關閉其他佔用記憶體的程式。",
    },
    Rule {
        markers: &["UnsupportedClassVersionError"],
        advice: "Java 版本和遊戲或模組不相容。請移除最近加入的模組，或換一個遊戲版本。",
    },
    Rule {
        markers: &[
            "Pixel format not accelerated",
            "GLFW error 65542",
            "WGL: The driver does not appear to support OpenGL",
        ],
        advice: "顯示卡驅動不支援遊戲。請更新顯示卡驅動後再試一次。",
    },
    Rule {
        markers: &[
            "DuplicateModsFoundException",
            "Found duplicate mod",
            "Duplicate mods",
        ],
        advice: "有重複的模組。請到實例頁刪除重複的檔案。",
    },
    Rule {
        markers: &[
            "Missing or unsupported mandatory dependencies",
            "ModResolutionException",
            "requires any version of",
        ],
        advice: "有模組缺少前置模組或版本不相容。請檢查最近加入的模組。",
    },
    Rule {
        markers: &[
            "MixinApplyError",
            "Mixin apply failed",
            "InvalidMixinException",
        ],
        advice: "有模組互相衝突。請逐個停用最近加入的模組來找出問題。",
    },
    Rule {
        markers: &["Failed to load native library", "UnsatisfiedLinkError"],
        advice: "遊戲檔案可能不完整。請重新啟動一次，Sapling 會自動修復缺少的檔案。",
    },
    Rule {
        markers: &["net.fabricmc", "net.minecraftforge", "net.neoforged"],
        advice: "崩潰可能和模組有關。請逐個停用最近加入的模組來測試。",
    },
];

/// 分析日誌內容，回傳白話建議。
pub fn analyze(text: &str) -> &'static str {
    RULES
        .iter()
        .find(|rule| rule.markers.iter().any(|marker| text.contains(marker)))
        .map_or(UNKNOWN_CAUSE, |rule| rule.advice)
}

/// 讀取最新的崩潰報告和日誌結尾。讀不到的檔案直接略過。
pub async fn read_reports(game_dir: &Path) -> String {
    let mut text = String::new();
    if let Some(report) = newest_crash_report(game_dir).await {
        text.push_str(&read_tail(&report).await);
    }
    text.push_str(&read_tail(&game_dir.join("logs").join("latest.log")).await);
    text
}

async fn newest_crash_report(game_dir: &Path) -> Option<PathBuf> {
    let mut entries = tokio::fs::read_dir(game_dir.join("crash-reports"))
        .await
        .ok()?;
    let mut newest = None;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let modified = entry.metadata().await.ok()?.modified().ok()?;
        if newest.as_ref().is_none_or(|(time, _)| modified > *time) {
            newest = Some((modified, entry.path()));
        }
    }
    newest.map(|(_, path)| path)
}

async fn read_tail(path: &Path) -> String {
    let Ok(bytes) = tokio::fs::read(path).await else {
        return String::new();
    };
    let start = bytes.len().saturating_sub(LOG_TAIL_BYTES);
    String::from_utf8_lossy(&bytes[start..]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_common_crashes() {
        assert!(analyze("java.lang.OutOfMemoryError: Java heap space").contains("記憶體"));
        assert!(analyze("GLFW error 65542: WGL").contains("顯示卡"));
        assert!(analyze("Found duplicate mod sodium").contains("重複"));
    }

    #[test]
    fn specific_causes_win_over_general_mod_hints() {
        let text = "at net.fabricmc.loader\nOutOfMemoryError";
        assert!(analyze(text).contains("記憶體"));
    }

    #[test]
    fn unknown_crashes_still_give_a_next_step() {
        assert_eq!(analyze("something else"), UNKNOWN_CAUSE);
    }

    #[tokio::test]
    async fn reads_the_crash_report() {
        let dir = std::env::temp_dir().join(format!("sapling-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("crash-reports")).unwrap();
        std::fs::write(dir.join("crash-reports").join("a.txt"), "OutOfMemoryError").unwrap();

        assert!(analyze(&read_reports(&dir).await).contains("記憶體"));
    }
}
