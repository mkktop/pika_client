mod api;
mod category_policy;
mod commands;
mod credentials;
mod db;
mod download;
mod image_protocol;
mod updates;

use commands::*;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;
use updates::*;

/// pikaimg:// 自定义协议：webview 图片统一走 Rust 侧，附带磁盘缓存。
/// - pikaimg://remote/<编码后的远程URL>：缓存于 app_cache_dir/imgcache（按 URL 哈希分桶），
///   未命中时用共享客户端下载（代理/画质设置生效），超过 8MB 的响应不落盘。
/// - pikaimg://local/<编码后的绝对路径>：仅允许读当前下载目录内的文件（离线阅读）。
///
/// 使用异步协议处理器：命中缓存时同步读取（亚毫秒级）；未命中时把下载丢进运行时并发执行，
/// 不阻塞 WebView2 的 UI 线程。下载并发上限 8，避免首屏几十张封面同时打爆代理。
fn handle_pikaimg(
    app: tauri::AppHandle,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    fn respond(status: u16, content_type: &str, body: Vec<u8>) -> tauri::http::Response<Vec<u8>> {
        tauri::http::Response::builder()
            .status(status)
            .header("Content-Type", content_type)
            .body(body)
            .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
    }
    fn content_type_of(path: &str) -> &'static str {
        let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "avif" => "image/avif",
            _ => "image/jpeg",
        }
    }

    let uri = request.uri().to_string();
    log::debug!("pikaimg request: {uri}");
    let (kind, decoded) = match image_protocol::parse_image_uri(&uri) {
        Ok(route) => route,
        Err(error) => {
            responder.respond(respond(400, "text/plain", error.as_bytes().to_vec()));
            return;
        }
    };

    match kind.as_str() {
        "local" => {
            let dir = app
                .try_state::<AppState>()
                .map(|state| tauri::async_runtime::block_on(async { state.download_dir.read().await.clone() }))
                .unwrap_or_default();
            let base = std::fs::canonicalize(&dir).unwrap_or_else(|_| std::path::PathBuf::from(&dir));
            let path = std::path::PathBuf::from(&decoded);
            let real = match std::fs::canonicalize(&path) {
                Ok(p) => p,
                Err(_) => {
                    responder.respond(respond(404, "text/plain", b"not found".to_vec()));
                    return;
                }
            };
            if !real.starts_with(&base) {
                responder.respond(respond(403, "text/plain", b"forbidden".to_vec()));
                return;
            }
            match std::fs::read(&real) {
                Ok(bytes) => responder.respond(respond(200, content_type_of(&decoded), bytes)),
                Err(_) => responder.respond(respond(404, "text/plain", b"read failed".to_vec())),
            }
        }
        "remote" => {
            if !decoded.starts_with("http://") && !decoded.starts_with("https://") {
                responder.respond(respond(400, "text/plain", b"bad url".to_vec()));
                return;
            }
            let content_type = content_type_of(&decoded);
            let config = app
                .try_state::<AppState>()
                .map(|state| {
                    tauri::async_runtime::block_on(async { state.image_cache.read().await.clone() })
                })
                .unwrap_or(commands::ImageCacheConfig {
                    enabled: false,
                    limit_bytes: 0,
                });
            let cache_root = app
                .path()
                .app_cache_dir()
                .unwrap_or_default()
                .join("imgcache");
            let hex = hex_prefix(&decoded);
            let cache_file = cache_root.join(&hex[..2]).join(&hex);
            // 开关关闭时既不读缓存也不写缓存，只做代理转发。
            if config.enabled {
                if let Ok(bytes) = std::fs::read(&cache_file) {
                    responder.respond(respond(200, content_type, bytes));
                    return;
                }
            }
            // 未命中：离开 UI 线程，运行时并发下载后异步应答。
            tauri::async_runtime::spawn(async move {
                let Ok(_permit) = image_download_limit().acquire().await else {
                    responder.respond(respond(503, "text/plain", b"busy".to_vec()));
                    return;
                };
                let Some(state) = app.try_state::<AppState>() else {
                    responder.respond(respond(500, "text/plain", b"state unavailable".to_vec()));
                    return;
                };
                let client = state.client.clone();
                let used_counter = state.image_cache_used.clone();
                match client.download_image(&decoded).await {
                    Ok(bytes) => {
                        if config.enabled && bytes.len() <= 8 * 1024 * 1024 {
                            let dir_ready = cache_file
                                .parent()
                                .map(|parent| std::fs::create_dir_all(parent).is_ok())
                                .unwrap_or(false);
                            let written = dir_ready && std::fs::write(&cache_file, &bytes).is_ok();
                            if written {
                                let size = bytes.len() as u64;
                                let used = used_counter
                                    .fetch_add(size, std::sync::atomic::Ordering::Relaxed)
                                    + size;
                                // 超限时按最久未修改的文件淘汰到上限的 90%。
                                if config.limit_bytes > 0 && used > config.limit_bytes {
                                    evict_cache(&cache_root, &used_counter, config.limit_bytes / 10 * 9);
                                }
                            }
                        }
                        responder.respond(respond(200, content_type, bytes));
                    }
                    Err(e) => {
                        log::warn!("pikaimg fetch failed for {decoded}: {e}");
                        responder.respond(respond(502, "text/plain", b"fetch failed".to_vec()));
                    }
                }
            });
        }
        _ => responder.respond(respond(400, "text/plain", b"unknown kind".to_vec())),
    }
}

