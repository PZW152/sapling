//! 崩潰分析：從遊戲日誌和崩潰報告找出常見原因，翻譯成白話。

use std::path::{Path, PathBuf};

const LOG_TAIL_BYTES: usize = 200_000;
const UNKNOWN_CAUSE: &str = "沒有找到明確原因，可以查看問題記錄，或把崩潰報告發給別人幫忙。";

struct Rule {
    markers: &'static [&'static str],
    advice: &'static str,
}

/// 由最具體到最籠統，只回報第一個符合的原因。
const RULES: [Rule; 13] = [
    Rule {
        markers: &["OutOfMemoryError", "Could not reserve enough space"],
        advice: "遊戲記憶體不足。請到實例頁的「記憶體」分頁調高，或關閉其他佔用記憶體的程式。",
    },
    Rule {
        markers: &[
            "Could not create the Java Virtual Machine",
            "Unrecognized VM option",
        ],
        advice: "Java 不接受目前的參數。請到實例的設定分頁清空額外 Java 參數，或調低記憶體。",
    },
    Rule {
        markers: &[
            "There is not enough space on the disk",
            "No space left on device",
            "磁碟空間不足",
        ],
        advice: "硬碟空間不足。請清出一些空間，或到進階設定的清理空間刪除用不到的檔案。",
    },
    Rule {
        markers: &[
            "nvoglv64.dll",
            "atio6axx.dll",
            "ig9icd64.dll",
            "ig11icd64.dll",
            "ig12icd64.dll",
        ],
        advice: "顯示卡驅動崩潰了。請更新顯示卡驅動，用光影時也可以先停用試試。",
    },
    Rule {
        markers: &[
            "UnsupportedClassVersionError",
            "Unsupported class file major version",
        ],
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
        markers: &[
            "zip END header not found",
            "ZipException",
            "InvalidModFileException",
        ],
        advice: "有檔案損壞了。請重新下載最近加入的模組，或到實例的管理分頁修復遊戲檔案。",
    },
    Rule {
        markers: &[
            "NoSuchMethodError",
            "NoSuchFieldError",
            "NoClassDefFoundError",
        ],
        advice: "有模組和這個遊戲版本不相容。請更新模組，或逐個停用最近加入的模組。",
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

/// 崩潰報告裡點名模組的幾種寫法。分隔符號之前就是模組名稱。
const SUSPECT_MARKERS: [(&str, &str); 4] = [
    ("Suspected Mods: ", " ("),
    ("Suspected Mod: ", " ("),
    ("Mixin apply for mod ", " failed"),
    ("from mod ", "]"),
];

/// 完整的白話說明：原因加上可能有問題的模組。
pub fn explain(text: &str) -> String {
    let advice = analyze(text);
    match suspect_mod(text) {
        Some(name) => format!("{advice}可能有問題的模組：{name}。"),
        None => advice.to_string(),
    }
}

/// 找出報告點名的模組。
fn suspect_mod(text: &str) -> Option<String> {
    SUSPECT_MARKERS.iter().find_map(|(start, end)| {
        let rest = text.split_once(start)?.1;
        let line = rest.lines().next()?;
        let name = line.split_once(end).map_or(line, |(name, _)| name).trim();
        (!name.is_empty() && name != "NONE" && name.len() < 80).then(|| name.to_string())
    })
}

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
    if let Some(report) = newest_file(game_dir, "hs_err_pid").await {
        text.push_str(&read_tail(&report).await);
    }
    text.push_str(&read_tail(&game_dir.join("logs").join("latest.log")).await);
    text
}

/// Java 自己當掉時，報告放在遊戲資料夾最外層。
async fn newest_file(folder: &Path, prefix: &str) -> Option<PathBuf> {
    let mut entries = tokio::fs::read_dir(folder).await.ok()?;
    let mut newest = None;
    while let Ok(Some(entry)) = entries.next_entry().await {
        if !entry.file_name().to_string_lossy().starts_with(prefix) {
            continue;
        }
        let modified = entry.metadata().await.ok()?.modified().ok()?;
        if newest.as_ref().is_none_or(|(time, _)| modified > *time) {
            newest = Some((modified, entry.path()));
        }
    }
    newest.map(|(_, path)| path)
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
    fn names_the_mod_the_report_points_at() {
        let forge = "Suspected Mods: Create (create), Version: 6.0\nStacktrace:";
        assert!(explain(forge).ends_with("可能有問題的模組：Create。"));

        let mixin = "Mixin apply for mod sodium failed sodium.mixins.json:x";
        assert!(explain(mixin).contains("模組：sodium。"));

        let none = "Suspected Mods: NONE\n";
        assert!(!explain(none).contains("模組："));
    }

    #[test]
    fn recognises_more_common_causes() {
        assert!(analyze("# Problematic frame: nvoglv64.dll+0x1").contains("顯示卡"));
        assert!(analyze("java.util.zip.ZipException: zip END header not found").contains("損壞"));
        assert!(analyze("Unrecognized VM option 'Foo'").contains("Java 參數"));
        assert!(analyze("java.lang.NoSuchMethodError: x").contains("不相容"));
    }

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
