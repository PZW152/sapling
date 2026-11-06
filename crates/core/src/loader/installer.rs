//! Forge 和 NeoForge 共用的安裝流程：兩者都用同一種安裝程式格式。
//!
//! 安裝程式裡有版本資料、函式庫清單，以及一串處理器。處理器是下載來的 Java 程式，
//! 用來把原版遊戲打上補丁，官方安裝程式也是這樣執行它們。

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Deserialize;
use serde_json::Value;
use tokio::process::Command;
use zip::ZipArchive;

use crate::download::{DownloadTask, download_all, download_file, fetch_text, has_expected_sha1};
use crate::platform::{CREATE_NO_WINDOW, DataDirs};
use crate::storage::{read_json, write_json};
use crate::versions::details::Library;

use super::LoaderError;
use super::profile::{LoaderProfile, ProfileLibrary, maven_path};

const PROFILE_FILE: &str = "install_profile.json";
const MANIFEST_FILE: &str = "META-INF/MANIFEST.MF";
const MAIN_CLASS_KEY: &str = "Main-Class:";
const MAVEN_FOLDER: &str = "maven/";
const SIDE: &str = "client";
const CLASSPATH_SEPARATOR: &str = ";";
const INSTALLED_MARKER: &str = ".installed";

/// 要安裝的載入器。
pub struct InstallRequest {
    /// 本機保存用的名稱。
    pub id: String,
    pub installer_url: String,
    pub game_version: String,
}

#[derive(Debug, Deserialize)]
struct InstallProfile {
    json: String,
    #[serde(default)]
    data: HashMap<String, SidedValue>,
    #[serde(default)]
    processors: Vec<Processor>,
    #[serde(default)]
    libraries: Vec<ProfileLibrary>,
}

#[derive(Debug, Deserialize)]
struct SidedValue {
    client: String,
}

#[derive(Debug, Deserialize)]
struct Processor {
    sides: Option<Vec<String>>,
    jar: String,
    #[serde(default)]
    classpath: Vec<String>,
    args: Vec<String>,
    #[serde(default)]
    outputs: HashMap<String, String>,
}

impl Processor {
    fn runs_on_client(&self) -> bool {
        self.sides
            .as_ref()
            .is_none_or(|sides| sides.iter().any(|side| side == SIDE))
    }
}

/// 確保載入器已經安裝，回傳它的版本資料。裝過就直接讀本機，不需要網路。
pub async fn ensure_installed(
    client: &reqwest::Client,
    request: &InstallRequest,
    java: &Path,
    dirs: &DataDirs,
    concurrency: usize,
) -> Result<Value, LoaderError> {
    let folder = dirs.versions().join(&request.id);
    let details_path = folder.join(format!("{}.json", request.id));
    let marker = folder.join(INSTALLED_MARKER);
    if marker.exists()
        && let Some(saved) = read_json(&details_path).await?
    {
        return Ok(saved);
    }

    let installer = folder.join("installer.jar");
    let sha1 = fetch_text(client, &format!("{}.sha1", request.installer_url)).await?;
    download_file(
        client,
        &DownloadTask {
            url: request.installer_url.clone(),
            path: installer.clone(),
            sha1: sha1.trim().to_string(),
            size: 0,
        },
    )
    .await?;

    let contents = read_installer(installer.clone(), folder.clone(), dirs.libraries()).await?;
    let details: Value = serde_json::from_slice(&contents.version_json)?;
    let version: LoaderProfile = serde_json::from_value(details.clone())?;

    let mut wanted = version.libraries()?;
    for library in &contents.profile.libraries {
        wanted.push(library.to_library()?);
    }
    download_all(client, library_tasks(&wanted, dirs), concurrency, |_| {}).await?;

    let context = Context::new(
        &contents.profile,
        &request.game_version,
        &installer,
        &folder,
        dirs,
    )?;
    for processor in contents
        .profile
        .processors
        .iter()
        .filter(|processor| processor.runs_on_client())
    {
        run_processor(processor, &context, java, dirs).await?;
    }

    write_json(&details_path, &details).await?;
    tokio::fs::write(&marker, b"")
        .await
        .map_err(|source| LoaderError::Write {
            path: marker,
            source,
        })?;
    Ok(details)
}

/// 需要下載的函式庫。沒有網址的是處理器產生的，同一個檔案只下載一次。
pub fn library_tasks(libraries: &[Library], dirs: &DataDirs) -> Vec<DownloadTask> {
    let mut seen = HashSet::new();
    libraries
        .iter()
        .map(|library| &library.downloads.artifact)
        .filter(|artifact| !artifact.url.is_empty() && seen.insert(artifact.path.clone()))
        .map(|artifact| DownloadTask {
            url: artifact.url.clone(),
            path: dirs.libraries().join(&artifact.path),
            sha1: artifact.sha1.clone(),
            size: artifact.size,
        })
        .collect()
}

