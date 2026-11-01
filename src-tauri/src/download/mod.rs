use crate::api::PicaClient;
use crate::db::Database;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

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

        // Fetch all picture metadata
        let pictures = match client.pictures_all(&comic_id, episode_order).await {
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

                let data = client.download_image(&url).await?;
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
