//! 多帳號。憑證管理員裡固定有一份「目前帳號」的 token，登入流程只碰它；
//! 每個帳號另外有自己的一格，切換時把那格複製過來。
//!
//! `accounts.json` 只有玩家名稱和 UUID 這些公開資料，token 不會寫進檔案。

use std::path::PathBuf;

use crate::platform::DataDirs;
use crate::storage::{read_json, write_json};

use super::{AuthError, Profile, token_store};

const ACCOUNTS_FILE: &str = "accounts.json";

fn accounts_path(dirs: &DataDirs) -> PathBuf {
    dirs.root().join(ACCOUNTS_FILE)
}

/// 這台電腦登入過的帳號。
pub async fn list(dirs: &DataDirs) -> Result<Vec<Profile>, AuthError> {
    Ok(read_json(&accounts_path(dirs)).await?.unwrap_or_default())
}

/// 記下成功登入的帳號，並把目前的 token 存進它的格子。
pub async fn remember(dirs: &DataDirs, profile: &Profile) -> Result<(), AuthError> {
    if let Some(token) = token_store::load()? {
        token_store::save_slot(&profile.id, &token)?;
    }
    let updated = with_account(list(dirs).await?, profile.clone());
    write_json(&accounts_path(dirs), &updated).await?;
    Ok(())
}

fn with_account(mut accounts: Vec<Profile>, profile: Profile) -> Vec<Profile> {
    match accounts.iter_mut().find(|account| account.id == profile.id) {
        Some(account) => *account = profile,
        None => accounts.push(profile),
    }
    accounts
}

/// 把這個帳號設成目前帳號，下次恢復登入就會用它。
pub fn switch(id: &str) -> Result<(), AuthError> {
    let token = token_store::load_slot(id)?.ok_or(AuthError::SessionExpired)?;
    token_store::save(&token)?;
    Ok(())
}

/// 從這台電腦移除帳號。
pub async fn forget(dirs: &DataDirs, id: &str) -> Result<(), AuthError> {
    token_store::delete_slot(id)?;
    let mut accounts = list(dirs).await?;
    accounts.retain(|account| account.id != id);
    write_json(&accounts_path(dirs), &accounts).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, name: &str) -> Profile {
        Profile {
            id: id.to_string(),
            name: name.to_string(),
            skin_url: None,
        }
    }

    #[test]
    fn a_new_account_is_added_at_the_end() {
        let accounts = with_account(vec![profile("a", "Alex")], profile("b", "Steve"));

        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[1].name, "Steve");
    }

    #[test]
    fn a_renamed_account_replaces_the_old_entry() {
        let accounts = with_account(vec![profile("a", "Alex")], profile("a", "NewName"));

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].name, "NewName");
    }
}
