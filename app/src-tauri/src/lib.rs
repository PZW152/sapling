//! Tauri 命令：只做轉接，業務邏輯都在 sapling-core。

use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sapling_core::auth::{self, DeviceLogin, Profile, Session, accounts};
use sapling_core::cleanup::{self, CleanupSummary};
use sapling_core::config::LauncherConfig;
use sapling_core::download::new_client;
use sapling_core::instance::Instance;
use sapling_core::instance::import::{self, ImportSource};
use sapling_core::instance::mods::{self, ModFile};
use sapling_core::instance::worlds::{self, World};
use sapling_core::launch::{LaunchCancel, LaunchEvent, PlayOptions, play};
use sapling_core::loader::{self, LoaderKind};
use sapling_core::modrinth::pack;
use sapling_core::modrinth::{self, ModHit};
use sapling_core::platform::{DataDirs, open_folder, open_in_browser, total_memory_mb};
use sapling_core::update::{self, UpdateInfo};
use sapling_core::versions::fetch_manifest;
use sapling_core::versions::install::{PrepareStage, is_installed, prepare_version};
use serde::{Deserialize, Serialize};
use tauri::async_runtime::Mutex as AsyncMutex;
use tauri::{AppHandle, Emitter, Manager, State};

const LAUNCH_EVENT: &str = "launch-event";
/// 與前端功能模塊的 id 相同。
const CRASH_FEATURE: &str = "crash";
const OFFLINE_FEATURE: &str = "offline-account";
const INSTALL_EVENT: &str = "install-event";
const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);
/// 首頁最多顯示幾個最近遊玩的世界。
const RECENT_WORLD_LIMIT: usize = 3;
/// 實例頁最多列出幾個世界。
const WORLD_LIST_LIMIT: usize = 50;
const STATE_ERROR: &str = "啟動器內部狀態出錯，請重開啟動器";

#[derive(Default)]
struct AppState {
    session: Mutex<Option<Session>>,
    device_login: Mutex<Option<DeviceLogin>>,
    /// 同一時間只做一次恢復登入，避免短時間重複請求被 Minecraft 伺服器限流。
    restoring: AsyncMutex<()>,
    launch_cancel: LaunchCancel,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionList {
    latest: String,
    versions: Vec<String>,
    installed: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginCode {
    user_code: String,
    verification_uri: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModAction {
    instance_id: String,
    name: String,
    enabled: bool,
}

fn text<E: ToString>(error: E) -> String {
    error.to_string()
}

fn data_dirs() -> Result<DataDirs, String> {
    DataDirs::locate().map_err(text)
}

async fn load_instance(dirs: &DataDirs, id: &str) -> Result<Instance, String> {
    Instance::load(dirs, id).await.map_err(text)
}

#[tauri::command]
async fn list_versions() -> Result<VersionList, String> {
    let client = new_client().map_err(text)?;
    let manifest = fetch_manifest(&client).await.map_err(text)?;
    let dirs = data_dirs()?;
    let versions: Vec<String> = manifest
        .supported_releases()
        .map_err(text)?
        .iter()
        .map(|entry| entry.id.clone())
        .collect();
    let installed = versions
        .iter()
        .filter(|id| is_installed(&dirs, id))
        .cloned()
        .collect();
    Ok(VersionList {
        latest: manifest.latest.release.clone(),
        versions,
        installed,
    })
}

#[tauri::command]
fn system_memory_mb() -> u32 {
    total_memory_mb()
}

#[tauri::command]
async fn load_config() -> Result<LauncherConfig, String> {
    LauncherConfig::load(&data_dirs()?).await.map_err(text)
}

#[tauri::command]
async fn save_config(config: LauncherConfig) -> Result<(), String> {
    config.save(&data_dirs()?).await.map_err(text)
}

#[tauri::command]
async fn list_instances() -> Result<Vec<Instance>, String> {
    Instance::list(&data_dirs()?).await.map_err(text)
}

#[tauri::command]
async fn create_instance(
    name: String,
    version: String,
    loader: LoaderKind,
) -> Result<Instance, String> {
    let loader_version = loader::latest_version(loader, &version)
        .await
        .map_err(text)?;
    Instance::create_with_loader(&data_dirs()?, &name, &version, loader, loader_version)
        .await
        .map_err(text)
}

#[tauri::command]
async fn save_instance(instance: Instance) -> Result<(), String> {
    instance.save(&data_dirs()?).await.map_err(text)
}

#[tauri::command]
async fn delete_instance(id: String) -> Result<(), String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &id).await?;
    instance.delete(&dirs).await.map_err(text)
}

#[tauri::command]
async fn open_instance_folder(id: String) -> Result<(), String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &id).await?;
    open_folder(&instance.game_dir(&dirs)).map_err(text)
}

#[tauri::command]
async fn duplicate_instance(id: String) -> Result<Instance, String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &id).await?;
    instance.duplicate(&dirs).await.map_err(text)
}

#[tauri::command]
async fn list_worlds(id: String) -> Result<Vec<World>, String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &id).await?;
    worlds::recent(&instance.game_dir(&dirs), WORLD_LIST_LIMIT)
        .await
        .map_err(text)
}

