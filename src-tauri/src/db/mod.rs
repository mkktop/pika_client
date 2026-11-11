use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;
use std::str::FromStr;

fn earliest(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if y < x { y } else { x }),
        (x, y) => x.or(y),
    }
}

fn latest(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if y > x { y } else { x }),
        (x, y) => x.or(y),
    }
}

pub struct Database {
    pool: SqlitePool,
}

#[allow(dead_code)]
impl Database {
    pub async fn new(db_path: &str) -> Result<Self, sqlx::Error> {
        // Ensure directory exists
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.init_tables().await?;
        Ok(db)
    }

    async fn init_tables(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS comics (
                comic_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                author TEXT,
                finished INTEGER DEFAULT 0,
                pages_count INTEGER DEFAULT 0,
                category TEXT,
                eps_count INTEGER DEFAULT 0,
                update_time TEXT,
                cover_path TEXT,
                downloaded_at TEXT DEFAULT (datetime('now')),
                created_at TEXT DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS episodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                comic_id TEXT NOT NULL,
                episode_order INTEGER NOT NULL,
                title TEXT NOT NULL,
                downloaded INTEGER DEFAULT 0,
                image_count INTEGER DEFAULT 0,
                downloaded_images INTEGER DEFAULT 0,
                save_path TEXT,
                downloaded_at TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                UNIQUE(comic_id, episode_order)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS download_queue (
                id TEXT PRIMARY KEY,
                comic_id TEXT NOT NULL,
                comic_title TEXT NOT NULL,
                episode_order INTEGER NOT NULL,
                episode_title TEXT NOT NULL,
                status TEXT DEFAULT 'pending',
                total_images INTEGER DEFAULT 0,
                downloaded_images INTEGER DEFAULT 0,
                save_path TEXT,
                error TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // 下载历史与本地文件/书库状态解耦：文件删除或书库清理后记录依然保留。
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS download_history (
                comic_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                episode_orders TEXT NOT NULL DEFAULT '[]',
                first_downloaded_at TEXT,
                last_downloaded_at TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_comics_title ON comics(title)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_comics_category ON comics(category)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_episodes_comic_id ON episodes(comic_id)")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_download_queue_status ON download_queue(status)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn save_comic(&self, comic: &crate::api::ComicDetail) -> Result<(), sqlx::Error> {
        let category = comic.categories.join(",");
        let thumb_path = comic
            .thumb
            .as_ref()
            .map(|t| format!("{}/static/{}", t.file_server, t.path));

        sqlx::query(
            r#"
            INSERT INTO comics (comic_id, title, author, finished, pages_count, category, eps_count, update_time, cover_path)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(comic_id) DO UPDATE SET
                title = excluded.title,
                author = excluded.author,
                finished = excluded.finished,
                pages_count = excluded.pages_count,
                category = excluded.category,
                eps_count = excluded.eps_count,
                update_time = excluded.update_time,
                cover_path = excluded.cover_path
            "#,
        )
        .bind(&comic.id)
        .bind(&comic.title)
        .bind(&comic.author)
        .bind(comic.finished.unwrap_or(false))
        .bind(comic.pages_count.unwrap_or(0))
        .bind(&category)
        .bind(comic.eps_count.unwrap_or(0))
        .bind(&comic.updated_at)
        .bind(&thumb_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_comic(
        &self,
        comic_id: &str,
    ) -> Result<Option<crate::api::LocalComic>, sqlx::Error> {
        let row = sqlx::query("SELECT * FROM comics WHERE comic_id = ?")
            .bind(comic_id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let episodes = self.get_downloaded_episodes(comic_id).await?;
                Ok(Some(crate::api::LocalComic {
                    comic_id: row.get("comic_id"),
                    title: row.get("title"),
                    author: row.get("author"),
                    finished: Some(row.get::<i64, _>("finished") == 1),
                    pages_count: row.get::<Option<i64>, _>("pages_count"),
                    category: row.get("category"),
                    eps_count: row.get::<Option<i64>, _>("eps_count"),
                    update_time: row.get("update_time"),
                    downloaded_episodes: Some(episodes),
                    cover_path: row.get("cover_path"),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_comics(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::api::LocalComic>, sqlx::Error> {
        let rows = sqlx::query("SELECT * FROM comics ORDER BY downloaded_at DESC LIMIT ? OFFSET ?")
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let mut comics = Vec::new();
        for row in rows {
            let comic_id: String = row.get("comic_id");
            let episodes = self.get_downloaded_episodes(&comic_id).await?;
            comics.push(crate::api::LocalComic {
                comic_id: comic_id.clone(),
                title: row.get("title"),
                author: row.get("author"),
                finished: Some(row.get::<i64, _>("finished") == 1),
                pages_count: row.get::<Option<i64>, _>("pages_count"),
                category: row.get("category"),
                eps_count: row.get::<Option<i64>, _>("eps_count"),
                update_time: row.get("update_time"),
                downloaded_episodes: Some(episodes),
                cover_path: row.get("cover_path"),
            });
        }

        Ok(comics)
    }

    pub async fn get_comic_count(&self) -> Result<i64, sqlx::Error> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM comics")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }

    pub async fn save_episode(
        &self,
        comic_id: &str,
        episode_order: i64,
        title: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO episodes (comic_id, episode_order, title)
            VALUES (?, ?, ?)
            ON CONFLICT(comic_id, episode_order) DO UPDATE SET title = excluded.title
            "#,
        )
        .bind(comic_id)
        .bind(episode_order)
        .bind(title)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 已下载章节的磁盘目录（离线阅读用）；目录是否仍在由调用方核对。
    pub async fn get_episode_save_path(
        &self,
        comic_id: &str,
        episode_order: i64,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT save_path FROM episodes WHERE comic_id = ? AND episode_order = ? AND downloaded = 1",
        )
        .bind(comic_id)
        .bind(episode_order)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn mark_episode_downloaded(
        &self,
        comic_id: &str,
        episode_order: i64,
        image_count: i64,
        save_path: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE episodes
            SET downloaded = 1, image_count = ?, downloaded_images = ?, save_path = ?, downloaded_at = datetime('now')
            WHERE comic_id = ? AND episode_order = ?
            "#,
        )
        .bind(image_count)
        .bind(image_count)
        .bind(save_path)
        .bind(comic_id)
        .bind(episode_order)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn is_episode_downloaded(
        &self,
        comic_id: &str,
        episode_order: i64,
    ) -> Result<bool, sqlx::Error> {
        let row = sqlx::query(
            "SELECT downloaded FROM episodes WHERE comic_id = ? AND episode_order = ?",
        )
        .bind(comic_id)
        .bind(episode_order)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row
            .map(|r| r.get::<i64, _>("downloaded") == 1)
            .unwrap_or(false))
    }

    pub async fn get_downloaded_episodes(
        &self,
        comic_id: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT title FROM episodes WHERE comic_id = ? AND downloaded = 1 ORDER BY episode_order",
        )
        .bind(comic_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| r.get("title")).collect())
    }

    pub async fn get_downloaded_episode_orders(
        &self,
        comic_id: &str,
    ) -> Result<Vec<i64>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT episode_order FROM episodes WHERE comic_id = ? AND downloaded = 1 ORDER BY episode_order",
        )
        .bind(comic_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| r.get("episode_order")).collect())
    }

    /// 整章下载完成时记录历史；重复章节幂等，first_downloaded_at 保持首次时间。
    pub async fn upsert_download_history(
        &self,
        comic_id: &str,
        title: &str,
        episode_order: i64,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let existing: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT episode_orders, first_downloaded_at FROM download_history WHERE comic_id = ?",
        )
        .bind(comic_id)
        .fetch_optional(&self.pool)
        .await?;

        let (orders_json, first_at) = match existing {
            Some((raw_orders, first)) => {
                let mut orders: Vec<i64> = serde_json::from_str(&raw_orders).unwrap_or_default();
                if !orders.contains(&episode_order) {
                    orders.push(episode_order);
                    orders.sort_unstable();
                }
                (
                    serde_json::to_string(&orders).unwrap_or_else(|_| "[]".into()),
                    first,
                )
            }
            None => (
                serde_json::to_string(&[episode_order]).unwrap_or_else(|_| "[]".into()),
                None,
            ),
        };
        let first = first_at.unwrap_or_else(|| now.clone());

        sqlx::query(
            "INSERT INTO download_history (comic_id, title, episode_orders, first_downloaded_at, last_downloaded_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(comic_id) DO UPDATE SET
                title = excluded.title,
                episode_orders = excluded.episode_orders,
                last_downloaded_at = excluded.last_downloaded_at",
        )
        .bind(comic_id)
        .bind(title)
        .bind(orders_json)
        .bind(first)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_download_history(
        &self,
    ) -> Result<Vec<crate::api::DownloadHistoryEntry>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT comic_id, title, episode_orders, first_downloaded_at, last_downloaded_at
             FROM download_history ORDER BY last_downloaded_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| crate::api::DownloadHistoryEntry {
                comic_id: r.get("comic_id"),
                title: r.get("title"),
                episode_orders: {
                    let raw: String = r.get("episode_orders");
                    serde_json::from_str(&raw).unwrap_or_default()
                },
                first_downloaded_at: r.get("first_downloaded_at"),
                last_downloaded_at: r.get("last_downloaded_at"),
            })
            .collect())
    }

    pub async fn get_history_episode_orders(
        &self,
        comic_id: &str,
    ) -> Result<Vec<i64>, sqlx::Error> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT episode_orders FROM download_history WHERE comic_id = ?")
                .bind(comic_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row
            .map(|(raw,)| serde_json::from_str(&raw).unwrap_or_default())
            .unwrap_or_default())
    }

    /// 把书库里已完成章节回填进下载历史（升级/首次启用历史时调用，幂等）。
    /// 批量内存合并 + 单事务写入，几万章节也在秒级内完成。
    pub async fn backfill_download_history(&self) -> Result<usize, sqlx::Error> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT e.comic_id, e.episode_order FROM episodes e
             WHERE e.downloaded = 1 ORDER BY e.comic_id, e.episode_order",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut map: std::collections::HashMap<String, crate::api::DownloadHistoryEntry> = self
            .get_download_history()
            .await?
            .into_iter()
            .map(|e| (e.comic_id.clone(), e))
            .collect();

        // 只处理历史里缺的章节；标题从 comics 表取（下载任务写入过），取不到退回漫画 ID。
        let mut backfilled = 0usize;
        for (comic_id, order) in rows {
            if !map.contains_key(&comic_id) {
                let title: Option<(String,)> =
                    sqlx::query_as("SELECT title FROM comics WHERE comic_id = ?")
                        .bind(&comic_id)
                        .fetch_optional(&self.pool)
                        .await?;
                map.insert(
                    comic_id.clone(),
                    crate::api::DownloadHistoryEntry {
                        comic_id: comic_id.clone(),
                        title: title.map(|(t,)| t).unwrap_or_else(|| comic_id.clone()),
                        episode_orders: Vec::new(),
                        first_downloaded_at: None,
                        last_downloaded_at: None,
                    },
                );
            }
            let entry = map.get_mut(&comic_id).expect("just ensured present");
            if entry.episode_orders.binary_search(&order).is_ok() {
                continue;
            }
            entry.episode_orders.push(order);
            backfilled += 1;
        }
        for entry in map.values_mut() {
            entry.episode_orders.sort_unstable();
        }

        self.persist_history_batch(&map).await?;
        Ok(backfilled)
    }

