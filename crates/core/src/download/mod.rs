//! 統一下載器：已存在且 SHA1 正確就跳過，否則下載、校驗，失敗則重試。

mod segmented;

use std::collections::HashMap;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::Duration;

use serde::Serialize;
use sha1::{Digest, Sha1};
use thiserror::Error;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::{JoinError, JoinSet};

/// 同時下載的檔案數量上限的預設值。
pub const DEFAULT_CONCURRENCY: usize = 16;

/// 網路不穩時一個檔案失敗就會中斷整批下載，所以多試幾次。
const MAX_ATTEMPTS: u32 = 5;
/// Modrinth 要求每個程式帶上可以識別的名稱。
const USER_AGENT: &str = concat!(
    "sapling-launcher/",
    env!("CARGO_PKG_VERSION"),
    " (github.com/PZW152/sapling)"
);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// 連線後超過這個時間收不到新資料就換一條連線。不設總時限，大檔案在慢網絡下也能下完。
const READ_TIMEOUT: Duration = Duration::from_secs(15);
const RETRY_DELAY: Duration = if cfg!(test) {
    Duration::from_millis(1)
} else {
    Duration::from_secs(1)
};
const HASH_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("無法準備下載功能")]
    ClientSetup(#[source] reqwest::Error),
    #[error("下載中斷，請檢查網絡連線後再試一次：{url}")]
    Incomplete { url: String },
    #[error("下載失敗，請檢查網絡連線後再試一次：{url}")]
    Network {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("檔案下載後校驗不通過，可能已損壞：{url}")]
    ChecksumMismatch { url: String },
    #[error("無法讀寫檔案，請檢查硬碟空間和權限：{}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("下載工作意外中斷，請再試一次")]
    Interrupted(#[source] JoinError),
}

/// 一個要下載的檔案。
#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub path: PathBuf,
    pub sha1: String,
    /// 檔案大小，用來計算進度。
    pub size: u64,
}

/// 整批下載的進度。檔案完成一個才更新一次。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub finished_files: usize,
    /// 真正下載的檔案數，其餘是已存在而跳過的。
    pub downloaded_files: usize,
    pub total_files: usize,
    pub finished_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadOutcome {
    /// 檔案已存在且 SHA1 正確，沒有下載。
    AlreadyValid,
    Downloaded,
}

pub fn new_client() -> Result<reqwest::Client, DownloadError> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .build()
        .map_err(DownloadError::ClientSetup)
}

/// 下載一小段文字內容（例如清單），不寫入硬碟。
pub async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, DownloadError> {
    let network_error = |source| DownloadError::Network {
        url: url.to_string(),
        source,
    };
    client
        .get(url)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(network_error)?
        .text()
        .await
        .map_err(network_error)
}

/// 並行下載一批檔案。任何一個檔案最終失敗，就取消其餘工作並回傳該錯誤。
///
/// `tasks` 裡不可有重複的路徑，否則兩個工作會同時寫同一個暫存檔。
pub async fn download_all(
    client: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    concurrency: usize,
    mut on_progress: impl FnMut(DownloadProgress),
) -> Result<(), DownloadError> {
    let mut progress = DownloadProgress {
        finished_files: 0,
        downloaded_files: 0,
        total_files: tasks.len(),
        finished_bytes: 0,
        total_bytes: tasks.iter().map(|task| task.size).sum(),
    };
    let mut pending = tasks.into_iter();
    let mut running = JoinSet::new();
    loop {
        while running.len() < concurrency.max(1) {
            let Some(task) = pending.next() else {
                break;
            };
            let client = client.clone();
            running.spawn(async move {
                let outcome = download_file(&client, &task).await?;
                Ok::<_, DownloadError>((task.size, outcome))
            });
        }
        let Some(finished) = running.join_next().await else {
            return Ok(());
        };
        let (size, outcome) = finished.map_err(DownloadError::Interrupted)??;
        progress.finished_files += 1;
        if outcome == DownloadOutcome::Downloaded {
            progress.downloaded_files += 1;
        }
        progress.finished_bytes += size;
        on_progress(progress);
    }
}

/// 正在處理的檔案。兩批下載共用同一個檔案時要排隊，否則會同時寫同一個暫存檔。
static FILE_LOCKS: LazyLock<Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(Mutex::default);

fn file_lock(path: &Path) -> Arc<tokio::sync::Mutex<()>> {
    FILE_LOCKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .entry(path.to_path_buf())
        .or_default()
        .clone()
}

