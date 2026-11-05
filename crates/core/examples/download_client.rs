//! 手動測試：下載最新正式版的 client.jar 並校驗 SHA1。
//! 執行兩次：第一次會下載，第二次會顯示已存在並跳過。
//! 執行：cargo run -p sapling-core --example download_client

use std::error::Error;
use std::path::Path;
use std::time::Instant;

use sapling_core::download::{DownloadOutcome, DownloadTask, download_file, new_client};
use sapling_core::versions::{fetch_details, fetch_manifest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = new_client()?;
    let manifest = fetch_manifest(&client).await?;
    let latest = manifest
        .versions
        .iter()
        .find(|entry| entry.id == manifest.latest.release)
        .ok_or("版本清單裡找不到最新正式版")?;
    let details = fetch_details(&client, latest).await?;
    let client_jar = &details.downloads.client;

    let task = DownloadTask {
        url: client_jar.url.clone(),
        path: Path::new("target/example-downloads")
            .join(&details.id)
            .join("client.jar"),
        sha1: client_jar.sha1.clone(),
        size: client_jar.size,
    };

    let started = Instant::now();
    let outcome = download_file(&client, &task).await?;
    let result = match outcome {
        DownloadOutcome::AlreadyValid => "檔案已存在且 SHA1 正確，跳過下載",
        DownloadOutcome::Downloaded => "下載完成，SHA1 校驗通過",
    };

    println!("版本：{}", details.id);
    println!("結果：{result}");
    println!("位置：{}", task.path.display());
    println!("大小：{} bytes", std::fs::metadata(&task.path)?.len());
    println!("耗時：{:.1} 秒", started.elapsed().as_secs_f64());
    Ok(())
}
