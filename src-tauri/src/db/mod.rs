use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;
use std::str::FromStr;

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

    pub async fn get_stats(&self) -> Result<crate::api::LibraryStats, sqlx::Error> {
        let comics = sqlx::query("SELECT COUNT(*) as count FROM comics")
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
        created_at: r.get("created_at"),
    }
}
