//! 啟動遊戲：組裝命令行、啟動進程、轉發問題記錄。

pub mod crash;
mod natives;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;
use thiserror::Error;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, watch};

use crate::auth::Session;
use crate::download::DownloadProgress;
use crate::instance::{Instance, worlds};
use crate::loader::{self, LoaderError};
use crate::platform::{CREATE_NO_WINDOW, DataDirs, total_memory_mb};
use crate::versions::details::{Argument, VersionDetails};
use crate::versions::install::{
    InstallError, PrepareStage, PreparedVersion, log_config_argument, prepare_version,
    request_full_check,
};

const LAUNCHER_NAME: &str = "Sapling Launcher";
const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLASSPATH_SEPARATOR: &str = ";";
/// 遊戲需要這兩個值，但 Sapling 不使用遙測，也不需要真實的 Xbox ID。
const UNUSED_ID: &str = "0";
const HIDDEN_TOKEN: &str = "***";
const LATEST_LOG: &str = "logs/latest.log";
/// 舊版遊戲要求這個參數，內容可以是空的。
const EMPTY_PROPERTIES: &str = "{}";
const LOG_CHANNEL_SIZE: usize = 256;
/// 版本 JSON 用這個功能開關標記支援直接進入世界，1.20 起才有。
const QUICK_PLAY_FEATURE: &str = "is_quick_play_singleplayer";
const QUICK_PLAY_ARGUMENT: &str = "--quickPlaySingleplayer";
/// 讓遊戲輸出固定用 UTF-8，日誌窗口的中文才不會變亂碼。
const OUTPUT_ENCODING_ARGUMENTS: [&str; 2] = ["-Dstdout.encoding=UTF-8", "-Dstderr.encoding=UTF-8"];

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error(transparent)]
    Install(#[from] InstallError),
    #[error(transparent)]
    Loader(#[from] LoaderError),
    #[error("無法建立遊戲資料夾：{}", path.display())]
    GameDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("找不到這個世界，可能已經被移動或刪除")]
    WorldNotFound,
    #[error("無法啟動 Java，請重新下載遊戲檔案後再試一次")]
    Spawn(#[source] io::Error),
    #[error("遊戲異常結束。{advice}")]
    Crashed { advice: &'static str },
    #[error("遊戲異常結束，請查看問題記錄。")]
    CrashedWithoutAnalysis,
}

/// 啟動過程中發生的事，轉發給介面顯示。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LaunchEvent {
    Progress {
        stage: PrepareStage,
        #[serde(flatten)]
        progress: DownloadProgress,
    },
    Started,
    Log {
        line: String,
    },
}

/// 取消「準備遊戲檔案」這一步。遊戲開始後不受影響。
pub struct LaunchCancel(watch::Sender<bool>);

impl Default for LaunchCancel {
    fn default() -> Self {
        Self(watch::Sender::new(false))
    }
}

impl LaunchCancel {
    /// 開始新的一次啟動前清除上一次的取消。
    pub fn reset(&self) {
        self.0.send_replace(false);
    }

    pub fn cancel(&self) {
        self.0.send_replace(true);
    }

    async fn cancelled(&self) {
        // 發送端就在 `self` 裡，等待不會因為它消失而失敗。
        let _ = self.0.subscribe().wait_for(|cancelled| *cancelled).await;
    }
}

/// 舊版遊戲會把 token 寫進日誌，玩家常把這個檔案貼到網上求助，所以遊戲結束後抹掉。
/// 日誌可能不是 UTF-8，要用位元組處理。清理失敗不影響遊戲本身。
async fn scrub_token(log: &Path, token: &str) {
    if let Ok(content) = tokio::fs::read(log).await
        && let Some(cleaned) = without_token(&content, token.as_bytes())
    {
        let _ = tokio::fs::write(log, cleaned).await;
    }
}

/// 把內容裡的 token 換掉。沒有出現時回傳 `None`。
fn without_token(content: &[u8], token: &[u8]) -> Option<Vec<u8>> {
    if token.is_empty() || !content.windows(token.len()).any(|part| part == token) {
        return None;
    }
    let mut cleaned = Vec::with_capacity(content.len());
    let mut rest = content;
    while !rest.is_empty() {
        if rest.starts_with(token) {
            cleaned.extend_from_slice(HIDDEN_TOKEN.as_bytes());
            rest = &rest[token.len()..];
        } else {
            cleaned.push(rest[0]);
            rest = &rest[1..];
        }
    }
    Some(cleaned)
}

/// 這次啟動的選項。
pub struct PlayOptions<'a> {
    pub concurrency: usize,
    pub analyze_crash: bool,
    /// 啟動後直接進入的世界資料夾，`None` 代表停在主畫面。
    pub world: Option<&'a str>,
}

