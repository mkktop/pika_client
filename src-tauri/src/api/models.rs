use serde::{Deserialize, Serialize};

/// 哔咔接口的字段类型不总是一致：comments 的 page 返回字符串 "1"，
/// comics 的 page 返回数字。统一按“数字或数字字符串”容错解析。
pub fn deserialize_i64_flexible<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| D::Error::custom("expected an integer number")),
        serde_json::Value::String(s) => s
            .trim()
            .parse::<i64>()
            .map_err(|_| D::Error::custom(format!("invalid numeric string: {s:?}"))),
        serde_json::Value::Null => Ok(0),
        other => Err(D::Error::custom(format!(
            "expected a number or numeric string, got {other:?}"
        ))),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: Option<String>,
    pub data: Option<T>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComicList {
    #[serde(default)]
    pub docs: Vec<Comic>,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub limit: i64,
    #[serde(default, deserialize_with = "deserialize_i64_flexible")]
    pub page: i64,
    #[serde(default)]
    pub pages: i64,
}

/// Wrapper for endpoints that return `data: { comics: ... }`
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComicsData {
    pub comics: ComicList,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LeaderboardData {
    pub comics: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoriesData {
    #[serde(default)]
    pub categories: Vec<Category>,
}

/// `data: { comics: [ ... ] }` — array form used by random & recommendation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComicsArrayData {
    #[serde(default)]
    pub comics: Vec<Comic>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeywordsData {
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommentsData {
    pub comments: CommentsPage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommentsPage {
    #[serde(default)]
    pub docs: Vec<CommentItem>,
    #[serde(default)]
    pub total: i64,
    #[serde(default, deserialize_with = "deserialize_i64_flexible")]
    pub page: i64,
    #[serde(default)]
    pub pages: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommentItem {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(default)]
    pub content: String,
    #[serde(rename = "likesCount", default)]
    pub likes_count: i64,
    #[serde(rename = "commentsCount", default)]
    pub comments_count: i64,
    #[serde(rename = "isLiked", default)]
    pub is_liked: Option<bool>,
    #[serde(rename = "_user", default)]
    pub user: Option<CommentUser>,
    #[serde(rename = "created_at", default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommentUser {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub level: i64,
    #[serde(default)]
    pub character: Option<String>,
    #[serde(default)]
    pub slogan: Option<String>,
    #[serde(default)]
    pub avatar: Option<Thumb>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProfileData {
    pub user: ProfileUser,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProfileUser {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub exp: i64,
    #[serde(default)]
    pub level: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(rename = "isPunched", default)]
    pub is_punched: bool,
    #[serde(default)]
    pub slogan: Option<String>,
    #[serde(default)]
    pub character: Option<String>,
    #[serde(default)]
    pub avatar: Option<Thumb>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Comic {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default, rename = "pagesCount")]
    pub pages_count: Option<i64>,
    #[serde(default, rename = "epsCount")]
    pub eps_count: Option<i64>,
    #[serde(default)]
    pub finished: Option<bool>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub thumb: Option<Thumb>,
    #[serde(default, rename = "likesCount")]
    pub likes_count: Option<i64>,
    #[serde(default, rename = "viewsCount")]
    pub views_count: Option<i64>,
    #[serde(default, rename = "isFavourite")]
    pub is_favourite: Option<bool>,
    #[serde(default, rename = "isLiked")]
    pub is_liked: Option<bool>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "chineseTeam")]
    pub chinese_team: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default, rename = "allowDownload")]
    pub allow_download: Option<bool>,
    #[serde(default, rename = "allowComment")]
    pub allow_comment: Option<bool>,
    #[serde(default, rename = "totalLikes")]
    pub total_likes: Option<i64>,
    #[serde(default, rename = "totalViews")]
    pub total_views: Option<i64>,
    #[serde(default, rename = "totalComments")]
    pub total_comments: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Thumb {
    #[serde(default, rename = "originalName")]
    pub original_name: Option<String>,
    #[serde(default)]
    pub path: String,
    #[serde(default, rename = "fileServer")]
    pub file_server: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EpisodeList {
    pub eps: EpisodePage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EpisodePage {
    #[serde(default)]
    pub docs: Vec<Episode>,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub limit: i64,
    #[serde(default, deserialize_with = "deserialize_i64_flexible")]
    pub page: i64,
    #[serde(default)]
    pub pages: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Episode {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    pub order: i64,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PictureList {
    pub pages: PicturePage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PicturePage {
    #[serde(default)]
    pub docs: Vec<Picture>,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub limit: i64,
    #[serde(default, deserialize_with = "deserialize_i64_flexible")]
    pub page: i64,
    #[serde(default)]
    pub pages: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Picture {
    #[serde(rename = "_id", default)]
    pub id: String,
    pub media: Media,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Media {
    #[serde(default, rename = "originalName")]
    pub original_name: Option<String>,
    #[serde(default)]
    pub path: String,
    #[serde(default, rename = "fileServer")]
    pub file_server: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub thumb: Option<Thumb>,
    #[serde(default, rename = "isWeb")]
    pub is_web: Option<bool>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComicDetailResponse {
    pub comic: ComicDetail,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComicDetail {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default, rename = "pagesCount")]
    pub pages_count: Option<i64>,
    #[serde(default, rename = "epsCount")]
    pub eps_count: Option<i64>,
    #[serde(default)]
    pub finished: Option<bool>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub thumb: Option<Thumb>,
    #[serde(default, rename = "likesCount")]
    pub likes_count: Option<i64>,
    #[serde(default, rename = "viewsCount")]
    pub views_count: Option<i64>,
    #[serde(default, rename = "isFavourite")]
    pub is_favourite: Option<bool>,
    #[serde(default, rename = "isLiked")]
    pub is_liked: Option<bool>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "chineseTeam")]
    pub chinese_team: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default, rename = "allowDownload")]
    pub allow_download: Option<bool>,
    #[serde(default, rename = "allowComment")]
    pub allow_comment: Option<bool>,
    #[serde(default, rename = "totalLikes")]
    pub total_likes: Option<i64>,
    #[serde(default, rename = "totalViews")]
    pub total_views: Option<i64>,
    #[serde(default, rename = "totalComments")]
    pub total_comments: Option<i64>,
    #[serde(default)]
    pub creator: Option<Creator>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Creator {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub verified: Option<bool>,
    #[serde(default)]
    pub exp: Option<i64>,
    #[serde(default)]
    pub level: Option<i64>,
    #[serde(default)]
    pub characters: Option<Vec<String>>,
    #[serde(default)]
    pub avatar: Option<Thumb>,
    #[serde(default)]
    pub slogan: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub character: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PunchInResponse {
    pub res: PunchInResult,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PunchInResult {
    #[serde(default)]
    pub status: String,
    #[serde(default, rename = "punchInLastDay")]
    pub punch_in_last_day: Option<String>,
}

// Download tracking (IPC-facing)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadTask {
    pub id: String,
    pub comic_id: String,
    pub comic_title: String,
    pub episode_id: String,
    pub episode_title: String,
    /// pending | downloading | paused | completed | failed
    pub status: String,
    pub total_images: i64,
    pub downloaded_images: i64,
    pub save_path: String,
    pub created_at: String,
}

// Local library types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalComic {
    pub comic_id: String,
    pub title: String,
    pub author: Option<String>,
    pub finished: Option<bool>,
    pub pages_count: Option<i64>,
    pub category: Option<String>,
    pub eps_count: Option<i64>,
    pub update_time: Option<String>,
    pub downloaded_episodes: Option<Vec<String>>,
    pub cover_path: Option<String>,
}

/// 下载历史条目：独立于本地文件状态，删除文件/清理书库后依然保留。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadHistoryEntry {
    pub comic_id: String,
    pub title: String,
    pub episode_orders: Vec<i64>,
    #[serde(default)]
    pub first_downloaded_at: Option<String>,
    #[serde(default)]
    pub last_downloaded_at: Option<String>,
}

/// 导出文件的顶层结构，version 供未来格式演进。
#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadHistoryExport {
    pub version: u32,
    pub exported_at: String,
    pub comics: Vec<DownloadHistoryEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryStats {
    pub total_comics: i64,
    pub total_episodes: i64,
    pub total_images: i64,
    pub total_size_bytes: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// comments 接口实测会把 page 序列化成字符串（"page":"1"），
    /// 而 comics 接口返回数字；两种形式都必须能解析。
    #[test]
    fn deserializes_comments_page_with_string_page_field() {
        let raw = r#"{
            "comments": {
                "docs": [],
                "total": 8,
                "limit": 20,
                "page": "1",
                "pages": 1
            }
        }"#;
        let data: CommentsData = serde_json::from_str(raw).expect("comments payload must parse");
        assert_eq!(data.comments.page, 1);
        assert_eq!(data.comments.pages, 1);
        assert_eq!(data.comments.total, 8);
    }

    #[test]
    fn deserializes_comic_list_with_numeric_page_field() {
        let raw = r#"{
            "comics": {
                "docs": [],
                "total": 1730,
                "limit": 20,
                "page": 2,
                "pages": 87
            }
        }"#;
        let data: ComicsData = serde_json::from_str(raw).expect("comics payload must parse");
        assert_eq!(data.comics.page, 2);
        assert_eq!(data.comics.pages, 87);
    }
}
