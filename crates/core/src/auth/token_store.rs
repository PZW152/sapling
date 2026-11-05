//! 把 refresh token 存進 Windows 憑證管理員。
//! 單筆憑證上限 2560 bytes，而 token 長度不固定，所以分段存放。

use keyring::{Entry, Error};

const SERVICE: &str = "Sapling Launcher";
const CHUNK_SIZE: usize = 2000;
/// 分段數量上限，只用來防止讀取或刪除時無限循環。
const MAX_CHUNKS: usize = 16;

fn entry(index: usize) -> Result<Entry, Error> {
    Entry::new(SERVICE, &format!("refresh-token-{index}"))
}

pub fn save(token: &str) -> Result<(), Error> {
    let chunks = split(token);
    for (index, chunk) in chunks.iter().enumerate() {
        entry(index)?.set_secret(chunk)?;
    }
    delete_from(chunks.len())
}

pub fn load() -> Result<Option<String>, Error> {
    let mut token = Vec::new();
    for index in 0..MAX_CHUNKS {
        match entry(index)?.get_secret() {
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

pub fn delete() -> Result<(), Error> {
    delete_from(0)
}

fn delete_from(first: usize) -> Result<(), Error> {
    for index in first..MAX_CHUNKS {
        match entry(index)?.delete_credential() {
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
