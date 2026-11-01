use super::models::*;
use super::signature;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const API_BASE: &str = "https://picaapi.picacomic.com/";
const API_KEY: &str = "C69BAF41DA5ABD1FFEDC6D2FEA56B";

#[derive(Debug, thiserror::Error)]
pub enum PicaError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: code={code}, message={message}")]
    Api { code: i32, message: String },
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct PicaClient {
    client: Arc<RwLock<Client>>,
    token: Arc<RwLock<Option<String>>>,
    proxy: Arc<RwLock<Option<String>>>,
    image_quality: Arc<RwLock<String>>,
}

fn build_client(proxy: Option<&str>) -> Result<Client, reqwest::Error> {
    let mut builder = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(30));

    if let Some(url) = proxy {
        if !url.trim().is_empty() {
            builder = builder.proxy(reqwest::Proxy::all(url.trim())?);
        }
    }

    builder.build()
}

impl PicaClient {
    pub fn new() -> Self {
        let client = build_client(None).expect("Failed to build HTTP client");
        Self {
            client: Arc::new(RwLock::new(client)),
            token: Arc::new(RwLock::new(None)),
            proxy: Arc::new(RwLock::new(None)),
            image_quality: Arc::new(RwLock::new("original".into())),
        }
    }

    /// Set (or clear) the proxy used for all API and image requests.
    pub async fn set_proxy(&self, proxy: Option<String>) -> Result<(), PicaError> {
        let cleaned = proxy.filter(|p| !p.trim().is_empty());
        let client = build_client(cleaned.as_deref())?;
        *self.client.write().await = client;
        *self.proxy.write().await = cleaned;
        Ok(())
    }

    pub async fn get_proxy(&self) -> Option<String> {
        self.proxy.read().await.clone()
    }

    /// image-quality header: original | lower | lowest
    pub async fn set_image_quality(&self, quality: &str) {
        let q = match quality {
            "lower" | "lowest" => quality.to_string(),
            _ => "original".to_string(),
        };
        *self.image_quality.write().await = q;
    }

    pub async fn get_image_quality(&self) -> String {
        self.image_quality.read().await.clone()
    }

    async fn build_headers(&self, method: &str, path: &str) -> reqwest::header::HeaderMap {
        let timestamp = chrono::Utc::now().timestamp().to_string();
        let nonce = signature::generate_nonce();
        let signature = signature::generate_signature(path, &timestamp, &nonce, method, API_KEY);
        let image_quality = self.image_quality.read().await.clone();

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("api-key", API_KEY.parse().unwrap());
        headers.insert(
            "accept",
            "application/vnd.picacomic.com.v1+json".parse().unwrap(),
        );
        headers.insert("app-channel", "1".parse().unwrap());
        headers.insert("nonce", nonce.parse().unwrap());
        headers.insert("app-version", "2.2.1.3.3.4".parse().unwrap());
        headers.insert("app-uuid", "defaultUuid".parse().unwrap());
        headers.insert("app-platform", "android".parse().unwrap());
        headers.insert("app-build-version", "45".parse().unwrap());
        headers.insert(
            "Content-Type",
            "application/json; charset=UTF-8".parse().unwrap(),
        );
        headers.insert("User-Agent", "okhttp/3.8.1".parse().unwrap());
        headers.insert("image-quality", image_quality.parse().unwrap());
        headers.insert("time", timestamp.parse().unwrap());
        headers.insert("signature", signature.parse().unwrap());

        if let Some(token) = self.token.read().await.as_ref() {
            headers.insert("authorization", token.parse().unwrap());
        }

        headers
    }

    async fn request<T: for<'de> serde::Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, PicaError> {
        let url = format!("{}{}", API_BASE, path);
        let headers = self.build_headers(method.as_str(), path).await;

        let client = self.client.read().await.clone();
        let mut req = client.request(method.clone(), &url).headers(headers);
        if let Some(b) = body {
            req = req.json(&b);
        }

        let resp = req.send().await?;
        let text = resp.text().await?;