/// 準備檔案並啟動遊戲，一直等到遊戲結束。準備途中被取消時直接結束。
pub async fn play(
    session: &Session,
    instance: &Instance,
    dirs: &DataDirs,
    options: PlayOptions<'_>,
    cancel: &LaunchCancel,
    mut on_event: impl FnMut(LaunchEvent) + Send,
) -> Result<(), LaunchError> {
    let PlayOptions {
        concurrency,
        analyze_crash,
        world,
    } = options;
    if world.is_some_and(|folder| !worlds::exists(&instance.game_dir(dirs), folder)) {
        return Err(LaunchError::WorldNotFound);
    }
    let prepare = prepare_version(&instance.version, dirs, concurrency, |stage, progress| {
        on_event(LaunchEvent::Progress { stage, progress })
    });
    let prepared = tokio::select! {
        prepared = prepare => prepared?,
        () = cancel.cancelled() => return Ok(()),
    };
    let apply_loader = loader::prepare(
        instance.loader,
        &instance.loader_version,
        prepared.details.clone(),
        &prepared.java,
        dirs,
        concurrency,
    );
    let details = tokio::select! {
        details = apply_loader => details?,
        () = cancel.cancelled() => return Ok(()),
    };
    let prepared = PreparedVersion {
        details,
        ..prepared
    };

    let game_dir = instance.game_dir(dirs);
    let natives_dir = dirs.versions().join(&prepared.details.id).join("natives");
    for path in [&game_dir, &natives_dir] {
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|source| LaunchError::GameDir {
                path: path.clone(),
                source,
            })?;
    }

    natives::extract_all(&prepared.details, dirs, &natives_dir)
        .await
        .map_err(|source| LaunchError::GameDir {
            path: natives_dir.clone(),
            source,
        })?;

    let mut arguments = build_arguments(session, instance, &prepared.details, dirs, &natives_dir);
    arguments.extend(world_arguments(&prepared.details, world));
    let mut child = Command::new(&prepared.java)
        .args(&arguments)
        .current_dir(&game_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(LaunchError::Spawn)?;
    on_event(LaunchEvent::Started);

    let (sender, mut receiver) = mpsc::channel(LOG_CHANNEL_SIZE);
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(forward_lines(stdout, sender.clone()));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(forward_lines(stderr, sender.clone()));
    }
    drop(sender);
    while let Some(line) = receiver.recv().await {
        if is_problem_line(&line) {
            // 離線的 token 只是佔位的 0，替換掉會弄亂記錄裡所有的數字。
            let line = if session.is_offline() {
                line
            } else {
                hide_token(&line, session.access_token())
            };
            on_event(LaunchEvent::Log { line });
        }
    }

    let status = child.wait().await.map_err(LaunchError::Spawn)?;
    if !session.is_offline() {
        scrub_token(&game_dir.join(LATEST_LOG), session.access_token()).await;
    }
    if status.success() {
        return Ok(());
    }
    // 只影響下次啟動的速度和檢查範圍，寫不進去也不該蓋過崩潰原因。
    let _ = request_full_check(dirs, &instance.version).await;
    if !analyze_crash {
        Err(LaunchError::CrashedWithoutAnalysis)
    } else {
        let reports = crash::read_reports(&game_dir).await;
        Err(LaunchError::Crashed {
            advice: crash::analyze(&reports),
        })
    }
}

/// 一直讀到遊戲關閉輸出為止。如果中途停止讀取，輸出管道填滿後遊戲會卡死。
async fn forward_lines(stream: impl AsyncRead + Unpin, sender: mpsc::Sender<String>) {
    let mut reader = BufReader::new(stream);
    while let Some(line) = read_line_lossy(&mut reader).await {
        if sender.send(line).await.is_err() {
            break;
        }
    }
}

/// 讀取一行。遊戲輸出不一定是 UTF-8，無法解碼的字元會被替換，不會中斷讀取。
async fn read_line_lossy(reader: &mut (impl AsyncBufRead + Unpin)) -> Option<String> {
    let mut bytes = Vec::new();
    match reader.read_until(b'\n', &mut bytes).await {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(String::from_utf8_lossy(&bytes).trim_end().to_string()),
    }
}

/// 只保留警告、錯誤和例外堆疊。完整日誌由遊戲自己寫在實例的 `logs/latest.log`。
fn is_problem_line(line: &str) -> bool {
    const MARKERS: [&str; 6] = [
        "/WARN]",
        "/ERROR]",
        "/FATAL]",
        "Exception",
        "Caused by:",
        "\tat ",
    ];
    MARKERS.iter().any(|marker| line.contains(marker))
}

