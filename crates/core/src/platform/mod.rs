//! 所有 Windows 專屬的設定都集中在這裡，日後移植時只改這個模塊。

use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

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
    #[error("找不到系統的應用程式資料夾（%APPDATA%）")]
    NoDataFolder,
}

/// `%APPDATA%\Sapling\` 下各資料夾的位置。
#[derive(Debug, Clone)]
pub struct DataDirs {
    root: PathBuf,
}

impl DataDirs {
    /// 本機的正式資料位置。
    pub fn for_this_user() -> Result<Self, PlatformError> {
        let app_data = dirs::config_dir().ok_or(PlatformError::NoDataFolder)?;
        Ok(Self::at(app_data.join("Sapling")))
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
