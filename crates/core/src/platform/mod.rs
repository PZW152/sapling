//! 所有 Windows 專屬的設定都集中在這裡，日後移植時只改這個模塊。

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Mojang 的 `rules` 裡用來代表本系統的名稱。
pub const MOJANG_OS_NAME: &str = "windows";

/// Mojang 的 `rules` 裡用來代表本機架構的名稱。
pub const MOJANG_ARCH: &str = "x64";

/// 這些 natives 依賴庫的 `rules` 只寫了 windows、沒寫架構，
/// 但它們是給其他 CPU 架構用的，Windows x64 不需要下載。
pub const OTHER_ARCH_NATIVE_SUFFIXES: [&str; 2] =
    [":natives-windows-x86", ":natives-windows-arm64"];

/// Mojang 的 Java runtime 清單裡代表本機的平台名稱。
pub const MOJANG_RUNTIME_PLATFORM: &str = "windows-x64";

/// `java.exe` 在 runtime 資料夾裡的相對位置。
pub const JAVA_EXE_PATH: &str = "bin/java.exe";

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
}
