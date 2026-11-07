//! 把 refresh token 和登入快取存進 Windows 憑證管理員，不寫進任何檔案。
//! 單筆憑證上限 2560 bytes，而 token 長度不固定，所以分段存放。

use keyring::{Entry, Error};

const SERVICE: &str = "Sapling Launcher";
const CHUNK_SIZE: usize = 2000;
/// 分段數量上限，只用來防止讀取或刪除時無限循環。
const MAX_CHUNKS: usize = 16;

/// 目前帳號的憑證名稱開頭。
const ACTIVE: &str = "refresh-token";
/// 目前帳號的登入快取。
const SESSION: &str = "session";

fn entry(name: &str, index: usize) -> Result<Entry, Error> {
    Entry::new(SERVICE, &format!("{name}-{index}"))
}

/// 某個帳號自己那一格的名稱。
fn slot(account_id: &str) -> String {
    format!("account-{account_id}")
}

pub fn save(token: &str) -> Result<(), Error> {
    save_to(ACTIVE, token)
}

pub fn load() -> Result<Option<String>, Error> {
    load_from(ACTIVE)
}

pub fn delete() -> Result<(), Error> {
    delete_from(ACTIVE, 0)
}

pub fn save_session(content: &str) -> Result<(), Error> {
    save_to(SESSION, content)
}

pub fn load_session() -> Result<Option<String>, Error> {
    load_from(SESSION)
}

pub fn delete_session() -> Result<(), Error> {
    delete_from(SESSION, 0)
}

pub fn save_slot(account_id: &str, token: &str) -> Result<(), Error> {
    save_to(&slot(account_id), token)
}

pub fn load_slot(account_id: &str) -> Result<Option<String>, Error> {
    load_from(&slot(account_id))
}

pub fn delete_slot(account_id: &str) -> Result<(), Error> {
    delete_from(&slot(account_id), 0)
}

fn save_to(name: &str, token: &str) -> Result<(), Error> {
    let chunks = split(token);
    for (index, chunk) in chunks.iter().enumerate() {
        entry(name, index)?.set_secret(chunk)?;
    }
    delete_from(name, chunks.len())
}

fn load_from(name: &str) -> Result<Option<String>, Error> {
    let mut token = Vec::new();
    for index in 0..MAX_CHUNKS {
        match entry(name, index)?.get_secret() {
            Ok(chunk) => token.extend(chunk),
            Err(Error::NoEntry) => break,
            Err(error) => return Err(error),
        }
    }
    if token.is_empty() {
        return Ok(None);
    }
    String::from_utf8(token)
        .map(Some)
        .map_err(|error| Error::BadEncoding(error.into_bytes()))
}

fn delete_from(name: &str, first: usize) -> Result<(), Error> {
    for index in first..MAX_CHUNKS {
        match entry(name, index)?.delete_credential() {
            Ok(()) => {}
            Err(Error::NoEntry) => break,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn split(token: &str) -> Vec<&[u8]> {
    token.as_bytes().chunks(CHUNK_SIZE).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_tokens_are_split_within_the_credential_limit() {
        let token = "x".repeat(CHUNK_SIZE * 2 + 1);
        let chunks = split(&token);
        assert_eq!(chunks.len(), 3);
        assert!(chunks.iter().all(|chunk| chunk.len() <= CHUNK_SIZE));
        assert_eq!(chunks.concat(), token.as_bytes());
    }
}
