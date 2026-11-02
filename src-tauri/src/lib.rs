mod api;
mod category_policy;
mod commands;
mod db;
mod download;

use commands::*;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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

            let client = Arc::new(api::PicaClient::new());
            if let Some(p) = proxy.filter(|p| !p.trim().is_empty()) {
                tauri::async_runtime::block_on(client.set_proxy(Some(p))).ok();
            }
            tauri::async_runtime::block_on(client.set_token(token));
            if let Ok(Some(q)) = tauri::async_runtime::block_on(db.get_setting("image_quality")) {
                tauri::async_runtime::block_on(client.set_image_quality(&q));
            }

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
            });

            log::info!("Pika Client initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login,
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
            search_comics,
            get_categories,
            get_comics_by_category,
            get_comics_by_tag,
            get_latest_comics,
            get_comic_info,
            get_episodes,
            get_pictures_page,
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
