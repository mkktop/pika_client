use crate::api::{DownloadTask, PicaClient};
use crate::db::Database;
use crate::download::DownloadManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

pub struct AppState {
    pub client: Arc<PicaClient>,
    pub db: Arc<Database>,
    pub download_manager: Arc<DownloadManager>,
    pub download_dir: Arc<RwLock<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub keyword: String,
    pub page: i64,
    pub sort: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub comic_id: String,
    pub comic_title: String,
    pub episode_order: i64,
    pub episode_title: String,
}

#[tauri::command]
pub async fn login(state: State<'_, AppState>, req: LoginRequest) -> Result<String, String> {
    let token = state
        .client
        .login(&req.email, &req.password)
        .await
        .map_err(|e| e.to_string())?;

    // Persist the token so the app stays logged in across restarts.
    state
        .db
        .set_setting("token", &token)
        .await
        .map_err(|e| e.to_string())?;

    Ok(token)
}

#[tauri::command]
pub async fn is_logged_in(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.client.is_logged_in().await)
}

#[tauri::command]
pub async fn punch_in(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state
        .client
        .punch_in()
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_leaderboard(
    state: State<'_, AppState>,
    tt: String,
) -> Result<serde_json::Value, String> {
    state
        .client
        .leaderboard(&tt)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_recommendations(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<serde_json::Value, String> {
    state
        .client
        .recommendations(&comic_id)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_random_comics(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state
        .client
        .random_comics()
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_keywords(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state
        .client
        .keywords()
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_comic_comments(
    state: State<'_, AppState>,
    comic_id: String,
    page: i64,
) -> Result<serde_json::Value, String> {
    state
        .client
        .comic_comments(&comic_id, page)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn like_comic(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<serde_json::Value, String> {
    state
        .client
        .like_comic(&comic_id)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_my_profile(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state
        .client
        .my_profile()
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_user_profile(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<serde_json::Value, String> {
    state
        .client
        .user_profile(&user_id)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_image_quality(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.client.get_image_quality().await)
}

#[tauri::command]
pub async fn set_image_quality(state: State<'_, AppState>, quality: String) -> Result<(), String> {
    state
        .client
        .set_image_quality(&quality)
        .await;

    state
        .db
        .set_setting("image_quality", &quality)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_comics(
    state: State<'_, AppState>,
    req: SearchRequest,
) -> Result<serde_json::Value, String> {
    state
        .client
        .search(&req.keyword, req.page, &req.sort)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_categories(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mut categories = state
        .client
        .categories()
        .await
        .map_err(|e| e.to_string())?;
    categories.retain(|category| {
        category.active != Some(false)
            && category.is_web != Some(true)
            && crate::category_policy::is_supported_category(&category.title)
    });
    serde_json::to_value(categories).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_comics_by_category(
    state: State<'_, AppState>,
    page: i64,
    category: String,
    sort: String,
) -> Result<serde_json::Value, String> {
    if !crate::category_policy::is_supported_category(&category) {
        return Err("该分类不可用".to_string());
    }
    let sort = match sort.as_str() {
        "da" | "ld" | "vd" => sort.as_str(),
        _ => "dd",
    };
    let mut comics = state
        .client
        .comics_by_category(page.max(1), category.trim(), sort)
        .await
        .map_err(|e| e.to_string())?;
    comics.docs.retain(|comic| {
        comic
            .categories
            .iter()
            .all(|title| crate::category_policy::is_supported_category(title))
    });
    serde_json::to_value(comics).map_err(|e| e.to_string())
}

/// 按标签精确过滤（comics?t=，无结果窗口上限），用于搜索页“标签”模式与详情页标签跳转。
#[tauri::command]
pub async fn get_comics_by_tag(
    state: State<'_, AppState>,
    page: i64,
    tag: String,
    sort: String,
) -> Result<serde_json::Value, String> {
    let tag = tag.trim().to_string();
    if tag.is_empty() {
        return Err("标签不能为空".to_string());
    }
    if !crate::category_policy::is_supported_category(&tag) {
        return Err("该标签不可用".to_string());
    }
    let sort = match sort.as_str() {
        "da" | "ld" | "vd" => sort.as_str(),
        _ => "dd",
    };
    let mut comics = state
        .client
        .comics_by_tag(page.max(1), &tag, sort)
        .await
        .map_err(|e| e.to_string())?;
    log::info!(
        "tag filter '{}': total={} pages={} docs={}",
        tag,
        comics.total,
        comics.pages,
        comics.docs.len()
    );
    comics.docs.retain(|comic| {
        comic
            .categories
            .iter()
            .all(|title| crate::category_policy::is_supported_category(title))
    });
    serde_json::to_value(comics).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_latest_comics(
    state: State<'_, AppState>,
    page: i64,
) -> Result<serde_json::Value, String> {
    state
        .client
        .latest_comics(page)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_comic_info(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<serde_json::Value, String> {
    state
        .client
        .comic_info(&comic_id)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_episodes(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<serde_json::Value, String> {
    state
        .client
        .episodes_all(&comic_id)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

/// 单页图片列表：前端先渲染第一批，再自行循环取后续页，避免首屏等全量。
#[tauri::command]
pub async fn get_pictures_page(
    state: State<'_, AppState>,
    comic_id: String,
    ep_order: i64,
    page: i64,
) -> Result<serde_json::Value, String> {
    state
        .client
        .pictures(&comic_id, ep_order, page)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_favourite(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<serde_json::Value, String> {
    state
        .client
        .favourite(&comic_id)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_my_favourites(
    state: State<'_, AppState>,
    page: i64,
    sort: String,
) -> Result<serde_json::Value, String> {
    state
        .client
        .my_favourite(page, &sort)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

/// 已下载到本地的章节 order 列表（用于详情页标记与整本下载跳过已下载）。
/// 查询前先核对磁盘，本地文件已删除的章节即时降级为未下载（下载历史保留）。
#[tauri::command]
pub async fn get_downloaded_episode_orders(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<Vec<i64>, String> {
    let _ = state.db.cleanup_missing_downloads_for_comic(&comic_id).await;
    state
        .db
        .get_downloaded_episode_orders(&comic_id)
        .await
        .map_err(|e| e.to_string())
}

/// 核对磁盘并清理书库中本地已删除的内容，返回被清理的章节数。下载历史不受影响。
#[tauri::command]
pub async fn cleanup_library(state: State<'_, AppState>) -> Result<usize, String> {
    state
        .db
        .cleanup_missing_downloads()
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct DownloadStateSets {
    /// 下载历史里出现过的漫画 ID（含本地文件已删除的）。
    pub history: Vec<String>,
    /// 当前本地仍有已下载章节的漫画 ID。
    pub local: Vec<String>,
}

/// 卡片级下载状态：一次返回历史/本地两个 ID 集合，供列表页批量判断。
#[tauri::command]
pub async fn get_download_state_sets(
    state: State<'_, AppState>,
) -> Result<DownloadStateSets, String> {
    let (history, local) = state
        .db
        .get_download_state_sets()
        .await
        .map_err(|e| e.to_string())?;
    Ok(DownloadStateSets { history, local })
}

/// 清理下载页里已结束（完成/失败）的任务记录，返回清理条数；书库与下载历史不受影响。
#[tauri::command]
pub async fn clear_finished_download_tasks(state: State<'_, AppState>) -> Result<usize, String> {
    state
        .db
        .clear_finished_download_tasks()
        .await
        .map_err(|e| e.to_string())
}

/// 下载历史（含本地文件已删除的记录）。
#[tauri::command]
pub async fn get_download_history(
    state: State<'_, AppState>,
) -> Result<Vec<crate::api::DownloadHistoryEntry>, String> {
    state
        .db
        .get_download_history()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_history_episode_orders(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<Vec<i64>, String> {
    state
        .db
        .get_history_episode_orders(&comic_id)
        .await
        .map_err(|e| e.to_string())
}

/// 导出下载历史到 JSON 文件；用户取消时返回 None。
#[tauri::command]
pub async fn export_download_history(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let entries = state
        .db
        .get_download_history()
        .await
        .map_err(|e| e.to_string())?;
    let export = crate::api::DownloadHistoryExport {
        version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        comics: entries,
    };
    let payload = serde_json::to_vec_pretty(&export).map_err(|e| e.to_string())?;

    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name("pika-download-history.json")
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = file else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|e| e.to_string())?
        .with_extension("json");
    std::fs::write(&path, payload).map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

/// 从 JSON 文件导入下载历史并合并；返回导入的漫画数，取消时返回 None。
#[tauri::command]
pub async fn import_download_history(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let file = tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        app.dialog().file().blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = file else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("读取文件失败：{}", e))?;
    let import: crate::api::DownloadHistoryExport =
        serde_json::from_str(&raw).map_err(|e| format!("文件格式不正确：{}", e))?;
    if import.version != 1 {
        return Err(format!("不支持的记录版本：{}", import.version));
    }
    let comics: Vec<crate::api::DownloadHistoryEntry> = import
        .comics
        .into_iter()
        .filter(|e| !e.comic_id.trim().is_empty() && !e.title.trim().is_empty())
        .collect();
    let count = comics.len();
    state
        .db
        .merge_download_history(&comics)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some(format!("已导入 {} 部漫画的下载记录", count)))
}

#[tauri::command]
pub async fn start_download(
    state: State<'_, AppState>,
    req: DownloadRequest,
) -> Result<String, String> {
    let download_dir = state.download_dir.read().await.clone();
    let task_id = uuid::Uuid::new_v4().to_string();

    let task = DownloadTask {
        id: task_id.clone(),
        comic_id: req.comic_id.clone(),
        comic_title: req.comic_title.clone(),
        episode_id: req.episode_order.to_string(),
        episode_title: req.episode_title.clone(),
        status: "pending".into(),
        total_images: 0,
        downloaded_images: 0,
        save_path: download_dir.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    state
        .db
        .add_download_task(&task)
        .await
        .map_err(|e| e.to_string())?;

    let dm = state.download_manager.clone();
    let client = state.client.clone();
    let db = state.db.clone();

    tokio::spawn(async move {
        dm.download_episode(
            client,
            db,
            task_id,
            req.comic_id,
            req.comic_title,
            req.episode_order,
            req.episode_title,
            download_dir,
        )
        .await;
    });

    Ok(task.id)
}

#[tauri::command]
pub async fn get_download_queue(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state
        .db
        .get_all_downloads(100)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_library(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<serde_json::Value, String> {
    state
        .db
        .get_comics(limit, offset)
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_library_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state
        .db
        .get_stats()
        .await
        .map(|r| serde_json::to_value(r).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_download_dir(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.download_dir.read().await.clone())
}

#[tauri::command]
pub async fn set_download_dir(state: State<'_, AppState>, dir: String) -> Result<(), String> {
    *state.download_dir.write().await = dir.clone();
    state
        .db
        .set_setting("download_dir", &dir)
        .await
        .map_err(|e| e.to_string())
}

/// 弹出系统文件夹选择框；用户取消时返回 None。
#[tauri::command]
pub async fn pick_download_dir(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        app.dialog()
            .file()
            .blocking_pick_folder()
            .map(|path| {
                path.into_path()
                    .map(|p| p.to_string_lossy().to_string())
                    .map_err(|e| e.to_string())
            })
            .transpose()
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_proxy(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.client.get_proxy().await)
}

#[tauri::command]
pub async fn set_proxy(state: State<'_, AppState>, proxy: String) -> Result<(), String> {
    let value = proxy.trim().to_string();
    state
        .client
        .set_proxy(if value.is_empty() {
            None
        } else {
            Some(value.clone())
        })
        .await
        .map_err(|e| e.to_string())?;

    state
        .db
        .set_setting("proxy_url", &value)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    state
        .db
        .set_setting(&key, &value)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    state.db.get_setting(&key).await.map_err(|e| e.to_string())
}
