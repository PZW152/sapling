//! 分段下載大檔案：同時用多條連線下載不同區段，斷線時只補下載缺少的部分。
//!
//! 跨國線路常常限制單條連線的速度，也容易中途斷線，這兩種情況分段都有幫助。
//! 刻意使用多條獨立連線而不是 HTTP/2，丟包時才不會全部一起變慢。

use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use reqwest::StatusCode;
use reqwest::header::RANGE;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::task::JoinSet;

use super::{DownloadError, DownloadTask, RETRY_DELAY};

/// 比這小的檔案直接下載比較快。
pub(super) const MIN_SIZE: u64 = 8 * 1024 * 1024;
const SEGMENTS: u64 = 4;
/// 一段連續這麼多次沒有任何進展才放棄。有進展就重新計算。
const MAX_STALLED_ATTEMPTS: u32 = 6;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Finished,
    /// 伺服器不支援分段，要改用一般下載。
    Unsupported,
}

/// 把檔案分段下載到 `part_path`。完整性由呼叫方校驗。
pub(super) async fn download(
    client: &reqwest::Client,
    task: &DownloadTask,
    part_path: &Path,
) -> Result<Outcome, DownloadError> {
    let io_error = |source| DownloadError::Io {
        path: part_path.to_path_buf(),
        source,
    };
    let file = File::create(part_path).await.map_err(io_error)?;
    file.set_len(task.size).await.map_err(io_error)?;
    drop(file);

    let mut running = JoinSet::new();
    for (start, end) in segments(task.size) {
        let client = client.clone();
        let url = task.url.clone();
        let path = part_path.to_path_buf();
        running.spawn(async move { fetch_segment(&client, url, path, start, end).await });
    }
    while let Some(finished) = running.join_next().await {
        let outcome = finished.map_err(DownloadError::Interrupted)??;
        if outcome == Outcome::Unsupported {
            return Ok(Outcome::Unsupported);
        }
    }
    Ok(Outcome::Finished)
}

/// 把 `size` 位元組平均切開，回傳每段的起點和終點（含）。
fn segments(size: u64) -> Vec<(u64, u64)> {
    let length = size.div_ceil(SEGMENTS).max(1);
    (0..SEGMENTS)
        .map(|index| index * length)
        .take_while(|start| *start < size)
        .map(|start| (start, (start + length - 1).min(size - 1)))
        .collect()
}

/// 下載一段。斷線時從已收到的位置繼續，不會重新開始。
async fn fetch_segment(
    client: &reqwest::Client,
    url: String,
    path: PathBuf,
    start: u64,
    end: u64,
) -> Result<Outcome, DownloadError> {
    let mut position = start;
    let mut stalled = 0;
    loop {
        let before = position;
        match read_range(client, &url, &path, &mut position, end).await {
            Ok(outcome) => return Ok(outcome),
            Err(error @ DownloadError::Io { .. }) => return Err(error),
            Err(error) => {
                stalled = if position > before { 1 } else { stalled + 1 };
                if stalled >= MAX_STALLED_ATTEMPTS {
                    return Err(error);
                }
            }
        }
        tokio::time::sleep(RETRY_DELAY * stalled).await;
    }
}

async fn read_range(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    position: &mut u64,
    end: u64,
) -> Result<Outcome, DownloadError> {
    let network_error = |source| DownloadError::Network {
        url: url.to_string(),
        source,
    };
    let io_error = |source| DownloadError::Io {
        path: path.to_path_buf(),
        source,
    };

    let mut response = client
        .get(url)
        .header(RANGE, format!("bytes={position}-{end}"))
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(network_error)?;
    if response.status() != StatusCode::PARTIAL_CONTENT {
        return Ok(Outcome::Unsupported);
    }

    let mut file = OpenOptions::new()
        .write(true)
        .open(path)
        .await
        .map_err(io_error)?;
    file.seek(SeekFrom::Start(*position))
        .await
        .map_err(io_error)?;
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        file.write_all(&chunk).await.map_err(io_error)?;
        *position += chunk.len() as u64;
    }
    file.flush().await.map_err(io_error)?;

    if *position > end {
        Ok(Outcome::Finished)
    } else {
        Err(DownloadError::Incomplete {
            url: url.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    use super::*;

    fn body() -> Vec<u8> {
        (0..1000u32).map(|value| (value % 251) as u8).collect()
    }

    /// 支援 Range 的本地伺服器。前 `cut_first` 個回應只送一半就斷線。
    async fn serve_ranges(cut_first: usize, ranges: bool) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let counter = requests.clone();
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let counter = counter.clone();
                tokio::spawn(async move {
                    let mut buffer = [0; 1024];
                    let read = socket.read(&mut buffer).await.unwrap();
                    let request = String::from_utf8_lossy(&buffer[..read]).to_lowercase();
                    let number = counter.fetch_add(1, Ordering::SeqCst);
                    let content = body();
                    if !ranges {
                        let head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            content.len()
                        );
                        socket.write_all(head.as_bytes()).await.unwrap();
                        let _ = socket.write_all(&content).await;
                        return;
                    }
                    let range = request
                        .split("range: bytes=")
                        .nth(1)
                        .and_then(|rest| rest.lines().next())
                        .unwrap();
                    let (start, end) = range.trim().split_once('-').unwrap();
                    let start: usize = start.parse().unwrap();
                    let end: usize = end.parse().unwrap();
                    let slice = &content[start..=end];
                    let head = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        slice.len()
                    );
                    socket.write_all(head.as_bytes()).await.unwrap();
                    let sent = if number < cut_first {
                        &slice[..slice.len() / 2]
                    } else {
                        slice
                    };
                    let _ = socket.write_all(sent).await;
                });
            }
        });
        (url, requests)
    }

    fn task(url: String, name: &str) -> (DownloadTask, PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "sapling-segment-{}-{name}.part",
            std::process::id()
        ));
        let task = DownloadTask {
            url,
            path: path.clone(),
            sha1: String::new(),
            size: body().len() as u64,
        };
        (task, path)
    }

    #[test]
    fn segments_cover_the_whole_file_without_gaps() {
        assert_eq!(segments(10), [(0, 2), (3, 5), (6, 8), (9, 9)]);
        assert_eq!(segments(1), [(0, 0)]);
    }

    #[tokio::test]
    async fn joins_all_segments_in_order() {
        let (url, _) = serve_ranges(0, true).await;
        let (task, path) = task(url, "join");

        let outcome = download(&reqwest::Client::new(), &task, &path)
            .await
            .unwrap();

        assert_eq!(outcome, Outcome::Finished);
        assert_eq!(tokio::fs::read(&path).await.unwrap(), body());
    }

    #[tokio::test]
    async fn resumes_a_segment_after_the_connection_drops() {
        let (url, requests) = serve_ranges(2, true).await;
        let (task, path) = task(url, "resume");

        let outcome = download(&reqwest::Client::new(), &task, &path)
            .await
            .unwrap();

        assert_eq!(outcome, Outcome::Finished);
        assert_eq!(tokio::fs::read(&path).await.unwrap(), body());
        assert!(requests.load(Ordering::SeqCst) > 4);
    }

    #[tokio::test]
    async fn reports_a_server_without_range_support() {
        let (url, _) = serve_ranges(0, false).await;
        let (task, path) = task(url, "plain");

        let outcome = download(&reqwest::Client::new(), &task, &path)
            .await
            .unwrap();

        assert_eq!(outcome, Outcome::Unsupported);
    }
}
