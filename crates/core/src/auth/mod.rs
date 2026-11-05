//! 微軟登入：設備代碼流程 → Xbox Live → XSTS → Minecraft。
//! 只有 refresh token 會被保存，而且只存在 Windows 憑證管理員。

mod token_store;

use std::fmt;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

const CLIENT_ID: &str = "847cfa56-a8dc-4c4c-9106-e24b444c74a0";
const SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
const ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/// Minecraft token 到期前多久就當作已過期，避免遊戲啟動到一半失效。
const EXPIRY_MARGIN: Duration = Duration::from_secs(300);
/// 微軟要求 `slow_down` 時，輪詢間隔要增加的秒數。
const SLOW_DOWN_STEP_SECS: u64 = 5;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("無法連線到登入伺服器，請檢查網絡連線後再試一次")]
    Network(#[from] reqwest::Error),
    #[error("微軟拒絕了登入請求，應用程式設定可能有問題（{code}）")]
    ClientRejected { code: String },
    #[error("你在微軟頁面拒絕了登入")]
    Declined,
    #[error("登入代碼已過期，請重新登入")]
    CodeExpired,
    #[error("登入已過期，請重新登入")]
    SessionExpired,
    #[error("{}", xsts_error_message(*.code))]
    XboxRefused { code: u64 },
    #[error("這個應用程式還沒獲得 Mojang 批准使用 Minecraft 登入，需要先到 aka.ms/mce-reviewappid 申請")]
    AppNotApproved,
    #[error("這個微軟帳號沒有購買 Minecraft Java 版")]
    GameNotOwned,
    #[error("這個帳號還沒建立 Minecraft 玩家檔案，請先用官方啟動器登入一次")]
    NoProfile,
    #[error("登入時收到看不懂的回應（{step}，狀態 {status}）")]
    Unexpected { step: &'static str, status: u16 },
    #[error("無法存取 Windows 憑證管理員")]
    TokenStore(#[from] keyring::Error),
}

fn xsts_error_message(code: u64) -> String {
    match code {
        2148916227 => "這個帳號已被 Xbox 封禁".to_string(),
        2148916233 => "這個微軟帳號還沒有 Xbox 檔案，請先到 minecraft.net 登入一次來建立".to_string(),
        2148916235 => "你所在的地區無法使用 Xbox Live".to_string(),
        2148916236 | 2148916237 => "這個帳號需要先在 Xbox 網站完成成年驗證".to_string(),
        2148916238 => "這是未成年帳號，需要由成年人把它加入微軟家庭群組".to_string(),
        other => format!("Xbox 拒絕了登入（錯誤碼 {other}）"),
    }
}

/// 顯示給用戶的登入代碼。
#[derive(Clone, Deserialize)]
pub struct DeviceLogin {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    device_code: String,
    interval: u64,
}

/// 可以公開顯示的玩家資料。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub skin_url: Option<String>,
}

/// 已登入的狀態。不實作 `Serialize`，token 不會離開 Rust。
#[derive(Clone)]
pub struct Session {
    pub profile: Profile,
    access_token: String,
    expires_at: Instant,
}

impl Session {
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() + EXPIRY_MARGIN >= self.expires_at
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Session")
            .field("profile", &self.profile)
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
struct MicrosoftTokens {
    access_token: String,
    refresh_token: String,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
}

#[derive(Deserialize)]
struct XboxResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XboxClaims,
}

#[derive(Deserialize)]
struct XboxClaims {
    xui: Vec<XboxUser>,
}

#[derive(Deserialize)]
struct XboxUser {
    uhs: String,
}

#[derive(Deserialize)]
struct XstsError {
    #[serde(rename = "XErr")]
    code: u64,
}

#[derive(Deserialize)]
struct MinecraftLogin {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
struct Entitlements {
    items: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct MinecraftProfile {
    id: String,
    name: String,
    #[serde(default)]
    skins: Vec<MinecraftSkin>,
}

#[derive(Deserialize)]
struct MinecraftSkin {
    state: String,
    url: String,
}

/// 第一步：向微軟要一組登入代碼，讓用戶在瀏覽器輸入。
pub async fn begin_login(client: &reqwest::Client) -> Result<DeviceLogin, AuthError> {
    let response = client
        .post(DEVICE_CODE_URL)
        .form(&[("client_id", CLIENT_ID), ("scope", SCOPE)])
        .send()
        .await?;
    if response.status().is_success() {
        return Ok(response.json().await?);
    }
    let status = response.status().as_u16();
    match response.json::<OAuthError>().await {
        Ok(error) => Err(AuthError::ClientRejected { code: error.error }),
        Err(_) => Err(AuthError::Unexpected {
            step: "devicecode",
            status,
        }),
    }
}

/// 第二步：等用戶在瀏覽器完成登入，然後換到 Minecraft 登入狀態並保存。
pub async fn finish_login(
    client: &reqwest::Client,
    login: &DeviceLogin,
) -> Result<Session, AuthError> {
    let deadline = Instant::now() + Duration::from_secs(login.expires_in);
    let mut interval = login.interval.max(1);
    loop {
        tokio::time::sleep(Duration::from_secs(interval)).await;
        if Instant::now() >= deadline {
            return Err(AuthError::CodeExpired);
        }
        let response = client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", CLIENT_ID),
                ("device_code", login.device_code.as_str()),
            ])
            .send()
            .await?;
        if response.status().is_success() {
            return complete_login(client, response.json().await?).await;
        }
        let status = response.status().as_u16();
        let Ok(error) = response.json::<OAuthError>().await else {
            return Err(AuthError::Unexpected {
                step: "token",
                status,
            });
        };
        match error.error.as_str() {
            "authorization_pending" => {}
            "slow_down" => interval += SLOW_DOWN_STEP_SECS,
            "authorization_declined" => return Err(AuthError::Declined),
            "expired_token" | "bad_verification_code" => return Err(AuthError::CodeExpired),
            _ => return Err(AuthError::ClientRejected { code: error.error }),
        }
    }
}

/// 用保存的 refresh token 恢復登入。沒有保存過就回傳 `None`。
pub async fn restore_session(client: &reqwest::Client) -> Result<Option<Session>, AuthError> {
    let Some(refresh_token) = token_store::load()? else {
        return Ok(None);
    };
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("scope", SCOPE),
            ("refresh_token", refresh_token.as_str()),
        ])
        .send()
        .await?;
    if response.status().is_success() {
        return complete_login(client, response.json().await?)
            .await
            .map(Some);
    }
    let status = response.status().as_u16();
    match response.json::<OAuthError>().await {
        Ok(error) if error.error == "invalid_grant" => {
            token_store::delete()?;
            Err(AuthError::SessionExpired)
        }
        Ok(error) => Err(AuthError::ClientRejected { code: error.error }),
        Err(_) => Err(AuthError::Unexpected {
            step: "refresh",
            status,
        }),
    }
}

