//! 版本 JSON 裡的 `rules`：判斷某個項目在本機是否適用。

use std::collections::HashMap;

use serde::Deserialize;

use crate::platform::{MOJANG_ARCH, MOJANG_OS_NAME, MOJANG_OS_VERSION};

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    pub os: Option<OsCondition>,
    /// 例如 `is_demo_user`。Sapling 沒有開啟任何功能，所以要求為 true 的規則都不符合。
    #[serde(default)]
    pub features: HashMap<String, bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsCondition {
    pub name: Option<String>,
    pub arch: Option<String>,
    /// 版本條件，實際資料只有 `^10\\.` 這種開頭比對。
    pub version: Option<String>,
}

impl Rule {
    fn matches_this_machine(&self) -> bool {
        let features_match = self.features.values().all(|wanted| !wanted);
        let os_matches = self.os.as_ref().is_none_or(OsCondition::matches_this_machine);
        features_match && os_matches
    }
}

impl OsCondition {
    fn matches_this_machine(&self) -> bool {
        let name_matches = self.name.as_deref().is_none_or(|name| name == MOJANG_OS_NAME);
        let arch_matches = self.arch.as_deref().is_none_or(|arch| arch == MOJANG_ARCH);
        let version_matches = self.version.as_deref().is_none_or(version_prefix_matches);
        name_matches && arch_matches && version_matches
    }
}

fn version_prefix_matches(pattern: &str) -> bool {
    let prefix = pattern.trim_start_matches('^').replace("\\.", ".");
    MOJANG_OS_VERSION.starts_with(&prefix)
}

/// 沒有規則代表適用；有規則時預設不適用，由最後一條符合的規則決定。
pub fn rules_allow(rules: &[Rule]) -> bool {
    if rules.is_empty() {
        return true;
    }
    rules
        .iter()
        .rfind(|rule| rule.matches_this_machine())
        .is_some_and(|rule| rule.action == RuleAction::Allow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(json: &str) -> Vec<Rule> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn no_rules_means_allowed() {
        assert!(rules_allow(&[]));
    }

    #[test]
    fn allows_windows_only_rule() {
        assert!(rules_allow(&rules(
            r#"[{ "action": "allow", "os": { "name": "windows" } }]"#
        )));
    }

    #[test]
    fn rejects_other_systems() {
        assert!(!rules_allow(&rules(
            r#"[{ "action": "allow", "os": { "name": "osx" } }]"#
        )));
    }

    #[test]
    fn rejects_32_bit_rule() {
        assert!(!rules_allow(&rules(
            r#"[{ "action": "allow", "os": { "arch": "x86" } }]"#
        )));
    }

    #[test]
    fn matches_the_windows_10_version_rule() {
        assert!(rules_allow(&rules(
            r#"[{ "action": "allow", "os": { "name": "windows", "version": "^10\\." } }]"#
        )));
    }

    #[test]
    fn rejects_rules_that_need_a_feature() {
        assert!(!rules_allow(&rules(
            r#"[{ "action": "allow", "features": { "is_demo_user": true } }]"#
        )));
    }

    #[test]
    fn last_matching_rule_wins() {
        assert!(!rules_allow(&rules(
            r#"[{ "action": "allow" }, { "action": "disallow", "os": { "name": "windows" } }]"#
        )));
        assert!(rules_allow(&rules(
            r#"[{ "action": "allow" }, { "action": "disallow", "os": { "name": "osx" } }]"#
        )));
    }
}
