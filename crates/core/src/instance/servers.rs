//! 遊戲裡保存的伺服器清單。`servers.dat` 是沒有壓縮的 NBT，這裡只讀需要的欄位。

use std::path::Path;

use serde::Serialize;

const SERVERS_FILE: &str = "servers.dat";
const TAG_END: u8 = 0;
const TAG_LIST: u8 = 9;
const TAG_COMPOUND: u8 = 10;
const TAG_STRING: u8 = 8;
/// 檔案被改壞時，巢狀太深也不能讓程式當掉。
const MAX_DEPTH: u8 = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub name: String,
    pub address: String,
}

/// 讀取伺服器清單。沒有檔案或看不懂時回傳空清單。
pub async fn list(game_dir: &Path) -> Vec<Server> {
    let Ok(bytes) = tokio::fs::read(game_dir.join(SERVERS_FILE)).await else {
        return Vec::new();
    };
    parse(&bytes).unwrap_or_default()
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Reader<'_> {
    fn take(&mut self, count: usize) -> Option<&[u8]> {
        let end = self.position.checked_add(count)?;
        let part = self.bytes.get(self.position..end)?;
        self.position = end;
        Some(part)
    }

    fn byte(&mut self) -> Option<u8> {
        self.take(1).map(|part| part[0])
    }

    fn short(&mut self) -> Option<u16> {
        let part: [u8; 2] = self.take(2)?.try_into().ok()?;
        Some(u16::from_be_bytes(part))
    }

    fn int(&mut self) -> Option<usize> {
        let part: [u8; 4] = self.take(4)?.try_into().ok()?;
        usize::try_from(i32::from_be_bytes(part)).ok()
    }

    fn text(&mut self) -> Option<String> {
        let length = usize::from(self.short()?);
        Some(String::from_utf8_lossy(self.take(length)?).to_string())
    }

    /// 跳過一個值。回傳 `None` 代表檔案不完整。
    fn skip(&mut self, tag: u8, depth: u8) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        match tag {
            1 => self.take(1).map(|_| ()),
            2 => self.take(2).map(|_| ()),
            3 | 5 => self.take(4).map(|_| ()),
            4 | 6 => self.take(8).map(|_| ()),
            7 => {
                let count = self.int()?;
                self.take(count).map(|_| ())
            }
            TAG_STRING => self.text().map(|_| ()),
            TAG_LIST => {
                let inner = self.byte()?;
                for _ in 0..self.int()? {
                    self.skip(inner, depth + 1)?;
                }
                Some(())
            }
            TAG_COMPOUND => loop {
                let inner = self.byte()?;
                if inner == TAG_END {
                    return Some(());
                }
                self.text()?;
                self.skip(inner, depth + 1)?;
            },
            11 => {
                let count = self.int()?;
                self.take(count.checked_mul(4)?).map(|_| ())
            }
            12 => {
                let count = self.int()?;
                self.take(count.checked_mul(8)?).map(|_| ())
            }
            _ => None,
        }
    }

    fn server(&mut self) -> Option<Server> {
        let mut name = String::new();
        let mut address = String::new();
        loop {
            let tag = self.byte()?;
            if tag == TAG_END {
                return Some(Server { name, address });
            }
            let key = self.text()?;
            match (tag, key.as_str()) {
                (TAG_STRING, "name") => name = self.text()?,
                (TAG_STRING, "ip") => address = self.text()?,
                _ => self.skip(tag, 1)?,
            }
        }
    }
}

fn parse(bytes: &[u8]) -> Option<Vec<Server>> {
    let mut reader = Reader { bytes, position: 0 };
    if reader.byte()? != TAG_COMPOUND {
        return None;
    }
    reader.text()?;
    loop {
        let tag = reader.byte()?;
        if tag == TAG_END {
            return Some(Vec::new());
        }
        let key = reader.text()?;
        if tag != TAG_LIST || key != "servers" {
            reader.skip(tag, 1)?;
            continue;
        }
        if reader.byte()? != TAG_COMPOUND {
            return Some(Vec::new());
        }
        let mut servers = Vec::new();
        for _ in 0..reader.int()? {
            servers.push(reader.server()?);
        }
        servers.retain(|server| !server.address.is_empty());
        return Some(servers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(out: &mut Vec<u8>, value: &str) {
        out.extend((value.len() as u16).to_be_bytes());
        out.extend(value.as_bytes());
    }

    fn string_tag(out: &mut Vec<u8>, key: &str, value: &str) {
        out.push(TAG_STRING);
        text(out, key);
        text(out, value);
    }

    fn servers_file(servers: &[(&str, &str)]) -> Vec<u8> {
        let mut out = vec![TAG_COMPOUND];
        text(&mut out, "");
        out.push(TAG_LIST);
        text(&mut out, "servers");
        out.push(TAG_COMPOUND);
        out.extend((servers.len() as i32).to_be_bytes());
        for (name, address) in servers {
            string_tag(&mut out, "name", name);
            // 真實檔案裡還有其他欄位，要能跳過。
            out.push(1);
            text(&mut out, "acceptTextures");
            out.push(1);
            string_tag(&mut out, "ip", address);
            out.push(TAG_END);
        }
        out.push(TAG_END);
        out
    }

    #[test]
    fn reads_names_and_addresses() {
        let servers = parse(&servers_file(&[
            ("朋友的", "play.example.com:25565"),
            ("B", "b.net"),
        ]));

        assert_eq!(
            servers.unwrap(),
            [
                Server {
                    name: "朋友的".to_string(),
                    address: "play.example.com:25565".to_string()
                },
                Server {
                    name: "B".to_string(),
                    address: "b.net".to_string()
                },
            ]
        );
    }

    #[test]
    fn broken_files_do_not_panic() {
        let whole = servers_file(&[("A", "a.net")]);
        // 最後一個位元組只是結尾標記，少了清單仍然完整。
        for length in 0..whole.len() - 1 {
            assert_eq!(parse(&whole[..length]), None, "長度 {length}");
        }
        assert_eq!(parse(b"not nbt at all"), None);
    }

    #[tokio::test]
    async fn a_missing_file_is_an_empty_list() {
        assert!(list(Path::new("nowhere")).await.is_empty());
    }
}