/// 登出：刪除保存的 refresh token。
pub fn sign_out() -> Result<(), AuthError> {
    token_store::delete()?;
    Ok(())
}

async fn complete_login(
    client: &reqwest::Client,
    microsoft: MicrosoftTokens,
) -> Result<Session, AuthError> {
    let xbox = xbox_login(client, &microsoft.access_token).await?;
    let (xsts_token, user_hash) = xsts_authorize(client, &xbox).await?;
    let minecraft = minecraft_login(client, &user_hash, &xsts_token).await?;
    let token = &minecraft.access_token;

    let entitlements: Entitlements = get_with_token(client, ENTITLEMENTS_URL, token, "entitlements").await?;
    if entitlements.items.is_empty() {
        return Err(AuthError::GameNotOwned);
    }
    let profile = fetch_profile(client, token).await?;

    token_store::save(&microsoft.refresh_token)?;
    Ok(Session {
        profile,
        expires_at: Instant::now() + Duration::from_secs(minecraft.expires_in),
        access_token: minecraft.access_token,
    })
}

async fn xbox_login(client: &reqwest::Client, access_token: &str) -> Result<String, AuthError> {
    let response = client
        .post(XBOX_AUTH_URL)
        .header("Accept", "application/json")
        .json(&json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={access_token}"),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(AuthError::Unexpected {
            step: "xbox",
            status: response.status().as_u16(),
        });
    }
    let xbox: XboxResponse = response.json().await?;
    Ok(xbox.token)
}