/// 確保 `task.path` 是一個 SHA1 正確的檔案。
pub async fn download_file(
    client: &reqwest::Client,
    task: &DownloadTask,
) -> Result<DownloadOutcome, DownloadError> {
    let lock = file_lock(&task.path);
    let _turn = lock.lock().await;
    if has_expected_sha1(&task.path, &task.sha1).await? {
        return Ok(DownloadOutcome::AlreadyValid);
    }
    if let Some(parent) = task.path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|source| DownloadError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
    }

    let part_path = part_path(&task.path);
    let mut attempt = 1;
    loop {
        let result = download_to_part_file(client, task, &part_path).await;
        match result {
            Ok(()) => {
                fs::rename(&part_path, &task.path)
                    .await
                    .map_err(|source| DownloadError::Io {
                        path: task.path.clone(),
                        source,
                    })?;
                return Ok(DownloadOutcome::Downloaded);
            }
            Err(error) => {
                // 清理失敗不影響結果：下次下載會覆蓋這個暫存檔。
                let _ = fs::remove_file(&part_path).await;
                let retryable = !matches!(error, DownloadError::Io { .. });
                if !retryable || attempt >= MAX_ATTEMPTS {
                    return Err(error);
                }
            }
        }
        tokio::time::sleep(RETRY_DELAY * attempt).await;
        attempt += 1;
    }
}

/// 先寫入 `.part` 暫存檔，校驗通過才改名，這樣中斷時不會留下半個正式檔案。
async fn download_to_part_file(
    client: &reqwest::Client,
    task: &DownloadTask,
    part_path: &Path,
) -> Result<(), DownloadError> {
    if task.size >= segmented::MIN_SIZE
        && segmented::download(client, task, part_path).await? == segmented::Outcome::Finished
    {
        return if has_expected_sha1(part_path, &task.sha1).await? {
            Ok(())
        } else {
            Err(DownloadError::ChecksumMismatch {
                url: task.url.clone(),
            })
        };
    }

    let network_error = |source| DownloadError::Network {
        url: task.url.clone(),
        source,
    };
    let io_error = |source| DownloadError::Io {
        path: part_path.to_path_buf(),
        source,
    };

    let mut response = client
        .get(&task.url)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(network_error)?;

    let mut file = File::create(part_path).await.map_err(io_error)?;
    let mut hasher = Sha1::new();
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(io_error)?;
    }
    file.flush().await.map_err(io_error)?;
    drop(file);

    if to_hex(&hasher.finalize()).eq_ignore_ascii_case(&task.sha1) {
        Ok(())
    } else {
        Err(DownloadError::ChecksumMismatch {
            url: task.url.clone(),
        })
    }
}

pub(crate) async fn has_expected_sha1(path: &Path, expected: &str) -> Result<bool, DownloadError> {
    let io_error = |source| DownloadError::Io {
        path: path.to_path_buf(),
        source,
    };
    let mut file = match File::open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(io_error(error)),
    };
    let mut hasher = Sha1::new();
    let mut buffer = vec![0; HASH_BUFFER_SIZE];
    loop {
        let read = file.read(&mut buffer).await.map_err(io_error)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(to_hex(&hasher.finalize()).eq_ignore_ascii_case(expected))
}