struct InstallerContents {
    profile: InstallProfile,
    version_json: Vec<u8>,
}

/// 讀出安裝資料，並解開安裝程式自帶的函式庫和資料檔。
async fn read_installer(
    installer: PathBuf,
    folder: PathBuf,
    libraries: PathBuf,
) -> Result<InstallerContents, LoaderError> {
    tokio::task::spawn_blocking(move || {
        let mut archive = open_archive(&installer)?;
        let profile: InstallProfile =
            serde_json::from_slice(&read_entry(&mut archive, PROFILE_FILE)?)?;
        let version_json = read_entry(&mut archive, profile.json.trim_start_matches('/'))?;
        extract_entries(&mut archive, &folder, &libraries)?;
        Ok(InstallerContents {
            profile,
            version_json,
        })
    })
    .await
    .map_err(|_| LoaderError::BadInstaller)?
}

fn open_archive(path: &Path) -> Result<ZipArchive<File>, LoaderError> {
    let file = File::open(path).map_err(|_| LoaderError::BadInstaller)?;
    ZipArchive::new(file).map_err(|_| LoaderError::BadInstaller)
}

fn read_entry(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>, LoaderError> {
    let mut entry = archive
        .by_name(name)
        .map_err(|_| LoaderError::BadInstaller)?;
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|_| LoaderError::BadInstaller)?;
    Ok(bytes)
}

/// `maven/` 解到函式庫資料夾，`data/` 解到這個版本的資料夾。
fn extract_entries(
    archive: &mut ZipArchive<File>,
    folder: &Path,
    libraries: &Path,
) -> Result<(), LoaderError> {
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| LoaderError::BadInstaller)?;
        let name = entry
            .name()
            .map_err(|_| LoaderError::BadInstaller)?
            .to_string();
        let target = match name.strip_prefix(MAVEN_FOLDER) {
            Some(relative) => libraries.join(relative),
            None if name.starts_with("data/") => folder.join(&name),
            None => continue,
        };
        if entry.is_dir() {
            continue;
        }
        // 安裝程式來自官方倉庫，但仍然不接受跳出資料夾的路徑。
        if name.contains("..") {
            return Err(LoaderError::BadInstaller);
        }
        let write_error = |source| LoaderError::Write {
            path: target.clone(),
            source,
        };
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(write_error)?;
        }
        let mut output = File::create(&target).map_err(write_error)?;
        io::copy(&mut entry, &mut output).map_err(write_error)?;
    }
    Ok(())
}

/// 處理器參數裡的 `{名稱}` 對應的值。
struct Context {
    values: HashMap<String, String>,
    libraries: PathBuf,
}

impl Context {
    fn new(
        profile: &InstallProfile,
        game_version: &str,
        installer: &Path,
        folder: &Path,
        dirs: &DataDirs,
    ) -> Result<Self, LoaderError> {
        let libraries = dirs.libraries();
        let game_jar = dirs
            .versions()
            .join(game_version)
            .join(format!("{game_version}.jar"));
        let mut values = HashMap::from([
            ("SIDE".to_string(), SIDE.to_string()),
            ("MINECRAFT_JAR".to_string(), path_text(&game_jar)),
            ("MINECRAFT_VERSION".to_string(), game_version.to_string()),
            ("ROOT".to_string(), path_text(dirs.root())),
            ("INSTALLER".to_string(), path_text(installer)),
            ("LIBRARY_DIR".to_string(), path_text(&libraries)),
        ]);
        for (key, value) in &profile.data {
            values.insert(
                key.clone(),
                resolve_data(&value.client, folder, &libraries)?,
            );
        }
        Ok(Self { values, libraries })
    }

    fn expand(&self, argument: &str) -> Result<String, LoaderError> {
        if let Some(coordinate) = bracketed(argument) {
            return library_path(coordinate, &self.libraries);
        }
        if let Some(literal) = quoted(argument) {
            return Ok(literal.to_string());
        }
        let mut text = argument.to_string();
        for (key, value) in &self.values {
            text = text.replace(&format!("{{{key}}}"), value);
        }
        Ok(text)
    }
}

/// `[座標]` 是函式庫路徑，`'文字'` 是原文，`/路徑` 是解出來的資料檔。
fn resolve_data(value: &str, folder: &Path, libraries: &Path) -> Result<String, LoaderError> {
    if let Some(coordinate) = bracketed(value) {
        return library_path(coordinate, libraries);
    }
    if let Some(literal) = quoted(value) {
        return Ok(literal.to_string());
    }
    match value.strip_prefix('/') {
        Some(relative) => Ok(path_text(&folder.join(relative))),
        None => Ok(value.to_string()),
    }
}

fn bracketed(text: &str) -> Option<&str> {
    text.strip_prefix('[')?.strip_suffix(']')
}

fn quoted(text: &str) -> Option<&str> {
    text.strip_prefix('\'')?.strip_suffix('\'')
}