        let api_resp: ApiResponse<serde_json::Value> =
            serde_json::from_str(&text).map_err(PicaError::Serialization)?;

        if api_resp.code != 200 {
            return Err(PicaError::Api {
                code: api_resp.code,
                message: api_resp.message.unwrap_or_default(),
            });
        }

        let data = api_resp.data.unwrap_or(serde_json::Value::Null);
        Ok(serde_json::from_value(data)?)
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<String, PicaError> {
        let body = serde_json::json!({
            "email": email,
            "password": password,
        });

        let resp: LoginResponse = self
            .request(reqwest::Method::POST, "auth/sign-in", Some(body))
            .await?;

        let token = resp.token;
        *self.token.write().await = Some(token.clone());
        Ok(token)
    }

    pub async fn is_logged_in(&self) -> bool {
        self.token.read().await.is_some()
    }

    /// Restore a previously persisted token.
    pub async fn set_token(&self, token: Option<String>) {
        *self.token.write().await = token.filter(|t| !t.trim().is_empty());
    }

    pub async fn punch_in(&self) -> Result<PunchInResponse, PicaError> {
        self.request(reqwest::Method::POST, "users/punch-in", None)
            .await
    }

    /// Leaderboard period: H24 | D7 | D30
    pub async fn leaderboard(&self, tt: &str) -> Result<Vec<Comic>, PicaError> {
        let tt = match tt {
            "D7" | "D30" => tt,
            _ => "H24",
        };
        let path = format!("comics/leaderboard?tt={}&ct=VC", tt);
        let resp: LeaderboardData = self.request(reqwest::Method::GET, &path, None).await?;

        // The leaderboard returns either a plain array or a paginated object.
        if let Ok(list) = serde_json::from_value::<Vec<Comic>>(resp.comics.clone()) {
            return Ok(list);
        }
        if let Ok(paged) = serde_json::from_value::<ComicList>(resp.comics) {
            return Ok(paged.docs);
        }
        Ok(vec![])
    }

    pub async fn my_favourite(&self, page: i64, sort: &str) -> Result<ComicList, PicaError> {
        let sort = match sort {
            "da" => "da",
            _ => "dd",
        };
        let path = format!("users/favourite?s={}&page={}", sort, page);
        let resp: ComicsData = self.request(reqwest::Method::GET, &path, None).await?;
        Ok(resp.comics)
    }

    pub async fn favourite(&self, comic_id: &str) -> Result<serde_json::Value, PicaError> {
        let path = format!("comics/{}/favourite", comic_id);
        self.request(reqwest::Method::POST, &path, None).await
    }

    pub async fn like_comic(&self, comic_id: &str) -> Result<serde_json::Value, PicaError> {
        let path = format!("comics/{}/like", comic_id);
        self.request(reqwest::Method::POST, &path, None).await
    }

    /// Related comics on the detail page.
    pub async fn recommendations(&self, comic_id: &str) -> Result<Vec<Comic>, PicaError> {
        let path = format!("comics/{}/recommendation", comic_id);
        let resp: ComicsArrayData = self.request(reqwest::Method::GET, &path, None).await?;
        Ok(resp.comics)
    }

    /// Random comics for the explore page.
    pub async fn random_comics(&self) -> Result<Vec<Comic>, PicaError> {
        let resp: ComicsArrayData =
            self.request(reqwest::Method::GET, "comics/random", None).await?;
        Ok(resp.comics)
    }

    /// Popular search keywords.
    pub async fn keywords(&self) -> Result<Vec<String>, PicaError> {
        let resp: KeywordsData = self.request(reqwest::Method::GET, "keywords", None).await?;
        Ok(resp.keywords)
    }

    /// Comic comments, paginated.
    pub async fn comic_comments(
        &self,
        comic_id: &str,
        page: i64,
    ) -> Result<CommentsPage, PicaError> {
        let path = format!("comics/{}/comments?page={}", comic_id, page);
        let resp: CommentsData = self.request(reqwest::Method::GET, &path, None).await?;
        Ok(resp.comments)
    }

