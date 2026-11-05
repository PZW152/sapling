//! Tauri 命令：只做轉接，業務邏輯都在 sapling-core。

use std::sync::Mutex;
use std::time::{Duration, Instant};

use sapling_core::auth::{self, DeviceLogin, Profile, Session};
use sapling_core::config::LauncherConfig;
use sapling_core::download::new_client;
use sapling_core::instance::{DEFAULT_INSTANCE_ID, Instance};
use sapling_core::launch::{LaunchEvent, play};
use sapling_core::platform::{DataDirs, open_in_browser};
use sapling_core::versions::fetch_manifest;
use serde::{Deserialize, Serialize};
use tauri::async_runtime::Mutex as AsyncMutex;
use tauri::{AppHandle, Emitter, State};

const LAUNCH_EVENT: &str = "launch-event";
const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Default)]
struct AppState {
    session: Mutex<Option<Session>>,
    device_login: Mutex<Option<DeviceLogin>>,
    /// 同一時間只做一次恢復登入，避免短時間重複請求被 Minecraft 伺服器限流。
    restoring: AsyncMutex<()>,
}

impl AppState {
    fn set_session(&self, session: Option<Session>) -> Result<(), String> {
        *self.session.lock().map_err(|_| STATE_ERROR)? = session;
        Ok(())
    }

    fn session(&self) -> Result<Option<Session>, String> {
        Ok(self.session.lock().map_err(|_| STATE_ERROR)?.clone())
    }
}

const STATE_ERROR: &str = "啟動器內部狀態出錯，請重開啟動器";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionList {
    latest: String,
    versions: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    instance: Instance,
    config: LauncherConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginCode {
    user_code: String,
    verification_uri: String,
}

fn text<E: ToString>(error: E) -> String {
    error.to_string()
}

#[tauri::command]
async fn list_versions() -> Result<VersionList, String> {
    let manifest = fetch_manifest().await.map_err(text)?;
    let versions = manifest
        .supported_releases()
        .map_err(text)?
        .iter()
        .map(|entry| entry.id.clone())
        .collect();
    Ok(VersionList {
        latest: manifest.latest.release.clone(),
        versions,
    })
}

#[tauri::command]
async fn load_settings() -> Result<Settings, String> {
    let dirs = DataDirs::for_this_user().map_err(text)?;
    Ok(Settings {
        instance: Instance::load_or_new(&dirs, DEFAULT_INSTANCE_ID)
            .await
            .map_err(text)?,
        config: LauncherConfig::load(&dirs).await.map_err(text)?,
    })
}

#[tauri::command]
async fn save_settings(settings: Settings) -> Result<(), String> {
    let dirs = DataDirs::for_this_user().map_err(text)?;
    settings.instance.save(&dirs).await.map_err(text)?;
    settings.config.save(&dirs).await.map_err(text)
}

#[tauri::command]
async fn restore_account(state: State<'_, AppState>) -> Result<Option<Profile>, String> {
    Ok(current_session(&state).await?.map(|session| session.profile))
}

#[tauri::command]
async fn login_start(state: State<'_, AppState>) -> Result<LoginCode, String> {
    let client = new_client().map_err(text)?;
    let login = auth::begin_login(&client).await.map_err(text)?;
    // 打不開瀏覽器時，介面仍然顯示網址和代碼，用戶可以自己打開。
    let _ = open_in_browser(&login.verification_uri);
    let code = LoginCode {
        user_code: login.user_code.clone(),
        verification_uri: login.verification_uri.clone(),
    };
    *state.device_login.lock().map_err(|_| STATE_ERROR)? = Some(login);
    Ok(code)
}

#[tauri::command]
async fn login_finish(state: State<'_, AppState>) -> Result<Profile, String> {
    let login = state
        .device_login
        .lock()
        .map_err(|_| STATE_ERROR)?
        .take()
        .ok_or("請重新開始登入")?;
    let client = new_client().map_err(text)?;
    let session = auth::finish_login(&client, &login).await.map_err(text)?;
    let profile = session.profile.clone();
    state.set_session(Some(session))?;
    Ok(profile)
}

#[tauri::command]
fn sign_out(state: State<'_, AppState>) -> Result<(), String> {
    auth::sign_out().map_err(text)?;
    state.set_session(None)
}

#[tauri::command]
async fn play_game(app: AppHandle, state: State<'_, AppState>, version: String) -> Result<(), String> {
    let session = fresh_session(&state).await?;
    let dirs = DataDirs::for_this_user().map_err(text)?;
    let config = LauncherConfig::load(&dirs).await.map_err(text)?;
    let mut instance = Instance::load_or_new(&dirs, DEFAULT_INSTANCE_ID)
        .await
        .map_err(text)?;
    instance.version = version;
    instance.save(&dirs).await.map_err(text)?;

    let mut last_progress = Instant::now();
    play(&session, &instance, &dirs, config.download_concurrency, move |event| {
        if let LaunchEvent::Progress { progress, .. } = &event {
            let is_last = progress.finished_files == progress.total_files;
            if !is_last && last_progress.elapsed() < PROGRESS_INTERVAL {
                return;
            }
            last_progress = Instant::now();
        }
        // 事件只用於顯示，送不出去不影響遊戲本身。
        let _ = app.emit(LAUNCH_EVENT, event);
    })
    .await
    .map_err(text)
}

/// 取得可用的登入狀態，過期就自動刷新。
async fn fresh_session(state: &State<'_, AppState>) -> Result<Session, String> {
    current_session(state)
        .await?
        .ok_or_else(|| "請先登入微軟帳號".to_string())
}

/// 目前有效的登入狀態；沒有或已過期時用保存的 refresh token 恢復。
async fn current_session(state: &State<'_, AppState>) -> Result<Option<Session>, String> {
    let _restoring = state.restoring.lock().await;
    if let Some(session) = state.session()?.filter(|session| !session.is_expired()) {
        return Ok(Some(session));
    }
    let client = new_client().map_err(text)?;
    let session = auth::restore_session(&client).await.map_err(text)?;
    state.set_session(session.clone())?;
    Ok(session)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_versions,
            load_settings,
            save_settings,
            restore_account,
            login_start,
            login_finish,
            sign_out,
            play_game
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