fn hex_prefix(value: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn image_download_limit() -> &'static tokio::sync::Semaphore {
    static LIMIT: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
    LIMIT.get_or_init(|| tokio::sync::Semaphore::new(8))
}

/// 递归统计目录占用字节数（封面缓存启动扫描与清空统计用）。
pub(crate) fn dir_used_bytes(dir: &std::path::Path) -> u64 {
    fn walk(dir: &std::path::Path, total: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                walk(&entry.path(), total);
            } else {
                *total += meta.len();
            }
        }
    }
    let mut total = 0;
    walk(dir, &mut total);
    total
}

/// 按最久未修改优先删除缓存文件，直到占用回落到 target 以下。
fn evict_cache(root: &std::path::Path, used: &std::sync::atomic::AtomicU64, target: u64) {
    use std::sync::atomic::Ordering;
    fn collect(
        dir: &std::path::Path,
        files: &mut Vec<(std::time::SystemTime, u64, std::path::PathBuf)>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                collect(&entry.path(), files);
            } else {
                files.push((
                    meta.modified().unwrap_or(std::time::UNIX_EPOCH),
                    meta.len(),
                    entry.path(),
                ));
            }
        }
    }
    let mut files = Vec::new();
    collect(root, &mut files);
    files.sort_by_key(|(mtime, _, _)| *mtime);
    let mut current = used.load(Ordering::Relaxed);
    for (_, len, path) in files {
        if current <= target {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            used.fetch_sub(len.min(current), Ordering::Relaxed);
            current = current.saturating_sub(len);
        }
    }
    log::info!("imgcache evicted down to {current} bytes (target {target})");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(UpdateManager::default())
        .register_asynchronous_uri_scheme_protocol("pikaimg", |ctx, request, responder| {
            handle_pikaimg(ctx.app_handle().clone(), request, responder)
        })
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            std::fs::create_dir_all(&app_dir).ok();

            let db_path = app_dir.join("pika.db").to_string_lossy().to_string();
            let default_download_dir = app_dir
                .join("downloads")
                .to_string_lossy()
                .to_string();

            // Initialize database and load persisted settings before serving commands.
            let (db, download_dir, proxy, token) = tauri::async_runtime::block_on(async {
                let db = db::Database::new(&db_path).await?;
                let _ = db.mark_interrupted_downloads().await;
                let download_dir = db
                    .get_setting("download_dir")
                    .await
                    .ok()
                    .flatten()
                    .filter(|d| !d.trim().is_empty())
                    .unwrap_or(default_download_dir);
                let proxy = db.get_setting("proxy_url").await.ok().flatten();
                let token = db.get_setting("token").await.ok().flatten();
                Ok::<_, sqlx::Error>((Arc::new(db), download_dir, proxy, token))
            })?;

            let client = Arc::new(api::PicaClient::new(db.clone()));
            if let Some(p) = proxy.filter(|p| !p.trim().is_empty()) {
                tauri::async_runtime::block_on(client.set_proxy(Some(p))).ok();
            }
            tauri::async_runtime::block_on(client.set_token(token));
            // 恢复本机保存的登录凭据（系统凭据库），token 过期时自动重登。
            if let Ok(Some(account)) = tauri::async_runtime::block_on(db.get_setting("login_account")) {
                if let Ok(Some(password)) = credentials::load(&account) {
                    tauri::async_runtime::block_on(
                        client.set_credentials(Some((account, password))),
                    );
                }
            }
            if let Ok(Some(q)) = tauri::async_runtime::block_on(db.get_setting("image_quality")) {
                tauri::async_runtime::block_on(client.set_image_quality(&q));
            }

            // 封面缓存默认关闭；容量上限默认 256MB。
            let image_cache_enabled = tauri::async_runtime::block_on(db.get_setting("image_cache_enabled"))
                .ok()
                .flatten()
                .map(|v| v == "1")
                .unwrap_or(false);
            let image_cache_limit_mb: u64 =
                tauri::async_runtime::block_on(db.get_setting("image_cache_limit_mb"))
                    .ok()
                    .flatten()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(256)
                    .clamp(16, 8192);
            let image_cache_used = app
                .path()
                .app_cache_dir()
                .map(|p| dir_used_bytes(&p.join("imgcache")))
                .unwrap_or(0);

            let download_manager = Arc::new(download::DownloadManager::new(3));

            // 先把已完成章节回填进下载历史（含文件已被删除的），再核对磁盘清理书库。
            {
                let db = db.clone();
                tauri::async_runtime::spawn(async move {
                    match db.backfill_download_history().await {
                        Ok(n) if n > 0 => {
                            log::info!("Backfilled {n} episode records into download history")
                        }
                        Ok(_) => {}
                        Err(e) => log::warn!("Download history backfill failed: {e}"),
                    }
                    match db.cleanup_missing_downloads().await {
                        Ok(n) if n > 0 => {
                            log::info!("Cleaned {n} episodes whose local files are gone")
                        }
                        Ok(_) => {}
                        Err(e) => log::warn!("Library cleanup failed: {e}"),
                    }
                });
            }

            app.manage(AppState {
                client,
                db,
                download_manager,
                download_dir: Arc::new(RwLock::new(download_dir)),
                image_cache: Arc::new(RwLock::new(ImageCacheConfig {
                    enabled: image_cache_enabled,
                    limit_bytes: image_cache_limit_mb * 1024 * 1024,
                })),
                image_cache_used: Arc::new(std::sync::atomic::AtomicU64::new(image_cache_used)),
            });

            log::info!("Pika Client initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            check_for_updates,
            download_update,
            install_update,
            open_update_release,
            login,
            logout,
            get_auth_status,
            is_logged_in,
            punch_in,
            get_leaderboard,
            get_recommendations,
            get_random_comics,
            get_keywords,
            get_comic_comments,
            like_comic,
            get_my_profile,
            get_user_profile,
            get_image_quality,
            set_image_quality,
            get_image_cache_status,
            set_image_cache_settings,
            clear_image_cache,
            search_comics,
            get_categories,
            get_comics_by_category,
            get_comics_by_tag,
            get_latest_comics,
            get_comic_info,
            get_episodes,
            get_pictures_page,
            get_local_pictures,
            toggle_favourite,
            get_my_favourites,
            start_download,
            get_downloaded_episode_orders,
            get_download_history,
            get_download_state_sets,
            clear_finished_download_tasks,
            get_history_episode_orders,
            cleanup_library,
            export_download_history,
            import_download_history,
            get_download_queue,
            get_library,
            get_library_stats,
            get_download_dir,
            set_download_dir,
            pick_download_dir,
            get_proxy,
            set_proxy,
            save_setting,
            get_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
