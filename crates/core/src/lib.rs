//! Sapling 核心邏輯：不依賴 tauri 或任何 UI 代碼。

pub mod auth;
pub mod cleanup;
pub mod config;
pub mod download;
pub mod instance;
pub mod java;
pub mod launch;
pub mod loader;
pub mod modrinth;
pub mod platform;
mod storage;
pub mod update;
pub mod versions;