    /// 导入合并：按漫画取章节并集，时间取更早/更晚的极端值，标题保留现有记录。
    /// 批量内存合并 + 单事务写入，几万条也在秒级内完成。
    pub async fn merge_download_history(
        &self,
        entries: &[crate::api::DownloadHistoryEntry],
    ) -> Result<usize, sqlx::Error> {
        if entries.is_empty() {
            return Ok(0);
        }
        let mut map: std::collections::HashMap<String, crate::api::DownloadHistoryEntry> = self
            .get_download_history()
            .await?
            .into_iter()
            .map(|e| (e.comic_id.clone(), e))
            .collect();

        for entry in entries {
            match map.get_mut(&entry.comic_id) {
                Some(current) => {
                    for order in &entry.episode_orders {
                        if !current.episode_orders.contains(order) {
                            current.episode_orders.push(*order);
                        }
                    }
                    current.first_downloaded_at = earliest(
                        current.first_downloaded_at.take(),
                        entry.first_downloaded_at.clone(),
                    );
                    current.last_downloaded_at = latest(
                        current.last_downloaded_at.take(),
                        entry.last_downloaded_at.clone(),
                    );
                }
                None => {
                    let mut incoming = entry.clone();
                    incoming.episode_orders.sort_unstable();
                    map.insert(entry.comic_id.clone(), incoming);
                }
            }
        }

        self.persist_history_batch(&map).await?;
        Ok(entries.len())
    }

