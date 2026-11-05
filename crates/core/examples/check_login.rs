//! 手動測試：用保存的登入狀態恢復登入，只顯示玩家名或錯誤，不顯示 token。
//! 執行：cargo run -p sapling-core --example check_login

use std::error::Error;

use sapling_core::auth::restore_session;
use sapling_core::download::new_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    match restore_session(&new_client()?).await {
        Ok(Some(session)) => println!("恢復成功：{}", session.profile.name),
        Ok(None) => println!("沒有保存的登入狀態"),
        Err(error) => println!("恢復失敗：{error}（{error:?}）"),
    }
    Ok(())
}
