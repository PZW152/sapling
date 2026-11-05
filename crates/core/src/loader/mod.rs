//! 載入器接口：在原版版本 JSON 之上補充或覆蓋 libraries、mainClass、參數。
//! v1 只有原版；Fabric、Forge、NeoForge 日後照同一接口接入。

use serde::{Deserialize, Serialize};

use crate::versions::details::VersionDetails;

pub trait Loader {
    /// 收到原版的版本資料，回傳實際用於啟動的版本資料。
    fn apply(&self, details: VersionDetails) -> VersionDetails;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoaderKind {
    Vanilla,
}

impl LoaderKind {
    pub fn loader(self) -> Box<dyn Loader + Send + Sync> {
        match self {
            LoaderKind::Vanilla => Box::new(Vanilla),
        }
    }
}

/// 原版：不做任何改動。
pub struct Vanilla;

impl Loader for Vanilla {
    fn apply(&self, details: VersionDetails) -> VersionDetails {
        details
    }
}