fn hide_token(line: &str, token: &str) -> String {
    line.replace(token, HIDDEN_TOKEN)
}

/// `java.exe` 之後的全部參數：JVM 參數、主類、遊戲參數。
fn build_arguments(
    session: &Session,
    instance: &Instance,
    details: &VersionDetails,
    dirs: &DataDirs,
    natives_dir: &Path,
) -> Vec<String> {
    let values = placeholder_values(session, instance, details, dirs, natives_dir);

    let memory_mb = instance.effective_memory_mb(total_memory_mb());
    let mut arguments = vec![format!("-Xmx{memory_mb}M")];
    arguments.extend(OUTPUT_ENCODING_ARGUMENTS.map(str::to_string));
    arguments.extend(instance.java_args.split_whitespace().map(str::to_string));
    arguments.extend(log_config_argument(details, dirs));
    arguments.extend(expand(&details.arguments.jvm, &values));
    arguments.push(details.main_class.clone());
    arguments.extend(expand(&details.arguments.game, &values));
    arguments
}

/// 直接進入世界的參數。舊版本不支援時照常停在主畫面。
fn world_arguments(details: &VersionDetails, world: Option<&str>) -> Vec<String> {
    let supported = details
        .arguments
        .game
        .iter()
        .any(|argument| argument.depends_on_feature(QUICK_PLAY_FEATURE));
    match world {
        Some(folder) if supported => vec![QUICK_PLAY_ARGUMENT.to_string(), folder.to_string()],
        _ => Vec::new(),
    }
}

fn expand(arguments: &[Argument], values: &HashMap<&str, String>) -> Vec<String> {
    arguments
        .iter()
        .flat_map(Argument::values_for_this_machine)
        .map(|argument| fill_placeholders(argument, values))
        .collect()
}

fn fill_placeholders(argument: &str, values: &HashMap<&str, String>) -> String {
    values
        .iter()
        .fold(argument.to_string(), |text, (key, value)| {
            text.replace(&format!("${{{key}}}"), value)
        })
}

fn placeholder_values(
    session: &Session,
    instance: &Instance,
    details: &VersionDetails,
    dirs: &DataDirs,
    natives_dir: &Path,
) -> HashMap<&'static str, String> {
    HashMap::from([
        ("auth_player_name", session.profile.name.clone()),
        ("auth_uuid", session.profile.id.clone()),
        ("auth_access_token", session.access_token().to_string()),
        ("auth_xuid", UNUSED_ID.to_string()),
        ("clientid", UNUSED_ID.to_string()),
        ("user_type", session.user_type().to_string()),
        ("version_name", details.id.clone()),
        ("version_type", details.release_type.clone()),
        ("game_directory", path_text(&instance.game_dir(dirs))),
        ("assets_root", path_text(&dirs.assets())),
        ("assets_index_name", details.asset_index.id.clone()),
        ("natives_directory", path_text(natives_dir)),
        ("launcher_name", LAUNCHER_NAME.to_string()),
        ("launcher_version", LAUNCHER_VERSION.to_string()),
        ("classpath", classpath(details, dirs)),
        ("library_directory", path_text(&dirs.libraries())),
        ("user_properties", EMPTY_PROPERTIES.to_string()),
        ("auth_session", session.access_token().to_string()),
        ("classpath_separator", CLASSPATH_SEPARATOR.to_string()),
    ])
}

