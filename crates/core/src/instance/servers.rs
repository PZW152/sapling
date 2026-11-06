//! 遊戲裡保存的伺服器清單。`servers.dat` 是沒有壓縮的 NBT，這裡只讀需要的欄位。

use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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

/// 伺服器目前的狀況。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub online: u32,
    pub max: u32,
    pub latency_ms: u32,
}

const DEFAULT_PORT: u16 = 25565;
const PING_TIMEOUT: Duration = Duration::from_secs(4);
/// 伺服器回傳的狀態裡可能有很大的圖示，但超過這個大小就不正常了。
const MAX_STATUS_BYTES: usize = 1024 * 1024;
const HANDSHAKE_PACKET: u8 = 0;
const STATUS_STATE: u8 = 1;

#[derive(Deserialize)]
struct StatusResponse {
    players: StatusPlayers,
}

#[derive(Deserialize)]
struct StatusPlayers {
    online: u32,
    max: u32,
}

/// 問伺服器現在幾個人在線。連不上或看不懂都回傳 `None`，介面就不顯示。
pub async fn ping(address: &str) -> Option<ServerStatus> {
    tokio::time::timeout(PING_TIMEOUT, request_status(address))
        .await
        .ok()
        .flatten()
}

fn split_address(address: &str) -> (&str, u16) {
    match address.rsplit_once(':') {
        Some((host, port)) => (host, port.parse().unwrap_or(DEFAULT_PORT)),
        None => (address, DEFAULT_PORT),
    }
}

fn push_varint(out: &mut Vec<u8>, mut value: u32) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn handshake(host: &str, port: u16) -> Vec<u8> {
    let mut body = vec![HANDSHAKE_PACKET];
    // 協定版本填 -1：只問狀態時伺服器不看這個值。
    push_varint(&mut body, u32::MAX);
    push_varint(&mut body, host.len() as u32);
    body.extend(host.as_bytes());
    body.extend(port.to_be_bytes());
    body.push(STATUS_STATE);
    let mut packet = Vec::new();
    push_varint(&mut packet, body.len() as u32);
    packet.extend(body);
    // 緊接著送空的狀態請求。
    packet.extend([1, 0]);
    packet
}

async fn read_varint(stream: &mut TcpStream) -> Option<usize> {
    let mut value = 0_usize;
    for shift in 0..5 {
        let byte = stream.read_u8().await.ok()?;
        value |= usize::from(byte & 0x7F) << (shift * 7);
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

async fn request_status(address: &str) -> Option<ServerStatus> {
    let (host, port) = split_address(address.trim());
    let started = Instant::now();
    let mut stream = TcpStream::connect((host, port)).await.ok()?;
    stream.write_all(&handshake(host, port)).await.ok()?;
    read_varint(&mut stream).await?;
    let latency_ms = started.elapsed().as_millis() as u32;
    read_varint(&mut stream).await?;
    let length = read_varint(&mut stream).await?;
    if length > MAX_STATUS_BYTES {
        return None;
    }
    let mut text = vec![0; length];
    stream.read_exact(&mut text).await.ok()?;
    let status: StatusResponse = serde_json::from_slice(&text).ok()?;
    Some(ServerStatus {
        online: status.players.online,
        max: status.players.max,
        latency_ms,
    })
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

    #[test]
    fn reads_ports_from_addresses() {
        assert_eq!(split_address("a.net"), ("a.net", DEFAULT_PORT));
        assert_eq!(split_address("a.net:25566"), ("a.net", 25566));
    }

    #[test]
    fn writes_varints_like_the_protocol() {
        let mut out = Vec::new();
        push_varint(&mut out, 300);
        assert_eq!(out, [0xAC, 0x02]);

        let mut negative = Vec::new();
        push_varint(&mut negative, u32::MAX);
        assert_eq!(negative, [0xFF, 0xFF, 0xFF, 0xFF, 0x0F]);
    }

    /// 假伺服器照協定回應，確認整個來回。
    #[tokio::test]
    async fn asks_a_server_how_many_players_are_online() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 64];
            let _ = stream.read(&mut request).await.unwrap();
            let json = br#"{"version":{"name":"1.21"},"players":{"max":20,"online":3}}"#;
            let mut body = vec![0];
            push_varint(&mut body, json.len() as u32);
            body.extend(json);
            let mut packet = Vec::new();
            push_varint(&mut packet, body.len() as u32);
            packet.extend(body);
            stream.write_all(&packet).await.unwrap();
        });

        let status = ping(&format!("127.0.0.1:{port}")).await.unwrap();

        assert_eq!((status.online, status.max), (3, 20));
    }

    #[tokio::test]
    async fn an_unreachable_server_is_just_unknown() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        assert_eq!(ping(&format!("127.0.0.1:{port}")).await, None);
    }

    #[tokio::test]
    async fn a_missing_file_is_an_empty_list() {
        assert!(list(Path::new("nowhere")).await.is_empty());
    }
}