async fn xsts_authorize(
    client: &reqwest::Client,
    xbox_token: &str,
) -> Result<(String, String), AuthError> {
    let response = client
        .post(XSTS_AUTH_URL)
        .header("Accept", "application/json")
        .json(&json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbox_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))
        .send()
        .await?;
    let status = response.status().as_u16();
    if status == 401 {
        let Ok(error) = response.json::<XstsError>().await else {
            return Err(AuthError::Unexpected {
                step: "xsts",
                status,
            });
        };
        return Err(AuthError::XboxRefused { code: error.code });
    }
    if !response.status().is_success() {
        return Err(AuthError::Unexpected {
            step: "xsts",
            status,
        });
    }
    let xsts: XboxResponse = response.json().await?;
    let user_hash = xsts
        .display_claims
        .xui
        .into_iter()
        .next()
        .map(|user| user.uhs)
        .ok_or(AuthError::Unexpected {
            step: "xsts",
            status,
        })?;
    Ok((xsts.token, user_hash))
}

async fn minecraft_login(
    client: &reqwest::Client,
    user_hash: &str,
    xsts_token: &str,
) -> Result<MinecraftLogin, AuthError> {
    let response = client
        .post(MINECRAFT_LOGIN_URL)
        .header("Accept", "application/json")
        .json(&json!({ "identityToken": format!("XBL3.0 x={user_hash};{xsts_token}") }))
        .send()
        .await?;
    match response.status().as_u16() {
        403 => Err(AuthError::AppNotApproved),
        status if !response.status().is_success() => Err(AuthError::Unexpected {
            step: "minecraft",
            status,
        }),
        _ => Ok(response.json().await?),
    }
}

async fn fetch_profile(client: &reqwest::Client, token: &str) -> Result<Profile, AuthError> {
    let response = client.get(PROFILE_URL).bearer_auth(token).send().await?;
    match response.status().as_u16() {
        404 => Err(AuthError::NoProfile),
        status if !response.status().is_success() => Err(AuthError::Unexpected {
            step: "profile",
            status,
        }),
        _ => {
            let profile: MinecraftProfile = response.json().await?;
            let skin_url = active_skin_url(&profile.skins);
            Ok(Profile {
                id: profile.id,
                name: profile.name,
                skin_url,
            })
        }
    }
}

fn active_skin_url(skins: &[MinecraftSkin]) -> Option<String> {
    skins
        .iter()
        .find(|skin| skin.state == "ACTIVE")
        .map(|skin| skin.url.clone())
}

async fn get_with_token<T: DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    step: &'static str,
) -> Result<T, AuthError> {
    let response = client.get(url).bearer_auth(token).send().await?;
    if !response.status().is_success() {
        return Err(AuthError::Unexpected {
            step,
            status: response.status().as_u16(),
        });
    }
    Ok(response.json().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explains_common_xbox_errors_in_plain_language() {
        let message = AuthError::XboxRefused { code: 2148916233 }.to_string();
        assert!(message.contains("Xbox 檔案"));
        let message = AuthError::XboxRefused { code: 2148916238 }.to_string();
        assert!(message.contains("未成年"));
    }

    #[test]
    fn unknown_xbox_errors_still_show_the_code() {
        let message = AuthError::XboxRefused { code: 2148916262 }.to_string();
        assert!(message.contains("2148916262"));
    }

    #[test]
    fn debug_output_never_contains_the_token() {
        let session = Session {
            profile: Profile {
                id: "id".to_string(),
                name: "Steve".to_string(),
                skin_url: None,
            },
            access_token: "secret-token".to_string(),
            expires_at: Instant::now(),
        };
        assert!(!format!("{session:?}").contains("secret-token"));
    }

    #[test]
    fn session_close_to_expiry_counts_as_expired() {
        let session = Session {
            profile: Profile {
                id: String::new(),
                name: String::new(),
                skin_url: None,
            },
            access_token: String::new(),
            expires_at: Instant::now() + Duration::from_secs(60),
        };
        assert!(session.is_expired());
    }

    #[test]
    fn picks_the_active_skin() {
        let skins = [
            MinecraftSkin {
                state: "INACTIVE".to_string(),
                url: "old".to_string(),
            },
            MinecraftSkin {
                state: "ACTIVE".to_string(),
                url: "current".to_string(),
            },
        ];
        assert_eq!(active_skin_url(&skins).as_deref(), Some("current"));
    }

    #[test]
    fn reads_the_device_code_response() {
        let login: DeviceLogin = serde_json::from_str(
            r#"{
                "user_code": "ABC123",
                "device_code": "secret",
                "verification_uri": "https://www.microsoft.com/link",
                "expires_in": 900,
                "interval": 5,
                "message": "ignored"
            }"#,
        )
        .unwrap();
        assert_eq!(login.user_code, "ABC123");
        assert_eq!(login.interval, 5);
    }
}