fn classpath(details: &VersionDetails, dirs: &DataDirs) -> String {
    let libraries_dir = dirs.libraries();
    let client_jar = dirs
        .versions()
        .join(&details.id)
        .join(format!("{}.jar", details.id));
    details
        .libraries_for_this_machine()
        .into_iter()
        .filter_map(|library| library.downloads.artifact.as_ref())
        .map(|artifact| libraries_dir.join(&artifact.path))
        .chain(std::iter::once(client_jar))
        .map(|path| path_text(&path))
        .collect::<Vec<_>>()
        .join(CLASSPATH_SEPARATOR)
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancel_is_remembered_until_the_next_launch_resets_it() {
        let cancel = LaunchCancel::default();

        cancel.cancel();
        cancel.cancelled().await;

        cancel.reset();
        let waiting =
            tokio::time::timeout(std::time::Duration::from_millis(10), cancel.cancelled());
        assert!(waiting.await.is_err());
    }

    #[test]
    fn fills_known_placeholders_and_keeps_surrounding_text() {
        let values = HashMap::from([("natives_directory", "C:\\n".to_string())]);
        assert_eq!(
            fill_placeholders("-Djava.library.path=${natives_directory}/java", &values),
            "-Djava.library.path=C:\\n/java"
        );
    }

    #[test]
    fn expands_only_arguments_for_this_machine() {
        let arguments: Vec<Argument> = serde_json::from_str(
            r#"[
                "--username", "${auth_player_name}",
                { "rules": [{ "action": "allow", "features": { "is_demo_user": true } }], "value": "--demo" }
            ]"#,
        )
        .unwrap();
        let values = HashMap::from([("auth_player_name", "Steve".to_string())]);
        assert_eq!(expand(&arguments, &values), ["--username", "Steve"]);
    }

    fn details_with_game_arguments(game: &str) -> VersionDetails {
        VersionDetails::parse(&format!(
            r#"{{
                "id": "26.3",
                "mainClass": "m",
                "arguments": {{ "game": {game}, "jvm": [] }},
                "assetIndex": {{ "id": "34", "sha1": "a", "size": 2, "totalSize": 3, "url": "u" }},
                "downloads": {{ "client": {{ "sha1": "c", "size": 4, "url": "u" }} }},
                "javaVersion": {{ "component": "j", "majorVersion": 25 }},
                "libraries": []
            }}"#
        ))
        .unwrap()
    }

    #[test]
    fn enters_the_world_when_the_version_supports_it() {
        let details = details_with_game_arguments(
            r#"[{ "rules": [{ "action": "allow", "features": { "is_quick_play_singleplayer": true } }],
                  "value": ["--quickPlaySingleplayer", "${quickPlaySingleplayer}"] }]"#,
        );

        assert_eq!(
            world_arguments(&details, Some("My World")),
            ["--quickPlaySingleplayer", "My World"]
        );
        assert!(world_arguments(&details, None).is_empty());
    }

    #[test]
    fn older_versions_stay_on_the_title_screen() {
        let details = details_with_game_arguments(r#"["--username"]"#);

        assert!(world_arguments(&details, Some("My World")).is_empty());
    }

    #[test]
    fn classpath_ends_with_the_game_jar() {
        let details = VersionDetails::parse(
            r#"{
                "id": "26.3",
                "mainClass": "m",
                "assetIndex": { "id": "34", "sha1": "a", "size": 2, "totalSize": 3, "url": "u" },
                "downloads": { "client": { "sha1": "c", "size": 4, "url": "u" } },
                "javaVersion": { "component": "j", "majorVersion": 25 },
                "libraries": [{
                    "name": "a:b:1",
                    "downloads": { "artifact": { "path": "a/b.jar", "sha1": "l", "size": 5, "url": "u" } }
                }]
            }"#,
        )
        .unwrap();
        let classpath = classpath(&details, &DataDirs::at("root"));
        let parts: Vec<&str> = classpath.split(CLASSPATH_SEPARATOR).collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].ends_with("b.jar"));
        assert!(parts[1].ends_with("26.3.jar"));
    }

    #[tokio::test]
    async fn keeps_reading_after_a_line_that_is_not_utf8() {
        let mut reader: &[u8] = b"first\r\n\xB4\xFA\xB8\xD5\nlast";

        let first = read_line_lossy(&mut reader).await;
        let broken = read_line_lossy(&mut reader).await;
        let last = read_line_lossy(&mut reader).await;

        assert_eq!(first.as_deref(), Some("first"));
        assert!(broken.is_some());
        assert_eq!(last.as_deref(), Some("last"));
        assert_eq!(read_line_lossy(&mut reader).await, None);
    }

    #[test]
    fn keeps_only_warnings_errors_and_stack_traces() {
        assert!(is_problem_line("[12:00:00] [Render thread/WARN]: slow"));
        assert!(is_problem_line("[12:00:00] [main/ERROR]: failed"));
        assert!(is_problem_line("java.lang.NullPointerException: x"));
        assert!(is_problem_line(
            "\tat net.minecraft.client.Main.main(Main.java:1)"
        ));
        assert!(!is_problem_line("[12:00:00] [Render thread/INFO]: Loaded"));
    }

    #[test]
    fn removes_the_token_from_a_log_file_even_when_it_is_not_utf8() {
        let log = b"\xB4\xFA Session ID is token:secret:uuid\nsecret again\n";

        let cleaned = without_token(log, b"secret").unwrap();

        let expected = b"\xB4\xFA Session ID is token:***:uuid\n*** again\n";
        assert_eq!(cleaned, expected);
    }

    #[test]
    fn leaves_a_clean_log_alone() {
        assert_eq!(without_token(b"nothing here", b"secret"), None);
    }

    #[test]
    fn hides_the_token_in_game_output() {
        assert_eq!(
            hide_token("Setting token: abc123 done", "abc123"),
            "Setting token: *** done"
        );
    }
}
