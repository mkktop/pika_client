use super::models::*;
use super::signature;
use crate::db::Database;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

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
    /// 会话失效且无法自动重登。前缀由前端统一识别：清除登录态并引导重新登录。
    #[error("AUTH_REQUIRED:登录已过期，请重新登录")]
    AuthRequired,
}

pub struct PicaClient {
    client: Arc<RwLock<Client>>,
    token: Arc<RwLock<Option<String>>>,
    proxy: Arc<RwLock<Option<String>>>,
    image_quality: Arc<RwLock<String>>,
    /// 本机保存的登录凭据（内存副本，落盘在系统凭据库），token 过期时自动重登用。
    credentials: Arc<RwLock<Option<(String, String)>>>,
    /// 重登单飞锁：并发 401 只触发一次重新登录。
    relogin_lock: Arc<Mutex<()>>,
    /// 重登成功后把新 token 持久化回 settings 表。
    db: Arc<Database>,
    api_base: String,
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
    pub fn new(db: Arc<Database>) -> Self {
        let client = build_client(None).expect("Failed to build HTTP client");
        Self {
            client: Arc::new(RwLock::new(client)),
            token: Arc::new(RwLock::new(None)),
            proxy: Arc::new(RwLock::new(None)),
            image_quality: Arc::new(RwLock::new("original".into())),
            credentials: Arc::new(RwLock::new(None)),
            relogin_lock: Arc::new(Mutex::new(())),
            db,
            api_base: API_BASE.to_string(),
        }
    }