    /// 单事务批量写入历史（INSERT 冲突时不覆盖现有标题）。
    async fn persist_history_batch(
        &self,
        map: &std::collections::HashMap<String, crate::api::DownloadHistoryEntry>,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;
        for entry in map.values() {
            sqlx::query(
                "INSERT INTO download_history (comic_id, title, episode_orders, first_downloaded_at, last_downloaded_at)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(comic_id) DO UPDATE SET
                    episode_orders = excluded.episode_orders,
                    first_downloaded_at = excluded.first_downloaded_at,
                    last_downloaded_at = excluded.last_downloaded_at",
            )
            .bind(&entry.comic_id)
            .bind(&entry.title)
            .bind(
                serde_json::to_string(&entry.episode_orders)
                    .unwrap_or_else(|_| "[]".into()),
            )
            .bind(entry.first_downloaded_at.clone().unwrap_or_else(|| now.clone()))
            .bind(entry.last_downloaded_at.clone().unwrap_or_else(|| now.clone()))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_episodes(
        &self,
        comic_id: &str,
    ) -> Result<Vec<(i64, String, bool, i64, i64)>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT episode_order, title, downloaded, image_count, downloaded_images FROM episodes WHERE comic_id = ? ORDER BY episode_order",
        )
        .bind(comic_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| {
                (
                    r.get("episode_order"),
                    r.get("title"),
                    r.get::<i64, _>("downloaded") == 1,
                    r.get("image_count"),
                    r.get("downloaded_images"),
                )
            })
            .collect())
    }

    // Download queue operations
    pub async fn add_download_task(
        &self,
        task: &crate::api::DownloadTask,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO download_queue (id, comic_id, comic_title, episode_order, episode_title, status, total_images, downloaded_images, save_path)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                total_images = excluded.total_images,
                save_path = excluded.save_path,
                updated_at = datetime('now')
            "#,
        )
        .bind(&task.id)
        .bind(&task.comic_id)
        .bind(&task.comic_title)
        .bind(task.episode_id.parse::<i64>().unwrap_or(0))
        .bind(&task.episode_title)
        .bind(&task.status)
        .bind(task.total_images)
        .bind(task.downloaded_images)
        .bind(&task.save_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_download_task(
        &self,
        task_id: &str,
        downloaded: i64,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE download_queue
            SET downloaded_images = ?, status = ?, error = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(downloaded)
        .bind(status)
        .bind(error)
        .bind(task_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn set_download_total(&self, task_id: &str, total: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE download_queue
            SET total_images = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(total)
        .bind(task_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Tasks interrupted by an app restart can never finish; mark them failed.
    pub async fn mark_interrupted_downloads(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE download_queue
            SET status = 'failed', error = '应用重启，任务已中断', updated_at = datetime('now')
            WHERE status IN ('pending', 'downloading')
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_pending_downloads(
        &self,
    ) -> Result<Vec<crate::api::DownloadTask>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT * FROM download_queue WHERE status IN ('pending', 'downloading') ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(row_to_task).collect())
    }

    pub async fn get_all_downloads(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::api::DownloadTask>, sqlx::Error> {
        let rows =
            sqlx::query("SELECT * FROM download_queue ORDER BY created_at DESC LIMIT ?")
                .bind(limit)
                .fetch_all(&self.pool)
                .await?;

        Ok(rows.iter().map(row_to_task).collect())
    }

    // Settings
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get("value")))
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO settings (key, value, updated_at)
            VALUES (?, ?, datetime('now'))
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_setting(&self, key: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 离线回退：本地已下载漫画的详情，拼成与在线接口同形的 JSON。
    /// thumb.localPath 指向本地封面文件，前端经 pikaimg://local/ 渲染。
    /// 本地没有这部漫画时返回 None（调用方继续走在线错误）。
    pub async fn local_comic_json(&self, comic_id: &str) -> Result<Option<serde_json::Value>, sqlx::Error> {
        let row: Option<(String, Option<String>, i64, i64, i64, Option<String>, Option<String>)> =
            sqlx::query_as(
                "SELECT title, author, finished, pages_count, eps_count, category, cover_path
                 FROM comics WHERE comic_id = ?",
            )
            .bind(comic_id)
            .fetch_optional(&self.pool)
            .await?;
        let Some((title, author, finished, pages_count, eps_count, category, cover_path)) = row
        else {
            return Ok(None);
        };
        let categories: Vec<&str> = category.as_deref().filter(|c| !c.trim().is_empty()).map(|c| vec![c]).unwrap_or_default();
        let mut comic = serde_json::json!({
            "_id": comic_id,
            "title": title,
            "author": author.unwrap_or_default(),
            "finished": finished != 0,
            "pagesCount": pages_count,
            "epsCount": eps_count,
            "categories": categories,
        });
        if let Some(cover) = cover_path.filter(|p| !p.trim().is_empty()) {
            comic["thumb"] = serde_json::json!({
                "path": "",
                "fileServer": "",
                "localPath": cover,
            });
        }
        Ok(Some(comic))
    }

    /// 离线回退：本地记录的章节列表（含仅部分下载的漫画已下载的那些章节）。
    pub async fn local_episodes_json(&self, comic_id: &str) -> Result<Vec<serde_json::Value>, sqlx::Error> {
        let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
            "SELECT episode_order, title, downloaded_at
             FROM episodes WHERE comic_id = ? ORDER BY episode_order",
        )
        .bind(comic_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(order, title, downloaded_at)| {
                serde_json::json!({
                    "_id": format!("local-{comic_id}-{order}"),
                    "title": title,
                    "order": order,
                    "updated_at": downloaded_at,
                })
            })
            .collect())
    }

    /// 核对磁盘清理书库：save_path 目录已不存在的完成章节重置为未下载，
    /// 已无任何已下载章节的漫画从书库移除。下载历史（download_history）不受影响。
    pub async fn cleanup_missing_downloads(&self) -> Result<usize, sqlx::Error> {
        let rows: Vec<(String, i64, Option<String>)> = sqlx::query_as(
            "SELECT comic_id, episode_order, save_path FROM episodes WHERE downloaded = 1",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut cleaned = 0usize;
        for (comic_id, order, save_path) in rows {
            let missing = match &save_path {
                Some(path) => {
                    let dir = Path::new(path);
                    // 目录不存在，或目录已被清空，都视为本地文件丢失。
                    !dir.is_dir()
                        || std::fs::read_dir(dir)
                            .map(|mut entries| entries.next().is_none())
                            .unwrap_or(true)
                }
                None => true,
            };
            if missing {
                sqlx::query(
                    "UPDATE episodes
                     SET downloaded = 0, downloaded_images = 0, image_count = 0, save_path = NULL
                     WHERE comic_id = ? AND episode_order = ?",
                )
                .bind(&comic_id)
                .bind(order)
                .execute(&self.pool)
                .await?;
                cleaned += 1;
            }
        }

        sqlx::query(
            "DELETE FROM comics WHERE comic_id NOT IN
             (SELECT DISTINCT comic_id FROM episodes WHERE downloaded = 1)",
        )
        .execute(&self.pool)
        .await?;

        Ok(cleaned)
    }

    /// 单漫画版磁盘核对：进详情页时调用，让“已下载”状态即时反映磁盘真实情况。
    pub async fn cleanup_missing_downloads_for_comic(
        &self,
        comic_id: &str,
    ) -> Result<usize, sqlx::Error> {
        let rows: Vec<(i64, Option<String>)> = sqlx::query_as(
            "SELECT episode_order, save_path FROM episodes
             WHERE comic_id = ? AND downloaded = 1",
        )
        .bind(comic_id)
        .fetch_all(&self.pool)
        .await?;

        let mut cleaned = 0usize;
        for (order, save_path) in rows {
            let missing = match &save_path {
                Some(path) => {
                    let dir = Path::new(path);
                    !dir.is_dir()
                        || std::fs::read_dir(dir)
                            .map(|mut entries| entries.next().is_none())
                            .unwrap_or(true)
                }
                None => true,
            };
            if missing {
                sqlx::query(
                    "UPDATE episodes
                     SET downloaded = 0, downloaded_images = 0, image_count = 0, save_path = NULL
                     WHERE comic_id = ? AND episode_order = ?",
                )
                .bind(comic_id)
                .bind(order)
                .execute(&self.pool)
                .await?;
                cleaned += 1;
            }
        }

        sqlx::query(
            "DELETE FROM comics WHERE comic_id = ? AND comic_id NOT IN
             (SELECT DISTINCT comic_id FROM episodes WHERE downloaded = 1)",
        )
        .bind(comic_id)
        .execute(&self.pool)
        .await?;

        Ok(cleaned)
    }

    /// 清理下载页的任务记录（completed/failed）。
    /// 只动 download_queue 表，不影响本地文件、书库与下载历史。
    pub async fn clear_finished_download_tasks(&self) -> Result<usize, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM download_queue WHERE status IN ('completed', 'failed')")
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() as usize)
    }

    pub async fn get_download_task(&self, id: &str) -> Result<Option<crate::api::DownloadTask>, sqlx::Error> {
        let row = sqlx::query("SELECT * FROM download_queue WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.as_ref().map(row_to_task))
    }

    /// 删除单条任务记录。只动 download_queue 表，不影响本地文件。
    pub async fn delete_download_task(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM download_queue WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// 卡片级下载状态：历史里出现过的漫画 ID + 本地仍有文件的漫画 ID。
    pub async fn get_download_state_sets(&self) -> Result<(Vec<String>, Vec<String>), sqlx::Error> {
        let history = sqlx::query_as::<_, (String,)>("SELECT comic_id FROM download_history")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|(id,)| id)
            .collect();
        let local = sqlx::query_as::<_, (String,)>(
            "SELECT DISTINCT comic_id FROM episodes WHERE downloaded = 1",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(id,)| id)
        .collect();
        Ok((history, local))
    }

    pub async fn get_stats(&self) -> Result<crate::api::LibraryStats, sqlx::Error> {        let comics = sqlx::query("SELECT COUNT(*) as count FROM comics")
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>("count");

        let episodes = sqlx::query("SELECT COUNT(*) as count FROM episodes WHERE downloaded = 1")
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>("count");

        let images =
            sqlx::query("SELECT SUM(image_count) as total FROM episodes WHERE downloaded = 1")
                .fetch_one(&self.pool)
                .await?
                .get::<Option<i64>, _>("total")
                .unwrap_or(0);

        Ok(crate::api::LibraryStats {
            total_comics: comics,
            total_episodes: episodes,
            total_images: images,
            total_size_bytes: 0,
        })
    }
}

fn row_to_task(r: &sqlx::sqlite::SqliteRow) -> crate::api::DownloadTask {
    crate::api::DownloadTask {
        id: r.get("id"),
        comic_id: r.get("comic_id"),
        comic_title: r.get("comic_title"),
        episode_id: r.get::<i64, _>("episode_order").to_string(),
        episode_title: r.get("episode_title"),
        status: r.get("status"),
        total_images: r.get("total_images"),
        downloaded_images: r.get("downloaded_images"),
        save_path: r.get::<Option<String>, _>("save_path").unwrap_or_default(),
        error: r.get("error"),
        created_at: r.get("created_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::Database;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pika-db-test-{}-{}",
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn delete_setting_removes_key_and_is_idempotent() {
        let root = temp_dir("settings-delete");
        let db = Database::new(&root.join("test.db").to_string_lossy()).await.unwrap();
        db.set_setting("login_account", "reader").await.unwrap();
        assert_eq!(
            db.get_setting("login_account").await.unwrap().as_deref(),
            Some("reader")
        );
        db.delete_setting("login_account").await.unwrap();
        assert_eq!(db.get_setting("login_account").await.unwrap(), None);
        // 删除不存在的 key 不应报错（logout 的重复清理路径依赖这一点）。
        db.delete_setting("login_account").await.unwrap();
        std::fs::remove_dir_all(root).ok();
    }

    /// 离线回退：本地 comics/episodes 表能拼出与在线接口同形的详情与章节。
    #[tokio::test]
    async fn local_fallback_builds_online_shaped_comic_and_episodes() {
        let root = temp_dir("offline-fallback");
        let db = Database::new(&root.join("test.db").to_string_lossy()).await.unwrap();
        sqlx::query(
            "INSERT INTO comics (comic_id, title, author, finished, pages_count, eps_count, category, cover_path)
             VALUES ('c1', '测试漫画', '作者甲', 1, 120, 2, '漢化', 'D:/dl/c1/cover.jpg')",
        )
        .execute(&db.pool)
        .await
        .unwrap();
        for (order, title) in [(2i64, "第 2 话"), (1, "第 1 话")] {
            sqlx::query(
                "INSERT INTO episodes (comic_id, episode_order, title, downloaded, image_count)
                 VALUES ('c1', ?, ?, 1, 10)",
            )
            .bind(order)
            .bind(title)
            .execute(&db.pool)
            .await
            .unwrap();
        }

        let comic = db.local_comic_json("c1").await.unwrap().expect("local comic");
        assert_eq!(comic["_id"], "c1");
        assert_eq!(comic["title"], "测试漫画");
        assert_eq!(comic["finished"], true);
        assert_eq!(comic["epsCount"], 2);
        assert_eq!(comic["thumb"]["localPath"], "D:/dl/c1/cover.jpg");
        assert_eq!(comic["categories"].as_array().map(Vec::len), Some(1));

        let episodes = db.local_episodes_json("c1").await.unwrap();
        assert_eq!(episodes.len(), 2);
        assert_eq!(episodes[0]["order"], 1, "章节按 order 升序");
        assert_eq!(episodes[0]["_id"], "local-c1-1");
        assert_eq!(episodes[1]["title"], "第 2 话");

        // 未下载的漫画不回退（调用方继续走在线错误）。
        assert!(db.local_comic_json("missing").await.unwrap().is_none());
        assert!(db.local_episodes_json("missing").await.unwrap().is_empty());
        // 无封面时不产出 thumb 字段。
        sqlx::query("UPDATE comics SET cover_path = NULL WHERE comic_id = 'c1'")
            .execute(&db.pool)
            .await
            .unwrap();
        let comic = db.local_comic_json("c1").await.unwrap().unwrap();
        assert!(comic.get("thumb").is_none());

        std::fs::remove_dir_all(root).ok();
    }

    /// 删掉本地文件后：章节降级为未下载、漫画移出书库，但下载历史保留。
    #[tokio::test]
    async fn cleanup_keeps_history_after_local_files_deleted() {
        let root = temp_dir("history");
        let db = Database::new(&root.join("test.db").to_string_lossy()).await.unwrap();
        let ep_dir = root.join("images");
        std::fs::create_dir_all(&ep_dir).unwrap();
        std::fs::write(ep_dir.join("0001.jpg"), b"fake").unwrap();

        sqlx::query("INSERT INTO comics (comic_id, title) VALUES ('c1', '测试漫画')")
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO episodes (comic_id, episode_order, title, downloaded, image_count, save_path)
             VALUES ('c1', 1, '第 1 话', 1, 10, ?)",
        )
        .bind(ep_dir.to_string_lossy().to_string())
        .execute(&db.pool)
        .await
        .unwrap();
        db.upsert_download_history("c1", "测试漫画", 1).await.unwrap();

        // 文件还在时清理不应改动任何状态。
        let cleaned = db.cleanup_missing_downloads_for_comic("c1").await.unwrap();
        assert_eq!(cleaned, 0);
        assert_eq!(
            db.get_downloaded_episode_orders("c1").await.unwrap(),
            vec![1]
        );

        // 删光本地文件后：章节降级、书库移除，历史保留。
        std::fs::remove_dir_all(&ep_dir).unwrap();
        let cleaned = db.cleanup_missing_downloads_for_comic("c1").await.unwrap();
        assert_eq!(cleaned, 1);
        assert!(db.get_downloaded_episode_orders("c1").await.unwrap().is_empty());
        let comics: Vec<(String,)> =
            sqlx::query_as("SELECT comic_id FROM comics").fetch_all(&db.pool).await.unwrap();
        assert!(comics.is_empty());
        assert_eq!(
            db.get_history_episode_orders("c1").await.unwrap(),
            vec![1],
            "download history must survive local file deletion"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    /// 批量导入：几千部漫画、万级章节在单事务内合并，结果正确且不超时。
    #[tokio::test]
    async fn merge_download_history_handles_thousands_of_entries() {
        let root = temp_dir("bulk");
        let db = Database::new(&root.join("test.db").to_string_lossy()).await.unwrap();

        // 预置 1000 部已有记录（各 10 话）。
        let existing: Vec<crate::api::DownloadHistoryEntry> = (0..1000)
            .map(|i| crate::api::DownloadHistoryEntry {
                comic_id: format!("comic-{i}"),
                title: format!("旧标题 {i}"),
                episode_orders: (1..=10).collect(),
                first_downloaded_at: Some("2020-01-01".into()),
                last_downloaded_at: Some("2020-06-01".into()),
            })
            .collect();
        db.merge_download_history(&existing).await.unwrap();

        // 导入 2000 部：前 1000 部与现有重叠（各多 5 话 + 更晚时间），后 1000 部全新。
        let incoming: Vec<crate::api::DownloadHistoryEntry> = (0..2000)
            .map(|i| crate::api::DownloadHistoryEntry {
                comic_id: format!("comic-{i}"),
                title: format!("新标题 {i}"),
                episode_orders: (6..=20).collect(),
                first_downloaded_at: Some("2021-01-01".into()),
                last_downloaded_at: Some("2021-06-01".into()),
            })
            .collect();
        let merged = db.merge_download_history(&incoming).await.unwrap();
        assert_eq!(merged, 2000);

        let history = db.get_download_history().await.unwrap();
        assert_eq!(history.len(), 2000);
        let first = history
            .iter()
            .find(|e| e.comic_id == "comic-0")
            .expect("entry exists");
        assert_eq!(first.episode_orders, (1..=20).collect::<Vec<_>>(), "union of orders");
        assert_eq!(first.title, "旧标题 0", "existing title wins");
        assert_eq!(
            first.first_downloaded_at.as_deref(),
            Some("2020-01-01"),
            "earliest first time wins"
        );
        assert_eq!(
            first.last_downloaded_at.as_deref(),
            Some("2021-06-01"),
            "latest last time wins"
        );

        std::fs::remove_dir_all(&root).ok();
    }
}
