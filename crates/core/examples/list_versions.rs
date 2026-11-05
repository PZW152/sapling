//! 手動測試：下載版本清單並列出 v1 支援的正式版。
//! 執行：cargo run -p sapling-core --example list_versions

use sapling_core::versions::{VersionsError, fetch_manifest};

#[tokio::main]
async fn main() -> Result<(), VersionsError> {
    let manifest = fetch_manifest().await?;
    let supported = manifest.supported_releases()?;

    println!("最新正式版：{}", manifest.latest.release);
    println!("最新快照版：{}", manifest.latest.snapshot);
    println!("清單總版本數：{}", manifest.versions.len());
    println!("v1 支援的正式版：{} 個", supported.len());
    for entry in supported {
        println!("  {}", entry.id);
    }
    Ok(())
}
