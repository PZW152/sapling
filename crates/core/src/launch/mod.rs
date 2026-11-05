//! 啟動遊戲：組裝命令行、啟動進程、轉發日誌。

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::auth::Session;
use crate::download::DownloadProgress;
use crate::instance::Instance;
use crate::platform::{CREATE_NO_WINDOW, DataDirs};
use crate::versions::details::{Argument, VersionDetails};
use crate::versions::install::{InstallError, PrepareStage, PreparedVersion, prepare_version};

const LAUNCHER_NAME: &str = "Sapling Launcher";
const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
const CLASSPATH_SEPARATOR: &str = ";";
/// 遊戲需要這兩個值，但 Sapling 不使用遙測，也不需要真實的 Xbox ID。
const UNUSED_ID: &str = "0";
const HIDDEN_TOKEN: &str = "***";
const LOG_CHANNEL_SIZE: usize = 256;

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
    #[error("遊戲異常結束（代碼 {code}），請查看日誌")]
    Crashed { code: i32 },
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
    mut on_event: impl FnMut(LaunchEvent) + Send,
) -> Result<(), LaunchError> {
    let prepared = prepare_version(&instance.version, dirs, concurrency, |stage, progress| {
        on_event(LaunchEvent::Progress { stage, progress })
    })
    .await?;
    let details = instance.loader.loader().apply(prepared.details.clone());
    let prepared = PreparedVersion { details, ..prepared };

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
        .kill_on_drop(false)
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
        let line = hide_token(&line, session.access_token());
        on_event(LaunchEvent::Log { line });
    }

    let status = child.wait().await.map_err(LaunchError::Spawn)?;
    if status.success() {
        Ok(())
    } else {
        Err(LaunchError::Crashed {
            code: status.code().unwrap_or(-1),
        })
    }
}

async fn forward_lines(stream: impl AsyncRead + Unpin, sender: mpsc::Sender<String>) {
    let mut lines = BufReader::new(stream).lines();
    // 遊戲輸出不一定是 UTF-8，讀不了的那一行就結束轉發，不影響遊戲本身。
    while let Ok(Some(line)) = lines.next_line().await {
        if sender.send(line).await.is_err() {
            break;
        }
    }
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

    let mut arguments = vec![format!("-Xmx{}M", instance.memory_mb)];
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
    values.iter().fold(argument.to_string(), |text, (key, value)| {
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

    #[test]
    fn hides_the_token_in_game_output() {
        assert_eq!(
            hide_token("Setting token: abc123 done", "abc123"),
            "Setting token: *** done"
        );
    }
}
