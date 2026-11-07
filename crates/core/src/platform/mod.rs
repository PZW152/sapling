//! 所有 Windows 專屬的設定都集中在這裡，日後移植時只改這個模塊。

use std::io::{self, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

const CRASH_LOG: &str = "crash.log";
const SHORTCUT_SCRIPT: &str = "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:SAPLING_LINK); $s.TargetPath = $env:SAPLING_TARGET; $s.Arguments = $env:SAPLING_ARGS; $s.IconLocation = $env:SAPLING_TARGET; $s.Save()";

/// Mojang 的 `rules` 裡用來代表本系統的名稱。
pub const MOJANG_OS_NAME: &str = "windows";

/// Mojang 的 `rules` 裡用來代表本機架構的名稱。
pub const MOJANG_ARCH: &str = "x64";

/// Windows 10 和 11 的 Java `os.version` 都是這個值，`rules` 的版本條件用它比對。
pub const MOJANG_OS_VERSION: &str = "10.0";

/// 這些 natives 依賴庫的 `rules` 只寫了 windows、沒寫架構，
/// 但它們是給其他 CPU 架構用的，Windows x64 不需要下載。
pub const OTHER_ARCH_NATIVE_SUFFIXES: [&str; 2] =
    [":natives-windows-x86", ":natives-windows-arm64"];

/// Mojang 的 Java runtime 清單裡代表本機的平台名稱。
pub const MOJANG_RUNTIME_PLATFORM: &str = "windows-x64";

/// `java.exe` 在 runtime 資料夾裡的相對位置。
pub const JAVA_EXE_PATH: &str = "bin/java.exe";

/// 讓 `java.exe` 不彈出黑色命令行窗口的 Windows 旗標。
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 便攜版把資料放在程式旁邊的這個資料夾。
const PORTABLE_DATA_FOLDER: &str = "Sapling Data";
/// 讀不到電腦記憶體時，按這個大小估算。
const FALLBACK_MEMORY_MB: u32 = 8192;
const BYTES_PER_MB: u64 = 1024 * 1024;

/// 電腦的實體記憶體大小。
pub fn total_memory_mb() -> u32 {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: status 是有效的結構，且 dwLength 已按文檔設定。
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return FALLBACK_MEMORY_MB;
    }
    u32::try_from(status.ullTotalPhys / BYTES_PER_MB).unwrap_or(u32::MAX)
}

/// 用檔案總管打開資料夾。
/// 程式崩潰時把原因留在資料資料夾。發佈版一崩潰就直接結束，沒有記錄就查不到原因。
pub fn log_panics(folder: PathBuf) {
    std::panic::set_hook(Box::new(move |info| {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let text = format!("[{time}] Sapling {}: {info}\n", env!("CARGO_PKG_VERSION"));
        let _ = std::fs::create_dir_all(&folder);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(folder.join(CRASH_LOG))
        {
            let _ = file.write_all(text.as_bytes());
        }
    }));
}

/// Windows 的版本文字，例如 `Microsoft Windows [Version 10.0.26300.1]`。
pub fn os_version() -> String {
    Command::new("cmd")
        .args(["/c", "ver"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default()
}

/// 在桌面建立捷徑，打開後用 `arguments` 啟動這個程式。
///
/// 值都用環境變數傳進去，實例名稱裡的引號不會變成指令。
pub fn create_desktop_shortcut(name: &str, arguments: &str) -> io::Result<()> {
    let desktop = dirs::desktop_dir().ok_or_else(|| io::Error::other("找不到桌面"))?;
    let safe: String = name
        .chars()
        .map(|character| {
            if r#"\/:*?"<>|"#.contains(character) {
                '_'
            } else {
                character
            }
        })
        .collect();
    let exe = std::env::current_exe()?;
    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", SHORTCUT_SCRIPT])
        .env("SAPLING_LINK", desktop.join(format!("{safe}.lnk")))
        .env("SAPLING_TARGET", &exe)
        .env("SAPLING_ARGS", arguments)
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("建立捷徑失敗"))
    }
}

pub fn open_folder(path: &Path) -> io::Result<()> {
    Command::new("explorer").arg(path).spawn().map(|_| ())
}

/// 用系統預設瀏覽器打開網址。只接受 https，避免把其他內容交給系統執行。
pub fn open_in_browser(url: &str) -> io::Result<()> {
    if !url.starts_with("https://") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "只能打開 https 網址",
        ));
    }
    Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("找不到資料存放位置")]
    NoDataFolder,
}

/// 官方啟動器的遊戲資料夾。
pub fn official_game_dir() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join(".minecraft"))
}

/// 0.3.0 以前的 Sapling 把資料放在這裡，也是開發版現在的位置。
pub fn legacy_data_dir() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("Sapling"))
}

/// 資料資料夾下各資料夾的位置。
#[derive(Debug, Clone)]
pub struct DataDirs {
    root: PathBuf,
}

impl DataDirs {
    /// 正式版把資料放在程式旁邊；開發版放在 `%APPDATA%`，避免資料混進編譯資料夾。
    pub fn locate() -> Result<Self, PlatformError> {
        if cfg!(debug_assertions) {
            let app_data = dirs::config_dir().ok_or(PlatformError::NoDataFolder)?;
            return Ok(Self::at(app_data.join("Sapling")));
        }
        let exe = std::env::current_exe().map_err(|_| PlatformError::NoDataFolder)?;
        let folder = exe.parent().ok_or(PlatformError::NoDataFolder)?;
        Ok(Self::at(folder.join(PORTABLE_DATA_FOLDER)))
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn libraries(&self) -> PathBuf {
        self.root.join("libraries")
    }

    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn versions(&self) -> PathBuf {
        self.root.join("versions")
    }

    pub fn runtimes(&self) -> PathBuf {
        self.root.join("runtimes")
    }

    pub fn instances(&self) -> PathBuf {
        self.root.join("instances")
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.json")
    }
}
