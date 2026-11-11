use crate::api::PicaClient;
use crate::db::Database;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// 瞬时网络错误自动重试：首次失败后最多再试 MAX_RETRIES 次，指数退避。
const MAX_RETRIES: u32 = 3;
const RETRY_BACKOFF_BASE: Duration = Duration::from_millis(1000);

/// 网络抖动重试：失败后按 base * 2^attempt 退避重试，最后一次仍失败则返回原错误。
/// 只在下载管理器层使用，浏览等其它请求路径不受影响。
async fn with_retry<T, E, Fut, F>(mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    for attempt in 0..=MAX_RETRIES {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt < MAX_RETRIES => {
                let delay = RETRY_BACKOFF_BASE * (1u32 << attempt);
                log::warn!(
                    "Download request failed (attempt {}/{}), retrying in {:?}: {}",
                    attempt + 1,
                    MAX_RETRIES + 1,
                    delay,
                    e
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("loop returns on every branch")
}

pub struct DownloadManager {
    semaphore: Arc<Semaphore>,
}

impl DownloadManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn download_episode(
        &self,
        client: Arc<PicaClient>,
        db: Arc<Database>,
        task_id: String,
        comic_id: String,
        comic_title: String,
        episode_order: i64,
        episode_title: String,
        base_dir: String,
    ) {
        let _permit = self.semaphore.acquire().await.unwrap();

        let _ = db
            .update_download_task(&task_id, 0, "downloading", None)
            .await;

        // Make sure the comic & episode exist in the local DB before downloading.
        if let Ok(info) = client.comic_info(&comic_id).await {
            let _ = db.save_comic(&info).await;
        }
        let _ = db
            .save_episode(&comic_id, episode_order, &episode_title)
            .await;

        // Fetch all picture metadata (失败自动重试，吸收瞬时网络错误)
        let pictures = match with_retry(|| client.pictures_all(&comic_id, episode_order)).await {
            Ok(p) => p,
            Err(e) => {
                let _ = db
                    .update_download_task(&task_id, 0, "failed", Some(&e.to_string()))
                    .await;
                return;
            }
        };

        let total = pictures.len() as i64;
        let _ = db.set_download_total(&task_id, total).await;
        let _ = db
            .update_download_task(&task_id, 0, "downloading", None)
            .await;

        // Create the target directory
        let safe_title = sanitize_filename(&comic_title);
        let safe_ep = sanitize_filename(&episode_title);
        let dir_path = Path::new(&base_dir).join(&safe_title).join(&safe_ep);
        if let Err(e) = std::fs::create_dir_all(&dir_path) {
            let _ = db
                .update_download_task(
                    &task_id,
                    0,
                    "failed",
                    Some(&format!("Failed to create dir: {}", e)),
                )
                .await;
            return;
        }

        // Download images with a concurrency limit
        let mut downloaded = 0i64;
        let sem = Arc::new(Semaphore::new(8));
        let mut handles = Vec::new();

        for (idx, pic) in pictures.iter().enumerate() {
            let url = format!("{}/static/{}", pic.media.file_server, pic.media.path);
            let client = client.clone();
            let dir_path = dir_path.clone();
            let sem = sem.clone();

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let ext = url
                    .split('?')
                    .next()
                    .and_then(|u| u.rsplit('.').next())
                    .filter(|e| e.len() <= 5 && e.chars().all(|c| c.is_ascii_alphanumeric()))
                    .unwrap_or("jpg");
                let filename = format!("{:04}.{}", idx + 1, ext);
                let path = dir_path.join(&filename);

                if path.exists() {
                    return Ok(());
                }

                let data = with_retry(|| client.download_image(&url)).await?;
                std::fs::write(&path, &data)?;
                Ok::<_, anyhow::Error>(())
            });

            handles.push(handle);
        }

        // Wait for all downloads
        for handle in handles {
            match handle.await {
                Ok(Ok(())) => {
                    downloaded += 1;
                    let _ = db
                        .update_download_task(&task_id, downloaded, "downloading", None)
                        .await;
                }
                Ok(Err(e)) => {
                    log::error!("Download error: {}", e);
                }
                Err(e) => {
                    log::error!("Task join error: {}", e);
                }
            }
        }

        let save_path = dir_path.to_string_lossy().to_string();

        if downloaded == total && total > 0 {
            let _ = db
                .mark_episode_downloaded(&comic_id, episode_order, total, &save_path)
                .await;
            // 无论本地文件日后是否删除，下载历史都保留这一章的记录。
            let _ = db
                .upsert_download_history(&comic_id, &comic_title, episode_order)
                .await;
            let _ = db
                .update_download_task(&task_id, downloaded, "completed", None)
                .await;
            log::info!(
                "Downloaded {} images for {} - {}",
                downloaded,
                comic_title,
                episode_title
            );
        } else {
            let _ = db
                .update_download_task(
                    &task_id,
                    downloaded,
                    "failed",
                    Some(&format!("{}/{} images downloaded", downloaded, total)),
                )
                .await;
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    let replacements = [
        ("/", "／"),
        ("\\", "＼"),
        ("?", "？"),
        ("|", "︱"),
        ("\"", "＂"),
        ("*", "＊"),
        ("<", "＜"),
        (">", "＞"),
        (":", "："),
        ("-", "－"),
    ];
    let mut result = name.to_string();
    for (old, new) in replacements {
        result = result.replace(old, new);
    }
    result = result.replace(' ', "");
    result.chars().take(255).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 前 N 次失败、之后成功：with_retry 应自动重试直至成功。
    #[tokio::test]
    async fn retries_until_success() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let result: Result<&str, String> = with_retry(move || {
            let counter = counter.clone();
            async move {
                let n = counter.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err("transient".to_string())
                } else {
                    Ok("done")
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), "done");
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    /// 超过最大重试次数仍失败：返回最后一次的错误，共尝试 MAX_RETRIES + 1 次。
    #[tokio::test]
    async fn gives_up_after_max_retries() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let result: Result<(), String> = with_retry(move || {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>("permanent".to_string())
            }
        })
        .await;

        assert_eq!(result.unwrap_err(), "permanent");
        assert_eq!(calls.load(Ordering::SeqCst), (MAX_RETRIES + 1) as usize);
    }
}