fn part_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".part");
    PathBuf::from(name)
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tokio::net::TcpListener;

    use super::*;

    /// "hello" 的 SHA1。
    const HELLO_SHA1: &str = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";

    /// 依序回覆 `responses` 的本地測試伺服器，回傳網址和請求計數器。
    async fn serve(responses: Vec<(&'static str, &'static str)>) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let counter = requests.clone();
        tokio::spawn(async move {
            for (status, body) in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 1024];
                let _ = socket.read(&mut request).await.unwrap();
                counter.fetch_add(1, Ordering::SeqCst);
                let reply = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(reply.as_bytes()).await.unwrap();
            }
        });
        (url, requests)
    }

    fn empty_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sapling-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn task(url: &str, dir: &Path) -> DownloadTask {
        DownloadTask {
            url: url.to_string(),
            path: dir.join("sub").join("file.txt"),
            sha1: HELLO_SHA1.to_string(),
            size: 5,
        }
    }

    #[tokio::test]
    async fn downloads_a_batch_and_reports_progress() {
        let dir = empty_test_dir("batch");
        let (url, requests) = serve(vec![("200 OK", "hello"); 5]).await;
        let tasks: Vec<DownloadTask> = (0..5)
            .map(|index| DownloadTask {
                path: dir.join(format!("{index}.txt")),
                ..task(&url, &dir)
            })
            .collect();
        let mut updates = Vec::new();

        download_all(&new_client().unwrap(), tasks, 2, |progress| {
            updates.push(progress)
        })
        .await
        .unwrap();

        assert_eq!(requests.load(Ordering::SeqCst), 5);
        assert_eq!(updates.len(), 5);
        assert_eq!(
            updates.last(),
            Some(&DownloadProgress {
                finished_files: 5,
                downloaded_files: 5,
                total_files: 5,
                finished_bytes: 25,
                total_bytes: 25,
            })
        );
    }

    #[tokio::test]
    async fn two_downloads_of_one_file_take_turns() {
        let dir = empty_test_dir("same-file");
        let (url, requests) = serve(vec![("200 OK", "hello"); 2]).await;
        let task = task(&url, &dir);
        let client = new_client().unwrap();

        let (first, second) =
            tokio::join!(download_file(&client, &task), download_file(&client, &task));

        let outcomes = [first.unwrap(), second.unwrap()];
        assert!(outcomes.contains(&DownloadOutcome::Downloaded));
        assert!(outcomes.contains(&DownloadOutcome::AlreadyValid));
        assert_eq!(requests.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn batch_stops_with_the_error_of_a_failed_file() {
        let dir = empty_test_dir("batch-fail");
        let (url, _) = serve(vec![("200 OK", "wrong"); MAX_ATTEMPTS as usize]).await;

        let result = download_all(
            &new_client().unwrap(),
            vec![task(&url, &dir)],
            DEFAULT_CONCURRENCY,
            |_| {},
        )
        .await;

        assert!(matches!(
            result,
            Err(DownloadError::ChecksumMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn empty_batch_finishes_immediately() {
        let result = download_all(&new_client().unwrap(), vec![], 4, |_| {}).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn downloads_a_missing_file_and_creates_folders() {
        let dir = empty_test_dir("missing");
        let (url, _) = serve(vec![("200 OK", "hello")]).await;
        let task = task(&url, &dir);

        let outcome = download_file(&new_client().unwrap(), &task).await.unwrap();

        assert_eq!(outcome, DownloadOutcome::Downloaded);
        assert_eq!(std::fs::read_to_string(&task.path).unwrap(), "hello");
        assert!(!part_path(&task.path).exists());
    }

    #[tokio::test]
    async fn skips_a_file_that_is_already_valid() {
        let dir = empty_test_dir("valid");
        let (url, requests) = serve(vec![]).await;
        let task = task(&url, &dir);
        std::fs::create_dir_all(task.path.parent().unwrap()).unwrap();
        std::fs::write(&task.path, "hello").unwrap();

        let outcome = download_file(&new_client().unwrap(), &task).await.unwrap();

        assert_eq!(outcome, DownloadOutcome::AlreadyValid);
        assert_eq!(requests.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn replaces_a_corrupted_file() {
        let dir = empty_test_dir("corrupted");
        let (url, _) = serve(vec![("200 OK", "hello")]).await;
        let task = task(&url, &dir);
        std::fs::create_dir_all(task.path.parent().unwrap()).unwrap();
        std::fs::write(&task.path, "broken").unwrap();

        let outcome = download_file(&new_client().unwrap(), &task).await.unwrap();

        assert_eq!(outcome, DownloadOutcome::Downloaded);
        assert_eq!(std::fs::read_to_string(&task.path).unwrap(), "hello");
    }

    #[tokio::test]
    async fn retries_after_server_error_and_bad_content() {
        let dir = empty_test_dir("retry");
        let (url, requests) = serve(vec![
            ("500 Internal Server Error", ""),
            ("200 OK", "wrong"),
            ("200 OK", "hello"),
        ])
        .await;
        let task = task(&url, &dir);

        let outcome = download_file(&new_client().unwrap(), &task).await.unwrap();

        assert_eq!(outcome, DownloadOutcome::Downloaded);
        assert_eq!(requests.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn gives_up_after_repeated_bad_downloads_and_leaves_no_file() {
        let dir = empty_test_dir("give-up");
        let (url, requests) = serve(vec![("200 OK", "wrong"); MAX_ATTEMPTS as usize]).await;
        let task = task(&url, &dir);

        let result = download_file(&new_client().unwrap(), &task).await;

        assert!(matches!(
            result,
            Err(DownloadError::ChecksumMismatch { .. })
        ));
        assert_eq!(requests.load(Ordering::SeqCst), MAX_ATTEMPTS as usize);
        assert!(!task.path.exists());
        assert!(!part_path(&task.path).exists());
    }
}
