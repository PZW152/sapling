//! 載入器設定檔：和 Mojang 版本 JSON 同樣的格式，只列出要補充的內容。

use std::collections::HashSet;

use serde::Deserialize;

use crate::versions::details::{
    Arguments, Library, LibraryArtifact, LibraryDownloads, VersionDetails,
};

use super::LoaderError;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderProfile {
    main_class: String,
    #[serde(default)]
    arguments: Arguments,
    libraries: Vec<ProfileLibrary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileLibrary {
    name: String,
    /// Maven 倉庫的網址。
    url: Option<String>,
    sha1: Option<String>,
    size: Option<u64>,
    /// Forge 系列用和 Mojang 相同的寫法。沒有網址的檔案由安裝工具產生。
    downloads: Option<LibraryDownloads>,
}

impl LoaderProfile {
    /// 轉成啟動要用的函式庫清單。每個檔案都必須有網址和 SHA1。
    pub fn libraries(&self) -> Result<Vec<Library>, LoaderError> {
        self.libraries
            .iter()
            .map(ProfileLibrary::to_library)
            .collect()
    }

    /// 把載入器的內容疊到原版上。同一個函式庫以載入器指定的版本為準。
    pub fn apply(self, mut details: VersionDetails, libraries: Vec<Library>) -> VersionDetails {
        let replaced: HashSet<String> = libraries
            .iter()
            .map(|library| library_key(&library.name))
            .collect();
        details
            .libraries
            .retain(|library| !replaced.contains(&library_key(&library.name)));
        // 和官方啟動器一樣，載入器的函式庫排在原版前面。
        let vanilla = std::mem::replace(&mut details.libraries, libraries);
        details.libraries.extend(vanilla);
        details.main_class = self.main_class;
        details.arguments.game.extend(self.arguments.game);
        details.arguments.jvm.extend(self.arguments.jvm);
        details
    }
}

impl ProfileLibrary {
    pub fn to_library(&self) -> Result<Library, LoaderError> {
        if let Some(downloads) = &self.downloads {
            return Ok(Library {
                name: self.name.clone(),
                downloads: downloads.clone(),
                rules: Vec::new(),
            });
        }
        let bad = || LoaderError::BadLibrary {
            name: self.name.clone(),
        };
        let path = maven_path(&self.name).ok_or_else(bad)?;
        let base = self.url.as_deref().ok_or_else(bad)?;
        let sha1 = self.sha1.clone().ok_or_else(bad)?;
        Ok(Library {
            name: self.name.clone(),
            downloads: LibraryDownloads {
                artifact: LibraryArtifact {
                    url: format!("{}/{path}", base.trim_end_matches('/')),
                    path,
                    sha1,
                    // 來源不一定提供大小，它只用於進度，校驗靠 SHA1。
                    size: self.size.unwrap_or(0),
                },
            },
            rules: Vec::new(),
        })
    }
}

/// `group:artifact:version[:classifier][@副檔名]` 對應的相對路徑。
/// 這段文字來自網路，含有 `..` 或路徑符號的一律拒絕。
pub fn maven_path(name: &str) -> Option<String> {
    let (name, extension) = name.split_once('@').unwrap_or((name, "jar"));
    let parts: Vec<&str> = name.split(':').collect();
    let safe = parts
        .iter()
        .all(|part| !part.is_empty() && !part.contains("..") && !part.contains(['/', '\\']));
    if !safe {
        return None;
    }
    let (group, artifact, version, classifier) = match parts[..] {
        [group, artifact, version] => (group, artifact, version, None),
        [group, artifact, version, classifier] => (group, artifact, version, Some(classifier)),
        _ => return None,
    };
    let suffix = classifier.map_or(String::new(), |classifier| format!("-{classifier}"));
    Some(format!(
        "{}/{artifact}/{version}/{artifact}-{version}{suffix}.{extension}",
        group.replace('.', "/")
    ))
}

/// 用來判斷是不是同一個函式庫：不看版本號，但原生庫的分類要分開。
fn library_key(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    match parts[..] {
        [group, artifact, _, classifier] => format!("{group}:{artifact}:{classifier}"),
        [group, artifact, ..] => format!("{group}:{artifact}"),
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vanilla() -> VersionDetails {
        VersionDetails::parse(
            r#"{
                "id": "1.21.8",
                "mainClass": "net.minecraft.client.main.Main",
                "arguments": { "game": ["--username"], "jvm": ["-Dvanilla"] },
                "assetIndex": { "id": "34", "sha1": "a", "size": 2, "totalSize": 3, "url": "u" },
                "downloads": { "client": { "sha1": "c", "size": 4, "url": "u" } },
                "javaVersion": { "component": "j", "majorVersion": 21 },
                "libraries": [
                    { "name": "org.ow2.asm:asm:9.6", "downloads": { "artifact": { "path": "old.jar", "sha1": "o", "size": 1, "url": "u" } } },
                    { "name": "org.lwjgl:lwjgl:3.3.3", "downloads": { "artifact": { "path": "l.jar", "sha1": "l", "size": 1, "url": "u" } } },
                    { "name": "org.lwjgl:lwjgl:3.3.3:natives-windows", "downloads": { "artifact": { "path": "n.jar", "sha1": "n", "size": 1, "url": "u" } } }
                ]
            }"#,
        )
        .unwrap()
    }

    fn profile(libraries: &str) -> LoaderProfile {
        serde_json::from_str(&format!(
            r#"{{
                "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
                "arguments": {{ "game": [], "jvm": ["-DFabricMcEmu= net.minecraft.client.main.Main "] }},
                "libraries": {libraries}
            }}"#
        ))
        .unwrap()
    }

    const FABRIC_LIBRARIES: &str = r#"[
        { "name": "org.ow2.asm:asm:9.10.1", "url": "https://maven.fabricmc.net/", "sha1": "a", "size": 7 },
        { "name": "net.fabricmc:fabric-loader:0.20.1", "url": "https://maven.fabricmc.net/", "sha1": "b" }
    ]"#;

    #[test]
    fn builds_the_download_address_from_the_maven_name() {
        let libraries = profile(FABRIC_LIBRARIES).libraries().unwrap();
        let artifact = &libraries[1].downloads.artifact;

        assert_eq!(
            artifact.path,
            "net/fabricmc/fabric-loader/0.20.1/fabric-loader-0.20.1.jar"
        );
        assert_eq!(
            artifact.url,
            "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.20.1/fabric-loader-0.20.1.jar"
        );
    }

    #[test]
    fn a_library_without_a_checksum_is_refused() {
        let profile = profile(r#"[{ "name": "a:b:1", "url": "https://x/" }]"#);

        assert!(matches!(
            profile.libraries(),
            Err(LoaderError::BadLibrary { .. })
        ));
    }

    #[test]
    fn loader_settings_go_on_top_of_vanilla() {
        let profile = profile(FABRIC_LIBRARIES);
        let libraries = profile.libraries().unwrap();

        let merged = profile.apply(vanilla(), libraries);

        assert_eq!(
            merged.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        assert_eq!(merged.id, "1.21.8");
        assert_eq!(merged.arguments.jvm.len(), 2);
        let names: Vec<_> = merged.libraries.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "org.ow2.asm:asm:9.10.1",
                "net.fabricmc:fabric-loader:0.20.1",
                "org.lwjgl:lwjgl:3.3.3",
                "org.lwjgl:lwjgl:3.3.3:natives-windows",
            ]
        );
    }

    #[test]
    fn classifier_is_part_of_the_path() {
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-windows").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar"
        );
    }

    #[test]
    fn extension_can_be_something_other_than_jar() {
        assert_eq!(
            maven_path("de.oceanlabs.mcp:mcp_config:1.20.1:mappings@txt").unwrap(),
            "de/oceanlabs/mcp/mcp_config/1.20.1/mcp_config-1.20.1-mappings.txt"
        );
    }

    #[test]
    fn forge_style_libraries_keep_their_own_download_details() {
        let profile = profile(
            r#"[{ "name": "net.minecraftforge:forge:26.3-66.1.5:client",
                  "downloads": { "artifact": { "path": "p.jar", "sha1": "s", "size": 0, "url": "" } } }]"#,
        );

        let libraries = profile.libraries().unwrap();

        assert_eq!(libraries[0].downloads.artifact.url, "");
        assert_eq!(libraries[0].downloads.artifact.path, "p.jar");
    }

    #[test]
    fn refuses_names_that_leave_the_libraries_folder() {
        assert_eq!(maven_path("a:b"), None);
        assert_eq!(maven_path("a:..:1"), None);
        assert_eq!(maven_path("a:b/c:1"), None);
        assert_eq!(maven_path("a:b:..\\x"), None);
    }
}
