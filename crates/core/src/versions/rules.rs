//! 版本 JSON 裡的 `rules`：判斷某個項目在本機是否適用。

use serde::Deserialize;

use crate::platform::{MOJANG_ARCH, MOJANG_OS_NAME};

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    pub os: Option<OsCondition>,
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
}

impl Rule {
    fn matches_this_machine(&self) -> bool {
        let Some(os) = &self.os else {
            return true;
        };
        let name_matches = os.name.as_deref().is_none_or(|name| name == MOJANG_OS_NAME);
        let arch_matches = os.arch.as_deref().is_none_or(|arch| arch == MOJANG_ARCH);
        name_matches && arch_matches
    }
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
    fn last_matching_rule_wins() {
        assert!(!rules_allow(&rules(
            r#"[{ "action": "allow" }, { "action": "disallow", "os": { "name": "windows" } }]"#
        )));
        assert!(rules_allow(&rules(
            r#"[{ "action": "allow" }, { "action": "disallow", "os": { "name": "osx" } }]"#
        )));
    }
}
