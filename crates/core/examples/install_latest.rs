//! 手動測試：把最新正式版完整下載到 %APPDATA%\Sapling\。
//! 再執行一次會重新校驗所有檔案的 SHA1，並跳過下載。
//! 執行：cargo run -p sapling-core --example install_latest

use std::error::Error;
use std::time::Instant;

use sapling_core::download::{DEFAULT_CONCURRENCY, new_client};
use sapling_core::platform::DataDirs;
use sapling_core::versions::fetch_manifest;
use sapling_core::versions::install::install_version;

const BYTES_PER_MB: f64 = 1024.0 * 1024.0;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manifest = fetch_manifest().await?;
    let latest = manifest
        .versions
        .iter()
        .find(|entry| entry.id == manifest.latest.release)
        .ok_or("版本清單裡找不到最新正式版")?;
    let dirs = DataDirs::for_this_user()?;
    println!("安裝 {} 到 {}", latest.id, dirs.root().display());

    let started = Instant::now();
    let mut last_percent = 0;
    let mut finished = (0, 0.0);
    install_version(
        &new_client()?,
        latest,
        &dirs,
        DEFAULT_CONCURRENCY,
        |progress| {
            let percent = progress.finished_bytes * 100 / progress.total_bytes.max(1);
            finished = (
                progress.finished_files,
                progress.finished_bytes as f64 / BYTES_PER_MB,
            );
            if percent >= last_percent + 10 {
                last_percent = percent;
                println!(
                    "{percent:>3}%  {}/{} 個檔案",
                    progress.finished_files, progress.total_files
                );
            }
        },
    )
    .await?;

    println!(
        "完成：{} 個檔案，{:.0} MB，全部通過 SHA1 校驗，耗時 {:.1} 秒",
        finished.0,
        finished.1,
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
