//! 單個版本的版本 JSON：遊戲本體、依賴庫、資源索引、Java 需求、啟動參數。

use std::collections::HashMap;

use serde::Deserialize;

use super::VersionsError;
use crate::platform::MOJANG_OS_NAME;

use super::rules::{Rule, rules_allow};
use crate::platform::OTHER_ARCH_NATIVE_SUFFIXES;

/// 舊格式沒有 JVM 參數，這些是新格式裡的基本項目。
const LEGACY_JVM_ARGUMENTS: [&str; 3] = [
    "-Djava.library.path=${natives_directory}",
    "-cp",
    "${classpath}",
];
const LOG4J_CORE: &str = "org.apache.logging.log4j:log4j-core:";
/// 2.15 起修補了 Log4Shell。
const FIRST_SAFE_LOG4J: (u32, u32) = (2, 15);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetails {
    pub id: String,
    pub main_class: String,
    pub asset_index: AssetIndex,
    pub downloads: GameDownloads,
    pub java_version: JavaVersion,
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub arguments: Arguments,
    /// 例如 `release`，啟動時傳給遊戲。
    #[serde(default, rename = "type")]
    pub release_type: String,
    /// 1.13 以前的版本用一整串文字寫遊戲參數。
    #[serde(default, rename = "minecraftArguments")]
    legacy_arguments: Option<String>,
    #[serde(default)]
    pub logging: Option<Logging>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Logging {
    pub client: Option<ClientLogging>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClientLogging {
    /// 含有 `${path}` 的 JVM 參數。
    pub argument: String,
    pub file: LoggingFile,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingFile {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    One(String),
    Many(Vec<String>),
}

impl Argument {
    /// 這個參數是否由版本 JSON 的某個功能開關控制。
    pub fn depends_on_feature(&self, feature: &str) -> bool {
        match self {
            Argument::Plain(_) => false,
            Argument::Conditional { rules, .. } => {
                rules.iter().any(|rule| rule.features.contains_key(feature))
            }
        }
    }

    /// 這個參數在本機實際要傳的內容。
    pub fn values_for_this_machine(&self) -> Vec<&str> {
        match self {
            Argument::Plain(value) => vec![value],
            Argument::Conditional { rules, value } if rules_allow(rules) => match value {
                ArgumentValue::One(value) => vec![value],
                ArgumentValue::Many(values) => values.iter().map(String::as_str).collect(),
            },
            Argument::Conditional { .. } => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    /// 這個索引列出的所有資源檔加起來的大小。
    pub total_size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameDownloads {
    pub client: DownloadFile,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadFile {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: LibraryDownloads,
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// 各系統的原生庫分類，例如 `windows` 對應 `natives-windows`。
    #[serde(default)]
    pub natives: HashMap<String, String>,
    pub extract: Option<ExtractRules>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractRules {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryDownloads {
    /// 舊版的純原生庫沒有這一項，只有 `classifiers`。
    pub artifact: Option<LibraryArtifact>,
    #[serde(default)]
    pub classifiers: HashMap<String, LibraryArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryArtifact {
    /// 相對於 `libraries/` 資料夾的路徑。
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

impl Library {
    /// 這台電腦要解開的原生庫。
    pub fn native_artifact(&self) -> Option<&LibraryArtifact> {
        let classifier = self.natives.get(MOJANG_OS_NAME)?.replace("${arch}", "64");
        self.downloads.classifiers.get(&classifier)
    }

    fn is_needed_on_this_machine(&self) -> bool {
        let for_other_arch = OTHER_ARCH_NATIVE_SUFFIXES
            .iter()
            .any(|suffix| self.name.ends_with(suffix));
        !for_other_arch && rules_allow(&self.rules)
    }
}

impl VersionDetails {
    pub fn parse(json: &str) -> Result<Self, VersionsError> {
        let mut details: Self = serde_json::from_str(json)?;
        details.convert_legacy_arguments();
        Ok(details)
    }

    /// 把舊格式轉成新格式，啟動流程就不用分兩種寫法。
    fn convert_legacy_arguments(&mut self) {
        let Some(text) = self.legacy_arguments.take() else {
            return;
        };
        self.arguments.game = text
            .split_whitespace()
            .map(|argument| Argument::Plain(argument.to_string()))
            .collect();
        self.arguments.jvm = LEGACY_JVM_ARGUMENTS
            .map(|argument| Argument::Plain(argument.to_string()))
            .to_vec();
    }

    /// 這個版本的 log4j 有 Log4Shell 漏洞，必須套用 Mojang 提供的修補設定。
    /// 看函式庫實際的版本，不用遊戲版本號判斷。
    pub fn needs_log4j_fix(&self) -> bool {
        self.libraries.iter().any(|library| {
            library
                .name
                .strip_prefix(LOG4J_CORE)
                .is_some_and(is_vulnerable_log4j)
        })
    }

    /// 本機（Windows x64）實際需要的依賴庫。
    pub fn libraries_for_this_machine(&self) -> Vec<&Library> {
        self.libraries
            .iter()
            .filter(|library| library.is_needed_on_this_machine())
            .collect()
    }
}

fn is_vulnerable_log4j(version: &str) -> bool {
    let mut parts = version
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok());
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    (major, minor) < FIRST_SAFE_LOG4J
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(name: &str, rules: &str) -> String {
        format!(
            r#"{{
                "name": "{name}",
                "downloads": {{ "artifact": {{ "path": "p", "sha1": "s", "size": 1, "url": "u" }} }}
                {rules}
            }}"#
        )
    }

    fn sample() -> String {
        let windows = r#", "rules": [{ "action": "allow", "os": { "name": "windows" } }]"#;
        let linux = r#", "rules": [{ "action": "allow", "os": { "name": "linux" } }]"#;
        let libraries = [
            library("com.mojang:logging:1.0.0", ""),
            library("org.lwjgl:lwjgl:3.3.1:natives-windows", windows),
            library("org.lwjgl:lwjgl:3.3.1:natives-windows-x86", windows),
            library("org.lwjgl:lwjgl:3.3.1:natives-windows-arm64", windows),
            library("org.lwjgl:lwjgl:3.3.1:natives-linux", linux),
        ]
        .join(",");
        format!(
            r#"{{
                "id": "26.3",
                "mainClass": "net.minecraft.client.main.Main",
                "assetIndex": {{ "id": "34", "sha1": "a", "size": 2, "totalSize": 3, "url": "u" }},
                "downloads": {{ "client": {{ "sha1": "c", "size": 4, "url": "u" }} }},
                "javaVersion": {{ "component": "java-runtime-epsilon", "majorVersion": 25 }},
                "libraries": [{libraries}]
            }}"#
        )
    }

    #[test]
    fn reads_the_main_fields() {
        let details = VersionDetails::parse(&sample()).unwrap();
        assert_eq!(details.main_class, "net.minecraft.client.main.Main");
        assert_eq!(details.asset_index.total_size, 3);
        assert_eq!(details.downloads.client.size, 4);
        assert_eq!(details.java_version.component, "java-runtime-epsilon");
        assert_eq!(details.java_version.major_version, 25);
    }

    #[test]
    fn keeps_only_arguments_for_this_machine() {
        let arguments: Vec<Argument> = serde_json::from_str(
            r#"[
                "--username",
                { "rules": [{ "action": "allow", "features": { "is_demo_user": true } }], "value": "--demo" },
                { "rules": [{ "action": "allow", "os": { "name": "windows" } }], "value": ["-A", "-B"] },
                { "rules": [{ "action": "allow", "os": { "name": "osx" } }], "value": "-C" }
            ]"#,
        )
        .unwrap();
        let values: Vec<&str> = arguments
            .iter()
            .flat_map(Argument::values_for_this_machine)
            .collect();
        assert_eq!(values, ["--username", "-A", "-B"]);
    }

    #[test]
    fn treats_only_old_log4j_as_vulnerable() {
        assert!(is_vulnerable_log4j("2.8.1"));
        assert!(is_vulnerable_log4j("2.0-beta9"));
        assert!(!is_vulnerable_log4j("2.17.0"));
        assert!(!is_vulnerable_log4j("2.24.1"));
    }

    #[test]
    fn converts_the_old_argument_format() {
        let details = VersionDetails::parse(
            r#"{
                "id": "1.8.9", "mainClass": "m",
                "minecraftArguments": "--username ${auth_player_name} --version ${version_name}",
                "assetIndex": { "id": "1.8", "sha1": "a", "size": 2, "totalSize": 3, "url": "u" },
                "downloads": { "client": { "sha1": "c", "size": 4, "url": "u" } },
                "javaVersion": { "component": "jre-legacy", "majorVersion": 8 },
                "libraries": [{
                    "name": "org.lwjgl.lwjgl:lwjgl-platform:2.9.4",
                    "natives": { "linux": "natives-linux", "windows": "natives-windows-${arch}" },
                    "extract": { "exclude": ["META-INF/"] },
                    "downloads": { "classifiers": {
                        "natives-windows-64": { "path": "n.jar", "sha1": "n", "size": 1, "url": "u" }
                    } }
                }]
            }"#,
        )
        .unwrap();

        assert_eq!(details.arguments.game.len(), 4);
        assert_eq!(details.arguments.jvm.len(), 3);
        assert_eq!(
            details.libraries[0].native_artifact().unwrap().path,
            "n.jar"
        );
        assert!(details.libraries[0].downloads.artifact.is_none());
    }

    #[test]
    fn keeps_only_libraries_for_windows_x64() {
        let details = VersionDetails::parse(&sample()).unwrap();
        let names: Vec<&str> = details
            .libraries_for_this_machine()
            .iter()
            .map(|library| library.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "com.mojang:logging:1.0.0",
                "org.lwjgl:lwjgl:3.3.1:natives-windows"
            ]
        );
    }
}