#[tauri::command]
async fn backup_world(id: String, folder: String) -> Result<String, String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &id).await?;
    worlds::backup(&instance.game_dir(&dirs), &folder)
        .await
        .map_err(text)
}

#[tauri::command]
async fn recent_worlds(id: String) -> Result<Vec<World>, String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &id).await?;
    worlds::recent(&instance.game_dir(&dirs), RECENT_WORLD_LIMIT)
        .await
        .map_err(text)
}

#[tauri::command]
async fn search_packs(query: String) -> Result<Vec<ModHit>, String> {
    let client = new_client().map_err(text)?;
    pack::search_packs(&client, &query).await.map_err(text)
}

#[tauri::command]
async fn install_pack(
    app: AppHandle,
    project_id: String,
    name: String,
) -> Result<Instance, String> {
    let dirs = data_dirs()?;
    let config = LauncherConfig::load(&dirs).await.map_err(text)?;
    let client = new_client().map_err(text)?;
    let mut last_progress = Instant::now();
    pack::install_pack(
        &client,
        &dirs,
        &project_id,
        &name,
        config.download_concurrency,
        move |progress| {
            let is_last = progress.finished_files == progress.total_files;
            if is_last || last_progress.elapsed() >= PROGRESS_INTERVAL {
                last_progress = Instant::now();
                let stage = PrepareStage::Game;
                // 進度只用於顯示，送不出去不影響下載本身。
                let _ = app.emit(INSTALL_EVENT, LaunchEvent::Progress { stage, progress });
            }
        },
    )
    .await
    .map_err(text)
}

#[tauri::command]
async fn search_mods(
    query: String,
    version: String,
    loader: LoaderKind,
) -> Result<Vec<ModHit>, String> {
    let client = new_client().map_err(text)?;
    modrinth::search(&client, &query, &version, loader)
        .await
        .map_err(text)
}

#[tauri::command]
async fn install_mod(id: String, project_id: String) -> Result<Vec<String>, String> {
    let dirs = data_dirs()?;
    let config = LauncherConfig::load(&dirs).await.map_err(text)?;
    let instance = load_instance(&dirs, &id).await?;
    let client = new_client().map_err(text)?;
    let other_mods_dirs = mods::all_mods_dirs(&dirs).await.map_err(text)?;
    modrinth::install(
        &client,
        &project_id,
        &instance.version,
        instance.loader,
        &mods::mods_dir(&instance.game_dir(&dirs)),
        &other_mods_dirs,
        config.download_concurrency,
    )
    .await
    .map_err(text)
}

#[tauri::command]
async fn list_mods(id: String) -> Result<Vec<ModFile>, String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &id).await?;
    mods::list(&instance.game_dir(&dirs)).await.map_err(text)
}

#[tauri::command]
async fn set_mod_enabled(action: ModAction) -> Result<(), String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &action.instance_id).await?;
    mods::set_enabled(&instance.game_dir(&dirs), &action.name, action.enabled)
        .await
        .map_err(text)
}

#[tauri::command]
async fn delete_mod(action: ModAction) -> Result<(), String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &action.instance_id).await?;
    mods::delete(&instance.game_dir(&dirs), &action.name, action.enabled)
        .await
        .map_err(text)
}

#[tauri::command]
async fn open_mods_folder(id: String) -> Result<(), String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &id).await?;
    let folder = mods::mods_dir(&instance.game_dir(&dirs));
    std::fs::create_dir_all(&folder).map_err(text)?;
    open_folder(&folder).map_err(text)
}

#[tauri::command]
async fn restore_account(state: State<'_, AppState>) -> Result<Option<Profile>, String> {
    Ok(current_session(&state)
        .await?
        .map(|session| session.profile))
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
    remember_account(&profile).await;
    Ok(profile)
}

/// 帳號清單只是方便切換，記不下來也不影響這次登入。
async fn remember_account(profile: &Profile) {
    if let Ok(dirs) = data_dirs() {
        let _ = accounts::remember(&dirs, profile).await;
    }
}

#[tauri::command]
async fn list_accounts() -> Result<Vec<Profile>, String> {
    accounts::list(&data_dirs()?).await.map_err(text)
}

#[tauri::command]
async fn switch_account(state: State<'_, AppState>, id: String) -> Result<Option<Profile>, String> {
    accounts::switch(&id).map_err(text)?;
    state.set_session(None)?;
    Ok(current_session(&state)
        .await?
        .map(|session| session.profile))
}

#[tauri::command]
async fn cleanup_preview() -> Result<CleanupSummary, String> {
    cleanup::preview(&data_dirs()?).await.map_err(text)
}

#[tauri::command]
async fn cleanup_run() -> Result<u64, String> {
    cleanup::run(&data_dirs()?).await.map_err(text)
}

#[tauri::command]
async fn list_import_sources() -> Result<Vec<ImportSource>, String> {
    Ok(import::find_sources(&data_dirs()?).await)
}