    #[cfg(test)]
    fn with_api_base(db: Arc<Database>, api_base: &str) -> Self {
        let mut client = Self {
            api_base: api_base.to_string(),
            ..Self::new(db)
        };
        // 本地 mock 强制直连，避免环境变量里的系统代理劫持 127.0.0.1 请求。
        let direct = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(10))
            .no_proxy()
            .build()
            .expect("Failed to build test HTTP client");
        futures::executor::block_on(async {
            *client.client.write().await = direct;
        });
        client
    }

    /// 设置（或清除）自动重登用的本机凭据。仅驻内存，落盘在系统凭据库。
    pub async fn set_credentials(&self, credentials: Option<(String, String)>) {
        *self.credentials.write().await = credentials;
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

    /// 单次请求，不做任何会话恢复。401 的识别兼容两种形态：
    /// 业务码 401（JSON body）和 HTTP 401（body 非 JSON 的网关响应）。
    async fn request_raw<T: for<'de> serde::Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, PicaError> {
        let url = format!("{}{}", self.api_base, path);
        let headers = self.build_headers(method.as_str(), path).await;

        let client = self.client.read().await.clone();
        let mut req = client.request(method.clone(), &url).headers(headers);
        if let Some(b) = body {
            req = req.json(&b);
        }

        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;

        let api_resp: ApiResponse<serde_json::Value> =
            match serde_json::from_str(&text) {
                Ok(parsed) => parsed,
                Err(e) => {
                    if status == reqwest::StatusCode::UNAUTHORIZED {
                        return Err(PicaError::Api {
                            code: 401,
                            message: "unauthorized".into(),
                        });
                    }
                    return Err(PicaError::Serialization(e));
                }
            };

        if api_resp.code != 200 {
            return Err(PicaError::Api {
                code: api_resp.code,
                message: api_resp.message.unwrap_or_default(),
            });
        }

        let data = api_resp.data.unwrap_or(serde_json::Value::Null);
        Ok(serde_json::from_value(data)?)
    }

    /// token 过期时用本机保存的凭据重新登录。返回 true 表示已拿到新 token、可安全重试。
    /// 拿单飞锁后先检查 token 是否已被其他请求刷新，避免排队者重复打登录接口。
    async fn relogin_after_expiry(&self, stale_token: Option<String>) -> Result<bool, PicaError> {
        let _guard = self.relogin_lock.lock().await;
        let current = self.token.read().await.clone();
        if current.is_some() && current != stale_token {
            return Ok(true);
        }
        let Some((account, password)) = self.credentials.read().await.clone() else {
            return Ok(false);
        };
        let token = self.login_raw(&account, &password).await?;
        self.set_token(Some(token.clone())).await;
        // 持久化失败只影响下次启动的会话恢复，当前会话照常工作。
        let _ = self.db.set_setting("token", &token).await;
        Ok(true)
    }

    /// 统一请求入口：401 时先自动重登（有保存凭据）再重试一次。
    async fn request<T: for<'de> serde::Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, PicaError> {
        let token_before = self.token.read().await.clone();
        match self.request_raw(method.clone(), path, body.clone()).await {
            Err(PicaError::Api { code: 401, .. }) => match self.relogin_after_expiry(token_before).await
            {
                Ok(true) => self.request_raw(method, path, body).await,
                Ok(false) => Err(PicaError::AuthRequired),
                Err(relogin_error) => Err(match relogin_error {
                    // 重登时密码被改/账号失效：统一按会话过期处理。
                    PicaError::Api { code: 401, .. } => PicaError::AuthRequired,
                    other => other,
                }),
            },
            other => other,
        }
    }

    async fn login_raw(&self, email: &str, password: &str) -> Result<String, PicaError> {
        let body = serde_json::json!({
            "email": email,
            "password": password,
        });

        let resp: LoginResponse = self
            .request_raw(reqwest::Method::POST, "auth/sign-in", Some(body))
            .await?;

        let token = resp.token;
        *self.token.write().await = Some(token.clone());
        Ok(token)
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<String, PicaError> {
        self.login_raw(email, password).await
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

    /// 标签精确过滤列表（comics?t=）。与分类链路同源，无 1 万条结果窗口。
    pub async fn comics_by_tag(
        &self,
        page: i64,
        tag: &str,
        sort: &str,
    ) -> Result<ComicList, PicaError> {
        let encoded = urlencoding::encode(tag);
        let path = format!("comics?page={}&s={}&t={}", page, sort, encoded);
        let resp: ComicsData = self
            .request(reqwest::Method::GET, &path, None)
            .await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const VALID_PASSWORD: &str = "right-pass";
    const TOKEN_STALE: &str = "token-stale";
    const TOKEN_LIVE: &str = "token-refreshed";

    /// 极简 HTTP mock：每个连接处理一个请求（Connection: close）。
    /// sign-in 校验密码；受保护接口只认 TOKEN_LIVE，其余一律 401 —— 正是 token 过期的现场。
    async fn spawn_mock(sign_ins: Arc<AtomicUsize>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let sign_ins = sign_ins.clone();
                tokio::spawn(async move {
                    let mut buffer = vec![0u8; 16384];
                    let mut read = 0usize;
                    loop {
                        let count = socket.read(&mut buffer[read..]).await.unwrap_or(0);
                        if count == 0 {
                            break;
                        }
                        read += count;
                        let Some(head_end) = buffer[..read]
                            .windows(4)
                            .position(|window| window == b"\r\n\r\n")
                        else {
                            continue;
                        };
                        let headers = String::from_utf8_lossy(&buffer[..head_end]);
                        let body_len = headers
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|value| value.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if read >= head_end + 4 + body_len {
                            break;
                        }
                    }
                    let text = String::from_utf8_lossy(&buffer[..read]);
                    let request_line = text.lines().next().unwrap_or("");
                    let mut parts = request_line.split_whitespace();
                    let path = parts.nth(1).unwrap_or("");
                    let authorization = text.lines().find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.trim()
                            .eq_ignore_ascii_case("authorization")
                            .then(|| value.trim().to_string())
                    });
                    let body = text.split("\r\n\r\n").nth(1).unwrap_or("");

                    let (status, payload) = if path.starts_with("/auth/sign-in") {
                        if body.contains(VALID_PASSWORD) {
                            sign_ins.fetch_add(1, Ordering::SeqCst);
                            (
                                200,
                                format!(
                                    r#"{{"code":200,"message":"ok","data":{{"token":"{TOKEN_LIVE}"}}}}"#
                                ),
                            )
                        } else {
                            (
                                401,
                                r#"{"code":401,"error":"1005","message":"unauthorized"}"#.to_string(),
                            )
                        }
                    } else if authorization.as_deref() == Some(TOKEN_LIVE) {
                        (
                            200,
                            r#"{"code":200,"message":"ok","data":{"value":"done"}}"#.to_string(),
                        )
                    } else {
                        (
                            401,
                            r#"{"code":401,"error":"1005","message":"unauthorized"}"#.to_string(),
                        )
                    };
                    let response = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        status,
                        if status == 200 { "OK" } else { "Unauthorized" },
                        payload.len(),
                        payload
                    );
                    socket.write_all(response.as_bytes()).await.ok();
                });
            }
        });
        format!("http://{address}/")
    }

    async fn mock_client(tag: &str, credentials: Option<(&str, &str)>) -> (Arc<PicaClient>, Arc<AtomicUsize>, std::path::PathBuf) {
        let root =
            std::env::temp_dir().join(format!("pika-client-test-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = Arc::new(
            Database::new(&root.join("test.db").to_string_lossy())
                .await
                .unwrap(),
        );
        let sign_ins = Arc::new(AtomicUsize::new(0));
        let base = spawn_mock(sign_ins.clone()).await;
        let client = Arc::new(PicaClient::with_api_base(db, &base));
        client
            .set_credentials(credentials.map(|(a, p)| (a.to_string(), p.to_string())))
            .await;
        (client, sign_ins, root)
    }

    #[tokio::test]
    async fn expired_token_relogins_retries_and_persists_the_fresh_token() {
        let (client, sign_ins, root) = mock_client("relogin", Some(("reader", VALID_PASSWORD))).await;
        client.set_token(Some(TOKEN_STALE.into())).await;

        let value: serde_json::Value = client
            .request(reqwest::Method::GET, "comics/test", None)
            .await
            .unwrap();
        assert_eq!(value["value"], "done");
        assert_eq!(client.token.read().await.as_deref(), Some(TOKEN_LIVE));
        // 新 token 已写回 settings，重启后能恢复会话。
        assert_eq!(
            client.db.get_setting("token").await.unwrap().as_deref(),
            Some(TOKEN_LIVE)
        );
        assert_eq!(sign_ins.load(Ordering::SeqCst), 1);
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn expired_token_without_saved_credentials_demands_relogin() {
        let (client, sign_ins, root) = mock_client("nocreds", None).await;
        client.set_token(Some(TOKEN_STALE.into())).await;

        let error = client
            .request::<serde_json::Value>(reqwest::Method::GET, "comics/test", None)
            .await
            .unwrap_err();
        assert!(matches!(error, PicaError::AuthRequired));
        assert!(error.to_string().starts_with("AUTH_REQUIRED:"));
        assert_eq!(sign_ins.load(Ordering::SeqCst), 0);
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn wrong_saved_password_surfaces_auth_required() {
        let (client, sign_ins, root) = mock_client("badpw", Some(("reader", "wrong-pass"))).await;
        client.set_token(Some(TOKEN_STALE.into())).await;

        let error = client
            .request::<serde_json::Value>(reqwest::Method::GET, "comics/test", None)
            .await
            .unwrap_err();
        assert!(matches!(error, PicaError::AuthRequired), "{error}");
        // 错误密码永远登不进去：计数只统计成功的 sign-in。
        assert_eq!(sign_ins.load(Ordering::SeqCst), 0);
        // 二次过期仍按会话失效处理，不会反复打转。
        let again = client
            .request::<serde_json::Value>(reqwest::Method::GET, "comics/test", None)
            .await
            .unwrap_err();
        assert!(matches!(again, PicaError::AuthRequired));
        assert_eq!(sign_ins.load(Ordering::SeqCst), 0);
        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn concurrent_expired_requests_relogin_only_once() {
        let (client, sign_ins, root) =
            mock_client("singleflight", Some(("reader", VALID_PASSWORD))).await;
        client.set_token(Some(TOKEN_STALE.into())).await;

        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..3 {
            let client = client.clone();
            tasks.spawn(async move {
                client
                    .request::<serde_json::Value>(reqwest::Method::GET, "comics/test", None)
                    .await
            });
        }
        while let Some(result) = tasks.join_next().await {
            let value = result.unwrap().unwrap();
            assert_eq!(value["value"], "done");
        }
        assert_eq!(sign_ins.load(Ordering::SeqCst), 1, "单飞锁：并发 401 只重登一次");
        std::fs::remove_dir_all(root).ok();
    }
}
