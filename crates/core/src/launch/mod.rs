//! 啟動遊戲：組裝命令行、啟動進程、轉發問題記錄。

pub mod crash;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;
use thiserror::Error;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::auth::Session;
use crate::download::DownloadProgress;
use crate::instance::Instance;
use crate::platform::{CREATE_NO_WINDOW, DataDirs, total_memory_mb};
use crate::versions::details::{Argument, VersionDetails};
use crate::versions::install::{InstallError, PrepareStage, PreparedVersion, prepare_version};

const LAUNCHER_NAME: &str = "Sapling Launcher";
const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLASSPATH_SEPARATOR: &str = ";";
/// 遊戲需要這兩個值，但 Sapling 不使用遙測，也不需要真實的 Xbox ID。
const UNUSED_ID: &str = "0";
const HIDDEN_TOKEN: &str = "***";
const LOG_CHANNEL_SIZE: usize = 256;
/// 讓遊戲輸出固定用 UTF-8，日誌窗口的中文才不會變亂碼。
const OUTPUT_ENCODING_ARGUMENTS: [&str; 2] = ["-Dstdout.encoding=UTF-8", "-Dstderr.encoding=UTF-8"];

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error(transparent)]
    Install(#[from] InstallError),
    #[error("無法建立遊戲資料夾：{}", path.display())]
    GameDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
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

/// 準備檔案並啟動遊戲，一直等到遊戲結束。
pub async fn play(
    session: &Session,
    instance: &Instance,
    dirs: &DataDirs,
    concurrency: usize,
    analyze_crash: bool,
    mut on_event: impl FnMut(LaunchEvent) + Send,
) -> Result<(), LaunchError> {
    let prepared = prepare_version(&instance.version, dirs, concurrency, |stage, progress| {
        on_event(LaunchEvent::Progress { stage, progress })
    })
    .await?;
    let details = instance.loader.loader().apply(prepared.details.clone());
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

    let arguments = build_arguments(session, instance, &prepared.details, dirs, &natives_dir);
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
            let line = hide_token(&line, session.access_token());
            on_event(LaunchEvent::Log { line });
        }
    }

    let status = child.wait().await.map_err(LaunchError::Spawn)?;
    if status.success() {
        Ok(())
    } else if !analyze_crash {
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
    arguments.extend(expand(&details.arguments.jvm, &values));
    arguments.push(details.main_class.clone());
    arguments.extend(expand(&details.arguments.game, &values));
    arguments
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
        ("user_type", "msa".to_string()),
        ("version_name", details.id.clone()),
        ("version_type", details.release_type.clone()),
        ("game_directory", path_text(&instance.game_dir(dirs))),
        ("assets_root", path_text(&dirs.assets())),
        ("assets_index_name", details.asset_index.id.clone()),
        ("natives_directory", path_text(natives_dir)),
        ("launcher_name", LAUNCHER_NAME.to_string()),
        ("launcher_version", LAUNCHER_VERSION.to_string()),
        ("classpath", classpath(details, dirs)),
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
        .map(|library| libraries_dir.join(&library.downloads.artifact.path))
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
    fn hides_the_token_in_game_output() {
        assert_eq!(
            hide_token("Setting token: abc123 done", "abc123"),
            "Setting token: *** done"
        );
    }
}
