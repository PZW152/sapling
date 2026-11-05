//! 手動測試：解析所有支援版本的版本 JSON，列出 Windows x64 需要的內容。
//! 執行：cargo run -p sapling-core --example inspect_versions

use sapling_core::download::new_client;
use sapling_core::versions::{VersionsError, fetch_details, fetch_manifest};

#[tokio::main]
async fn main() -> Result<(), VersionsError> {
    let client = new_client()?;
    let manifest = fetch_manifest(&client).await?;
    let supported = manifest.supported_releases()?;

    for entry in &supported {
        let details = fetch_details(&client, entry).await?;
        let needed = details.libraries_for_this_machine();
        let natives: Vec<&str> = needed
            .iter()
            .map(|library| library.name.as_str())
            .filter(|name| name.contains(":natives-"))
            .collect();
        println!(
            "{:<8} {:<22} Java {:<3} 依賴庫 {:>3}/{:<3} natives {:>2}",
            details.id,
            details.java_version.component,
            details.java_version.major_version,
            needed.len(),
            details.libraries.len(),
            natives.len(),
        );
        if details.id == manifest.latest.release {
            for name in natives {
                println!("    {name}");
            }
        }
    }
    println!("全部 {} 個版本解析成功", supported.len());
    Ok(())
}