fn library_path(coordinate: &str, libraries: &Path) -> Result<String, LoaderError> {
    let relative = maven_path(coordinate).ok_or_else(|| LoaderError::BadLibrary {
        name: coordinate.to_string(),
    })?;
    Ok(path_text(&libraries.join(relative)))
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

async fn run_processor(
    processor: &Processor,
    context: &Context,
    java: &Path,
    dirs: &DataDirs,
) -> Result<(), LoaderError> {
    let libraries = dirs.libraries();
    let jar = PathBuf::from(library_path(&processor.jar, &libraries)?);
    let main_class = main_class(jar.clone()).await?;
    let mut classpath = vec![path_text(&jar)];
    for library in &processor.classpath {
        classpath.push(library_path(library, &libraries)?);
    }
    let arguments = processor
        .args
        .iter()
        .map(|argument| context.expand(argument))
        .collect::<Result<Vec<_>, _>>()?;

    let output = Command::new(java)
        .arg("-cp")
        .arg(classpath.join(CLASSPATH_SEPARATOR))
        .arg(main_class)
        .args(arguments)
        .current_dir(dirs.root())
        .stdin(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .kill_on_drop(true)
        .output()
        .await
        .map_err(LoaderError::Processor)?;
    if !output.status.success() {
        let text = String::from_utf8_lossy(&output.stderr);
        let detail = text.lines().last().unwrap_or_default().to_string();
        return Err(LoaderError::ProcessorFailed {
            tool: processor.jar.clone(),
            detail,
        });
    }

    for (path, sha1) in &processor.outputs {
        let path = PathBuf::from(context.expand(path)?);
        let expected = context.expand(sha1)?;
        if !has_expected_sha1(&path, &expected).await? {
            return Err(LoaderError::ProcessorFailed {
                tool: processor.jar.clone(),
                detail: "產生的檔案校驗失敗".to_string(),
            });
        }
    }
    Ok(())
}

async fn main_class(jar: PathBuf) -> Result<String, LoaderError> {
    tokio::task::spawn_blocking(move || {
        let mut archive = open_archive(&jar)?;
        let manifest = read_entry(&mut archive, MANIFEST_FILE)?;
        parse_main_class(&String::from_utf8_lossy(&manifest)).ok_or(LoaderError::BadInstaller)
    })
    .await
    .map_err(|_| LoaderError::BadInstaller)?
}

/// 長內容在 manifest 裡會換行，下一行以空白開頭。
fn parse_main_class(manifest: &str) -> Option<String> {
    let joined = manifest.replace("\r\n ", "").replace("\n ", "");
    joined
        .lines()
        .find_map(|line| line.strip_prefix(MAIN_CLASS_KEY))
        .map(|name| name.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> Context {
        Context {
            values: HashMap::from([
                ("ROOT".to_string(), "C:\\data".to_string()),
                ("SIDE".to_string(), "client".to_string()),
            ]),
            libraries: PathBuf::from("C:\\data\\libraries"),
        }
    }

    #[test]
    fn expands_placeholders_inside_longer_arguments() {
        assert_eq!(
            context().expand("{ROOT}/libraries/").unwrap(),
            "C:\\data/libraries/"
        );
        assert_eq!(context().expand("--side").unwrap(), "--side");
    }

    #[test]
    fn expands_coordinates_and_quoted_text() {
        assert_eq!(context().expand("'1.20'").unwrap(), "1.20");
        assert_eq!(
            context().expand("[a.b:c:1@zip]").unwrap(),
            "C:\\data\\libraries\\a\\b\\c\\1\\c-1.zip"
        );
    }

    #[test]
    fn resolves_data_values_by_their_prefix() {
        let folder = Path::new("C:\\v");
        let libraries = Path::new("C:\\l");

        assert_eq!(
            resolve_data("/data/client.lzma", folder, libraries).unwrap(),
            "C:\\v\\data\\client.lzma"
        );
        assert_eq!(resolve_data("'abc'", folder, libraries).unwrap(), "abc");
        assert_eq!(
            resolve_data("[a:b:1:c@txt]", folder, libraries).unwrap(),
            "C:\\l\\a\\b\\1\\b-1-c.txt"
        );
    }

    #[test]
    fn reads_a_main_class_that_wraps_onto_the_next_line() {
        let manifest = "Manifest-Version: 1.0\r\nMain-Class: net.minecraftforge.\r\n installertools.ConsoleTool\r\n";

        assert_eq!(
            parse_main_class(manifest).unwrap(),
            "net.minecraftforge.installertools.ConsoleTool"
        );
    }

    #[test]
    fn skips_processors_for_the_server() {
        let server: Processor =
            serde_json::from_str(r#"{ "sides": ["server"], "jar": "a:b:1", "args": [] }"#).unwrap();
        let both: Processor = serde_json::from_str(r#"{ "jar": "a:b:1", "args": [] }"#).unwrap();

        assert!(!server.runs_on_client());
        assert!(both.runs_on_client());
    }
}