#[tauri::command]
async fn import_world(id: String, source: String, folder: String) -> Result<String, String> {
    let dirs = data_dirs()?;
    let instance = load_instance(&dirs, &id).await?;
    let known = import::find_sources(&dirs)
        .await
        .into_iter()
        .any(|found| found.path.as_os_str() == source.as_str());
    if !known {
        return Err("找不到這個匯入來源".to_string());
    }
    import::import_world(
        std::path::Path::new(&source),
        &folder,
        &instance.game_dir(&dirs),
    )
    .await
    .map_err(text)
}

#[tauri::command]
fn has_saved_account() -> Result<bool, String> {
    auth::has_saved_account().map_err(text)
}

#[tauri::command]
async fn sign_out(state: State<'_, AppState>) -> Result<(), String> {
    let current = state.session()?;
    auth::sign_out().map_err(text)?;
    if let Some(session) = current {
        accounts::forget(&data_dirs()?, &session.profile.id)
            .await
            .map_err(text)?;
    }
    state.set_session(None)
}

#[tauri::command]
async fn install_game(app: AppHandle, version: String) -> Result<(), String> {
    let dirs = data_dirs()?;
    let config = LauncherConfig::load(&dirs).await.map_err(text)?;
    let mut last_progress = Instant::now();
    prepare_version(
        &version,
        &dirs,
        config.download_concurrency,
        move |stage, progress| {
            let is_last = progress.finished_files == progress.total_files;
            if is_last || last_progress.elapsed() >= PROGRESS_INTERVAL {
                last_progress = Instant::now();
                // 進度只用於顯示，送不出去不影響下載本身。
                let _ = app.emit(INSTALL_EVENT, LaunchEvent::Progress { stage, progress });
            }
        },
    )
    .await
    .map(|_| ())
    .map_err(text)
}

#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    open_in_browser(&url).map_err(text)
}

#[tauri::command]
async fn check_update() -> Result<Option<UpdateInfo>, String> {
    let client = new_client().map_err(text)?;
    update::check(&client).await.map_err(text)
}

#[tauri::command]
async fn apply_update(app: AppHandle, update: UpdateInfo) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("開發模式不能自動更新".to_string());
    }
    let client = new_client().map_err(text)?;
    let exe = update::install(&client, &update).await.map_err(text)?;
    Command::new(exe).spawn().map_err(text)?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
async fn play_game(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    offline_name: Option<String>,
    world: Option<String>,
) -> Result<(), String> {
    state.launch_cancel.reset();
    let dirs = data_dirs()?;
    let config = LauncherConfig::load(&dirs).await.map_err(text)?;
    let session = match offline_name {
        Some(name) if config.is_feature_enabled(OFFLINE_FEATURE) => {
            auth::offline_session(&name).map_err(text)?
        }
        Some(_) => return Err("離線模式已關閉".to_string()),
        None => current_session(&state)
            .await?
            .ok_or_else(|| "請先登入微軟帳號".to_string())?,
    };
    let instance = load_instance(&dirs, &instance_id).await?;

    let mut last_progress = Instant::now();
    play(
        &session,
        &instance,
        &dirs,
        PlayOptions {
            concurrency: config.download_concurrency,
            analyze_crash: config.is_feature_enabled(CRASH_FEATURE),
            world: world.as_deref(),
        },
        &state.launch_cancel,
        move |event| {
            if let LaunchEvent::Progress { progress, .. } = &event {
                let is_last = progress.finished_files == progress.total_files;
                if !is_last && last_progress.elapsed() < PROGRESS_INTERVAL {
                    return;
                }
                last_progress = Instant::now();
            }
            // 事件只用於顯示，送不出去不影響遊戲本身。
            let _ = app.emit(LAUNCH_EVENT, event);
        },
    )
    .await
    .map_err(text)
}

#[tauri::command]
fn cancel_launch(state: State<'_, AppState>) {
    state.launch_cancel.cancel();
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
    if let Some(session) = &session {
        remember_account(&session.profile).await;
    }
    Ok(session)
}

/// 預設大小就是最小大小。系統算最小值時包含隱形邊框，所以用開啟時的實際外框，不寫死數字。
fn lock_minimum_size(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if let (Ok(size), Ok(scale)) = (window.outer_size(), window.scale_factor()) {
        // 設定失敗也不影響使用，只是窗口能再縮小一點。
        let _ = window.set_min_size(Some(size.to_logical::<f64>(scale)));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    update::remove_old_version();
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            lock_minimum_size(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_versions,
            system_memory_mb,
            load_config,
            save_config,
            list_instances,
            create_instance,
            save_instance,
            delete_instance,
            open_instance_folder,
            duplicate_instance,
            list_worlds,
            backup_world,
            recent_worlds,
            search_packs,
            install_pack,
            search_mods,
            install_mod,
            list_mods,
            set_mod_enabled,
            delete_mod,
            open_mods_folder,
            restore_account,
            login_start,
            login_finish,
            sign_out,
            list_accounts,
            switch_account,
            cleanup_preview,
            cleanup_run,
            list_import_sources,
            import_world,
            has_saved_account,
            install_game,
            open_link,
            check_update,
            apply_update,
            play_game,
            cancel_launch
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
