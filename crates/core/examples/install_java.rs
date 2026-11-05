//! 手動測試：下載最新正式版需要的 Java，並執行 `java -version`。
//! 執行：cargo run -p sapling-core --example install_java

use std::error::Error;
use std::process::Command;

use sapling_core::download::{DEFAULT_CONCURRENCY, new_client};
use sapling_core::java::install_runtime;
use sapling_core::platform::DataDirs;
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
    let component = &details.java_version.component;
    println!("{} 需要 {component}", details.id);

    let dirs = DataDirs::for_this_user()?;
    let mut files = 0;
    let java = install_runtime(&client, component, &dirs, DEFAULT_CONCURRENCY, |progress| {
        files = progress.finished_files
    })
    .await?;
    println!("{files} 個檔案全部通過 SHA1 校驗");
    println!("java.exe：{}", java.display());

    let output = Command::new(&java).arg("-version").output()?;
    print!("{}", String::from_utf8_lossy(&output.stderr));
    println!("java -version 結束代碼：{}", output.status);
    Ok(())
}
