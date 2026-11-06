//! 更換皮膚。格式見 minecraft.wiki 的 Mojang API 頁面。

use std::path::Path;

use reqwest::multipart::{Form, Part};

use super::{AuthError, MinecraftProfile, Profile, Session, active_skin_url};

const SKIN_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
const SKIN_WIDTH: u32 = 64;
/// 1.8 以前的皮膚只有上半部。
const SKIN_HEIGHTS: [u32; 2] = [64, 32];

/// 上傳皮膚，回傳更新後的玩家資料。`slim` 是細手臂的 Alex 模型。
pub async fn upload(
    client: &reqwest::Client,
    session: &Session,
    path: &Path,
    slim: bool,
) -> Result<Profile, AuthError> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|_| AuthError::BadSkin)?;
    if !is_skin(&bytes) {
        return Err(AuthError::BadSkin);
    }
    let file = Part::bytes(bytes)
        .file_name("skin.png")
        .mime_str("image/png")?;
    let form = Form::new()
        .text("variant", if slim { "slim" } else { "classic" })
        .part("file", file);
    let response = client
        .post(SKIN_URL)
        .bearer_auth(session.access_token())
        .multipart(form)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(AuthError::Unexpected {
            step: "skin",
            status: status.as_u16(),
        });
    }
    let profile: MinecraftProfile = response.json().await?;
    Ok(Profile {
        skin_url: active_skin_url(&profile.skins),
        ..session.profile.clone()
    })
}

/// 看 PNG 開頭記錄的大小，不符合時伺服器也會拒絕，先給白話提示。
fn is_skin(bytes: &[u8]) -> bool {
    let number = |at: usize| {
        let part: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
        Some(u32::from_be_bytes(part))
    };
    bytes.starts_with(&PNG_SIGNATURE)
        && number(16) == Some(SKIN_WIDTH)
        && number(20).is_some_and(|height| SKIN_HEIGHTS.contains(&height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend([0, 0, 0, 13]);
        bytes.extend(b"IHDR");
        bytes.extend(width.to_be_bytes());
        bytes.extend(height.to_be_bytes());
        bytes
    }

    #[test]
    fn accepts_modern_and_classic_skin_sizes() {
        assert!(is_skin(&png(64, 64)));
        assert!(is_skin(&png(64, 32)));
    }

    #[test]
    fn refuses_other_images() {
        assert!(!is_skin(&png(128, 128)));
        assert!(!is_skin(b"not a png"));
        assert!(!is_skin(&[]));
    }
}
