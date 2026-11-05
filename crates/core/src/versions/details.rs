//! 單個版本的版本 JSON：遊戲本體、依賴庫、資源索引、Java 需求、啟動參數。

use serde::Deserialize;

use super::VersionsError;
use super::rules::{Rule, rules_allow};
use crate::platform::OTHER_ARCH_NATIVE_SUFFIXES;

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
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: LibraryArtifact,
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
    fn is_needed_on_this_machine(&self) -> bool {
        let for_other_arch = OTHER_ARCH_NATIVE_SUFFIXES
            .iter()
            .any(|suffix| self.name.ends_with(suffix));
        !for_other_arch && rules_allow(&self.rules)
    }
}

impl VersionDetails {
    pub fn parse(json: &str) -> Result<Self, VersionsError> {
        Ok(serde_json::from_str(json)?)
    }

    /// 本機（Windows x64）實際需要的依賴庫。
    pub fn libraries_for_this_machine(&self) -> Vec<&Library> {
        self.libraries
            .iter()
            .filter(|library| library.is_needed_on_this_machine())
            .collect()
    }
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
