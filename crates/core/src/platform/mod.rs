//! 所有 Windows 專屬的設定都集中在這裡，日後移植時只改這個模塊。

/// Mojang 的 `rules` 裡用來代表本系統的名稱。
pub const MOJANG_OS_NAME: &str = "windows";

/// Mojang 的 `rules` 裡用來代表本機架構的名稱。
pub const MOJANG_ARCH: &str = "x64";

/// 這些 natives 依賴庫的 `rules` 只寫了 windows、沒寫架構，
/// 但它們是給其他 CPU 架構用的，Windows x64 不需要下載。
pub const OTHER_ARCH_NATIVE_SUFFIXES: [&str; 2] =
    [":natives-windows-x86", ":natives-windows-arm64"];
