use serde::{Deserialize, Serialize};

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
    #[serde(default)]
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
pub struct CollectionsData {
    #[serde(default)]
    pub collections: Vec<Collection>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub comics: Vec<Comic>,
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
    #[serde(default)]
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
    #[serde(default)]
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
    #[serde(default)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryStats {
    pub total_comics: i64,
    pub total_episodes: i64,
    pub total_images: i64,
    pub total_size_bytes: i64,
}