    /// Current logged-in user's profile.
    pub async fn my_profile(&self) -> Result<ProfileUser, PicaError> {
        let resp: ProfileData = self.request(reqwest::Method::GET, "users/profile", None).await?;
        Ok(resp.user)
    }

    /// Another user's profile.
    pub async fn user_profile(&self, user_id: &str) -> Result<ProfileUser, PicaError> {
        let path = format!("users/{}/profile", user_id);
        let resp: ProfileData = self.request(reqwest::Method::GET, &path, None).await?;
        Ok(resp.user)
    }

    pub async fn episodes(&self, comic_id: &str, page: i64) -> Result<EpisodePage, PicaError> {
        let path = format!("comics/{}/eps?page={}", comic_id, page);
        let resp: EpisodeList = self.request(reqwest::Method::GET, &path, None).await?;
        Ok(resp.eps)
    }

    pub async fn episodes_all(&self, comic_id: &str) -> Result<Vec<Episode>, PicaError> {
        let first_page = self.episodes(comic_id, 1).await?;
        let total_pages = first_page.pages.max(1);
        let mut all_eps = first_page.docs;

        for page in 2..=total_pages {
            let eps = self.episodes(comic_id, page).await?;
            all_eps.extend(eps.docs);
        }

        all_eps.sort_by_key(|e| e.order);
        Ok(all_eps)
    }

    pub async fn pictures(
        &self,
        comic_id: &str,
        ep_order: i64,
        page: i64,
    ) -> Result<PicturePage, PicaError> {
        let path = format!(
            "comics/{}/order/{}/pages?page={}",
            comic_id, ep_order, page
        );
        let resp: PictureList = self.request(reqwest::Method::GET, &path, None).await?;
        Ok(resp.pages)
    }

    pub async fn pictures_all(
        &self,
        comic_id: &str,
        ep_order: i64,
    ) -> Result<Vec<Picture>, PicaError> {
        let first_page = self.pictures(comic_id, ep_order, 1).await?;
        let total_pages = first_page.pages.max(1);
        let mut all_pics = first_page.docs;

        for page in 2..=total_pages {
            let pics = self.pictures(comic_id, ep_order, page).await?;
            all_pics.extend(pics.docs);
        }

        Ok(all_pics)
    }

    pub async fn comic_info(&self, comic_id: &str) -> Result<ComicDetail, PicaError> {
        let path = format!("comics/{}", comic_id);
        let resp: ComicDetailResponse = self.request(reqwest::Method::GET, &path, None).await?;
        Ok(resp.comic)
    }

    pub async fn categories(&self) -> Result<Vec<Category>, PicaError> {
        let resp: CategoriesData = self.request(reqwest::Method::GET, "categories", None).await?;
        Ok(resp.categories)
    }

    pub async fn search(
        &self,
        keyword: &str,
        page: i64,
        sort: &str,
    ) -> Result<ComicList, PicaError> {
        let path = format!("comics/advanced-search?page={}", page);
        let body = serde_json::json!({
            "keyword": keyword,
            "sort": sort,
        });
        let resp: ComicsData = self
            .request(reqwest::Method::POST, &path, Some(body))
            .await?;
        Ok(resp.comics)
    }

    pub async fn comics_by_category(
        &self,
        page: i64,
        category: &str,
        sort: &str,
    ) -> Result<ComicList, PicaError> {
        let encoded = urlencoding::encode(category);
        let path = format!("comics?page={}&c={}&s={}", page, encoded, sort);
        let resp: ComicsData = self.request(reqwest::Method::GET, &path, None).await?;
        Ok(resp.comics)
    }

    pub async fn latest_comics(&self, page: i64) -> Result<ComicList, PicaError> {
        let path = format!("comics?page={}&s=dd", page);
        let resp: ComicsData = self.request(reqwest::Method::GET, &path, None).await?;
        Ok(resp.comics)
    }

    /// Download one image through the same client (so the proxy applies).
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>, PicaError> {
        let client = self.client.read().await.clone();
        let resp = client.get(url).send().await?.error_for_status()?;
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }
}
