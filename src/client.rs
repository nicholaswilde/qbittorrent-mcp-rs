use anyhow::{Result, anyhow};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

use crate::models::{
    BuildInfo, Category, LogEntry, PeerLogEntry, RssRule, SearchJob, SearchPlugin,
    SearchResultsResponse, SyncMainData, Torrent, TorrentFile, TorrentProperties, Tracker,
    TransferInfo,
};

#[derive(Clone)]
pub struct QBitClient {
    http: Client,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
}

/// A .torrent file to upload: (file name, raw bytes).
pub type TorrentUpload = (String, Vec<u8>);

impl QBitClient {
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        no_verify_ssl: bool,
    ) -> Self {
        Self {
            http: Self::build_http(no_verify_ssl),
            base_url: base_url.into(),
            username: Some(username.into()),
            password: Some(password.into()),
        }
    }

    pub fn new_no_auth(base_url: impl Into<String>, no_verify_ssl: bool) -> Self {
        Self {
            http: Self::build_http(no_verify_ssl),
            base_url: base_url.into(),
            username: None,
            password: None,
        }
    }

    fn build_http(no_verify_ssl: bool) -> Client {
        // Cookie store keeps the SID session; timeouts stop a dead host from hanging every call.
        Client::builder()
            .cookie_store(true)
            .danger_accept_invalid_certs(no_verify_ssl)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client")
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn has_credentials(&self) -> bool {
        self.username.is_some()
    }

    fn api(&self, path: &str) -> String {
        format!("{}/api/v2/{}", self.base_url.trim_end_matches('/'), path)
    }

    pub async fn login(&self) -> Result<()> {
        let base_url = self.base_url.trim_end_matches('/');
        let params = [
            ("username", self.username.as_deref().unwrap_or("")),
            ("password", self.password.as_deref().unwrap_or("")),
        ];

        let resp = self
            .http
            .post(self.api("auth/login"))
            .header("Referer", format!("{}/", base_url))
            .header("Origin", base_url)
            .form(&params)
            .send()
            .await?;

        let status = resp.status();
        if status == StatusCode::FORBIDDEN {
            return Err(anyhow!(
                "Login failed: 403 Forbidden (this IP is banned after too many failed logins; restart qBittorrent or wait for the ban to expire)"
            ));
        }
        // qBittorrent 5.x answers a wrong username or password with 401.
        if status != StatusCode::UNAUTHORIZED && !status.is_success() {
            return Err(anyhow!("Login failed with status: {}", status));
        }
        // Older versions answer a wrong username or password with 2xx and body "Fails.". Success is
        // "Ok." on older versions and an empty body on newer ones (5.3), so only "Fails." is a failure.
        if status == StatusCode::UNAUTHORIZED || resp.text().await?.trim() == "Fails." {
            return Err(anyhow!("Login failed: wrong username or password"));
        }
        Ok(())
    }

    pub async fn logout(&self) -> Result<()> {
        self.post("auth/logout", &[], "log out").await
    }

    /// Sends a request; on 403 with credentials, logs in again and retries once.
    async fn send(&self, req: RequestBuilder) -> Result<Response> {
        let retry = req.try_clone();
        let resp = req.send().await?;
        if resp.status() == StatusCode::FORBIDDEN
            && self.has_credentials()
            && let Some(retry) = retry
        {
            // Surface why re-login failed (wrong password, IP ban) instead of a bare 403.
            self.login().await?;
            return Ok(retry.send().await?);
        }
        Ok(resp)
    }

    fn check(resp: Response, what: &str) -> Result<Response> {
        if resp.status().is_success() {
            Ok(resp)
        } else {
            Err(anyhow!("Failed to {}: {}", what, resp.status()))
        }
    }

    async fn get_resp(&self, path: &str, query: &[(&str, &str)], what: &str) -> Result<Response> {
        let url = url::Url::parse_with_params(&self.api(path), query)?;
        Self::check(self.send(self.http.get(url)).await?, what)
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
        what: &str,
    ) -> Result<T> {
        Ok(self.get_resp(path, query, what).await?.json().await?)
    }

    async fn get_text(&self, path: &str, what: &str) -> Result<String> {
        Ok(self.get_resp(path, &[], what).await?.text().await?)
    }

    async fn post_resp(&self, path: &str, form: &[(&str, &str)]) -> Result<Response> {
        self.send(self.http.post(self.api(path)).form(form)).await
    }

    async fn post(&self, path: &str, form: &[(&str, &str)], what: &str) -> Result<()> {
        Self::check(self.post_resp(path, form).await?, what).map(|_| ())
    }

    /// qBittorrent 5 renamed some endpoints; try the new one, fall back to the old one on 404.
    async fn post_v5_or_v4(
        &self,
        v5: &str,
        v4: &str,
        form: &[(&str, &str)],
        what_v5: &str,
        what_v4: &str,
    ) -> Result<()> {
        let resp = self.post_resp(v5, form).await?;
        if resp.status() == StatusCode::NOT_FOUND {
            return self.post(v4, form, what_v4).await;
        }
        Self::check(resp, what_v5).map(|_| ())
    }

    async fn get_json_v5_or_v4<T: DeserializeOwned>(
        &self,
        v5: &str,
        v5_query: &[(&str, &str)],
        v4: &str,
        what: &str,
    ) -> Result<T> {
        let url = url::Url::parse_with_params(&self.api(v5), v5_query)?;
        let resp = self.send(self.http.get(url)).await?;
        if resp.status() == StatusCode::NOT_FOUND {
            return self.get_json(v4, &[], what).await;
        }
        Ok(Self::check(resp, what)?.json().await?)
    }

    // ── Torrents ──────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn get_torrent_list(
        &self,
        filter: Option<&str>,
        category: Option<&str>,
        tag: Option<&str>,
        sort: Option<&str>,
        reverse: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Torrent>> {
        let reverse = reverse.map(|r| r.to_string());
        let limit = limit.map(|l| l.to_string());
        let offset = offset.map(|o| o.to_string());
        let query: Vec<(&str, &str)> = [
            ("filter", filter),
            ("category", category),
            ("tag", tag),
            ("sort", sort),
            ("reverse", reverse.as_deref()),
            ("limit", limit.as_deref()),
            ("offset", offset.as_deref()),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .collect();
        self.get_json("torrents/info", &query, "get torrent list")
            .await
    }

    pub async fn get_torrents_info(&self, hashes: &str) -> Result<Vec<Torrent>> {
        self.get_json("torrents/info", &[("hashes", hashes)], "get torrents info")
            .await
    }

    pub async fn add_torrent(
        &self,
        urls: &str,
        save_path: Option<&str>,
        category: Option<&str>,
    ) -> Result<()> {
        let mut fields = vec![("urls", urls.to_string())];
        if let Some(p) = save_path {
            fields.push(("savepath", p.to_string()));
        }
        if let Some(c) = category {
            fields.push(("category", c.to_string()));
        }
        self.add_torrent_full(&fields, None).await
    }

    /// Adds torrents with any `torrents/add` form fields, plus an optional .torrent file upload.
    pub async fn add_torrent_full(
        &self,
        fields: &[(&str, String)],
        file: Option<TorrentUpload>,
    ) -> Result<()> {
        let mut form = reqwest::multipart::Form::new();
        for (k, v) in fields {
            form = form.text(k.to_string(), v.clone());
        }
        if let Some((name, bytes)) = file {
            form = form.part(
                "torrents",
                reqwest::multipart::Part::bytes(bytes)
                    .file_name(name)
                    .mime_str("application/x-bittorrent")?,
            );
        }
        let resp = self
            .send(self.http.post(self.api("torrents/add")).multipart(form))
            .await?;
        let resp = Self::check(resp, "add torrent")?;
        // qBittorrent answers 200 "Fails." when it rejects every torrent in the request.
        if resp.text().await?.trim() == "Fails." {
            return Err(anyhow!("Failed to add torrent: qBittorrent rejected it"));
        }
        Ok(())
    }

    pub async fn pause_torrents(&self, hashes: &str) -> Result<()> {
        self.post_v5_or_v4(
            "torrents/stop",
            "torrents/pause",
            &[("hashes", hashes)],
            "stop/pause torrents",
            "pause torrents",
        )
        .await
    }

    pub async fn resume_torrents(&self, hashes: &str) -> Result<()> {
        self.post_v5_or_v4(
            "torrents/start",
            "torrents/resume",
            &[("hashes", hashes)],
            "start/resume torrents",
            "resume torrents",
        )
        .await
    }

    pub async fn delete_torrents(&self, hashes: &str, delete_files: bool) -> Result<()> {
        let delete_files = delete_files.to_string();
        self.post(
            "torrents/delete",
            &[("hashes", hashes), ("deleteFiles", &delete_files)],
            "delete torrents",
        )
        .await
    }

    pub async fn reannounce_torrents(&self, hashes: &str) -> Result<()> {
        self.post(
            "torrents/reannounce",
            &[("hashes", hashes)],
            "reannounce torrents",
        )
        .await
    }

    pub async fn recheck_torrents(&self, hashes: &str) -> Result<()> {
        self.post(
            "torrents/recheck",
            &[("hashes", hashes)],
            "recheck torrents",
        )
        .await
    }

    pub async fn get_torrent_files(&self, hash: &str) -> Result<Vec<TorrentFile>> {
        self.get_json("torrents/files", &[("hash", hash)], "get torrent files")
            .await
    }

    pub async fn get_torrent_properties(&self, hash: &str) -> Result<TorrentProperties> {
        self.get_json(
            "torrents/properties",
            &[("hash", hash)],
            "get torrent properties",
        )
        .await
    }

    pub async fn get_torrent_trackers(&self, hash: &str) -> Result<Vec<Tracker>> {
        self.get_json(
            "torrents/trackers",
            &[("hash", hash)],
            "get torrent trackers",
        )
        .await
    }

    pub async fn get_torrent_webseeds(&self, hash: &str) -> Result<Value> {
        self.get_json("torrents/webseeds", &[("hash", hash)], "get web seeds")
            .await
    }

    pub async fn get_piece_states(&self, hash: &str) -> Result<Vec<i64>> {
        self.get_json(
            "torrents/pieceStates",
            &[("hash", hash)],
            "get piece states",
        )
        .await
    }

    pub async fn get_piece_hashes(&self, hash: &str) -> Result<Vec<String>> {
        self.get_json(
            "torrents/pieceHashes",
            &[("hash", hash)],
            "get piece hashes",
        )
        .await
    }

    pub async fn get_torrent_peers(&self, hash: &str) -> Result<Value> {
        self.get_json(
            "sync/torrentPeers",
            &[("hash", hash), ("rid", "0")],
            "get torrent peers",
        )
        .await
    }

    pub async fn add_peers(&self, hashes: &str, peers: &str) -> Result<()> {
        self.post(
            "torrents/addPeers",
            &[("hashes", hashes), ("peers", peers)],
            "add peers",
        )
        .await
    }

    pub async fn set_location(&self, hashes: &str, location: &str) -> Result<()> {
        self.post(
            "torrents/setLocation",
            &[("hashes", hashes), ("location", location)],
            "set location",
        )
        .await
    }

    pub async fn rename_torrent(&self, hash: &str, name: &str) -> Result<()> {
        self.post(
            "torrents/rename",
            &[("hash", hash), ("name", name)],
            "rename torrent",
        )
        .await
    }

    pub async fn set_auto_management(&self, hashes: &str, enable: bool) -> Result<()> {
        let enable = enable.to_string();
        self.post(
            "torrents/setAutoManagement",
            &[("hashes", hashes), ("enable", &enable)],
            "set automatic torrent management",
        )
        .await
    }

    /// `which` is one of topPrio, bottomPrio, increasePrio, decreasePrio.
    pub async fn set_queue_priority(&self, hashes: &str, which: &str) -> Result<()> {
        self.post(
            &format!("torrents/{}", which),
            &[("hashes", hashes)],
            "change queue priority",
        )
        .await
    }

    pub async fn rename_file(&self, hash: &str, old_path: &str, new_path: &str) -> Result<()> {
        self.post(
            "torrents/renameFile",
            &[("hash", hash), ("oldPath", old_path), ("newPath", new_path)],
            "rename file",
        )
        .await
    }

    pub async fn rename_folder(&self, hash: &str, old_path: &str, new_path: &str) -> Result<()> {
        self.post(
            "torrents/renameFolder",
            &[("hash", hash), ("oldPath", old_path), ("newPath", new_path)],
            "rename folder",
        )
        .await
    }

    pub async fn set_file_priority(&self, hash: &str, id: &str, priority: i32) -> Result<()> {
        let priority = priority.to_string();
        self.post(
            "torrents/filePrio",
            &[("hash", hash), ("id", id), ("priority", &priority)],
            "set file priority",
        )
        .await
    }

    pub async fn set_torrent_share_limits(
        &self,
        hashes: &str,
        ratio_limit: f64,
        seeding_time_limit: i64,
        inactive_seeding_time_limit: Option<i64>,
    ) -> Result<()> {
        let ratio = ratio_limit.to_string();
        let seeding = seeding_time_limit.to_string();
        // -2 = use global limit; newer qBittorrent versions require the field.
        let inactive = inactive_seeding_time_limit.unwrap_or(-2).to_string();
        self.post(
            "torrents/setShareLimits",
            &[
                ("hashes", hashes),
                ("ratioLimit", &ratio),
                ("seedingTimeLimit", &seeding),
                ("inactiveSeedingTimeLimit", &inactive),
            ],
            "set torrent share limits",
        )
        .await
    }

    pub async fn set_torrent_download_limit(&self, hashes: &str, limit: i64) -> Result<()> {
        let limit = limit.to_string();
        self.post(
            "torrents/setDownloadLimit",
            &[("hashes", hashes), ("limit", &limit)],
            "set torrent download limit",
        )
        .await
    }

    pub async fn set_torrent_upload_limit(&self, hashes: &str, limit: i64) -> Result<()> {
        let limit = limit.to_string();
        self.post(
            "torrents/setUploadLimit",
            &[("hashes", hashes), ("limit", &limit)],
            "set torrent upload limit",
        )
        .await
    }

    pub async fn toggle_sequential_download(&self, hashes: &str) -> Result<()> {
        self.post(
            "torrents/toggleSequentialDownload",
            &[("hashes", hashes)],
            "toggle sequential download",
        )
        .await
    }

    pub async fn toggle_first_last_piece_priority(&self, hashes: &str) -> Result<()> {
        self.post(
            "torrents/toggleFirstLastPiecePrio",
            &[("hashes", hashes)],
            "toggle first/last piece priority",
        )
        .await
    }

    pub async fn set_force_start(&self, hashes: &str, value: bool) -> Result<()> {
        let value = value.to_string();
        self.post(
            "torrents/setForceStart",
            &[("hashes", hashes), ("value", &value)],
            "set force start",
        )
        .await
    }

    pub async fn set_super_seeding(&self, hashes: &str, value: bool) -> Result<()> {
        let value = value.to_string();
        self.post(
            "torrents/setSuperSeeding",
            &[("hashes", hashes), ("value", &value)],
            "set super seeding",
        )
        .await
    }

    pub async fn add_trackers(&self, hashes: &str, urls: &str) -> Result<()> {
        self.post(
            "torrents/addTrackers",
            &[("hashes", hashes), ("urls", urls)],
            "add trackers",
        )
        .await
    }

    pub async fn edit_tracker(&self, hash: &str, orig_url: &str, new_url: &str) -> Result<()> {
        self.post(
            "torrents/editTracker",
            &[("hash", hash), ("origUrl", orig_url), ("newUrl", new_url)],
            "edit tracker",
        )
        .await
    }

    pub async fn remove_trackers(&self, hashes: &str, urls: &str) -> Result<()> {
        self.post(
            "torrents/removeTrackers",
            &[("hashes", hashes), ("urls", urls)],
            "remove trackers",
        )
        .await
    }

    // ── Categories & tags ─────────────────────────────────────────────

    pub async fn get_categories(&self) -> Result<HashMap<String, Category>> {
        self.get_json("torrents/categories", &[], "get categories")
            .await
    }

    pub async fn create_category(&self, name: &str, save_path: &str) -> Result<()> {
        self.post(
            "torrents/createCategory",
            &[("category", name), ("savePath", save_path)],
            "create category",
        )
        .await
    }

    pub async fn edit_category(&self, name: &str, save_path: &str) -> Result<()> {
        self.post(
            "torrents/editCategory",
            &[("category", name), ("savePath", save_path)],
            "edit category",
        )
        .await
    }

    pub async fn remove_categories(&self, categories: &str) -> Result<()> {
        self.post(
            "torrents/removeCategories",
            &[("categories", categories)],
            "remove categories",
        )
        .await
    }

    pub async fn set_category(&self, hashes: &str, category: &str) -> Result<()> {
        self.post(
            "torrents/setCategory",
            &[("hashes", hashes), ("category", category)],
            "set category",
        )
        .await
    }

    pub async fn get_tags(&self) -> Result<Vec<String>> {
        self.get_json("torrents/tags", &[], "get tags").await
    }

    pub async fn add_tags(&self, hashes: &str, tags: &str) -> Result<()> {
        self.post(
            "torrents/addTags",
            &[("hashes", hashes), ("tags", tags)],
            "add tags",
        )
        .await
    }

    pub async fn remove_tags(&self, hashes: &str, tags: &str) -> Result<()> {
        self.post(
            "torrents/removeTags",
            &[("hashes", hashes), ("tags", tags)],
            "remove tags",
        )
        .await
    }

    pub async fn create_tags(&self, tags: &str) -> Result<()> {
        self.post("torrents/createTags", &[("tags", tags)], "create tags")
            .await
    }

    pub async fn delete_tags(&self, tags: &str) -> Result<()> {
        self.post("torrents/deleteTags", &[("tags", tags)], "delete tags")
            .await
    }

    // ── Transfer ──────────────────────────────────────────────────────

    pub async fn get_global_transfer_info(&self) -> Result<TransferInfo> {
        self.get_json("transfer/info", &[], "get global transfer info")
            .await
    }

    pub async fn set_download_limit(&self, limit: i64) -> Result<()> {
        let limit = limit.to_string();
        self.post(
            "transfer/setDownloadLimit",
            &[("limit", &limit)],
            "set download limit",
        )
        .await
    }

    pub async fn set_upload_limit(&self, limit: i64) -> Result<()> {
        let limit = limit.to_string();
        self.post(
            "transfer/setUploadLimit",
            &[("limit", &limit)],
            "set upload limit",
        )
        .await
    }

    pub async fn toggle_alternative_speed_limits(&self) -> Result<()> {
        self.post(
            "transfer/toggleSpeedLimitsMode",
            &[],
            "toggle alternative speed limits",
        )
        .await
    }

    pub async fn get_speed_limits_mode(&self) -> Result<i64> {
        Ok(self
            .get_text("transfer/speedLimitsMode", "get speed limits mode")
            .await?
            .trim()
            .parse()?)
    }

    pub async fn ban_peers(&self, peers: &str) -> Result<()> {
        self.post("transfer/banPeers", &[("peers", peers)], "ban peers")
            .await
    }

    pub async fn get_main_data(&self, rid: i64) -> Result<SyncMainData> {
        self.get_json(
            "sync/maindata",
            &[("rid", &rid.to_string())],
            "get main data",
        )
        .await
    }

    // ── Search ────────────────────────────────────────────────────────

    pub async fn start_search(&self, pattern: &str, category: Option<&str>) -> Result<i64> {
        let resp = Self::check(
            self.post_resp(
                "search/start",
                &[
                    ("pattern", pattern),
                    ("plugins", "all"),
                    ("category", category.unwrap_or("all")),
                ],
            )
            .await?,
            "start search",
        )?;
        Ok(resp.json::<SearchJob>().await?.id)
    }

    /// Returns the job's status string ("Running" or "Stopped").
    pub async fn get_search_status(&self, id: i64) -> Result<String> {
        let jobs: Vec<Value> = self
            .get_json(
                "search/status",
                &[("id", &id.to_string())],
                "get search status",
            )
            .await?;
        Ok(jobs
            .first()
            .and_then(|j| j.get("status"))
            .and_then(|s| s.as_str())
            .unwrap_or("Stopped")
            .to_string())
    }

    pub async fn get_search_results(
        &self,
        id: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<SearchResultsResponse> {
        let id = id.to_string();
        let limit = limit.map(|l| l.to_string());
        let offset = offset.map(|o| o.to_string());
        let mut form = vec![("id", id.as_str())];
        if let Some(l) = &limit {
            form.push(("limit", l));
        }
        if let Some(o) = &offset {
            form.push(("offset", o));
        }
        let resp = Self::check(
            self.post_resp("search/results", &form).await?,
            "get search results",
        )?;
        Ok(resp.json().await?)
    }

    pub async fn stop_search(&self, id: i64) -> Result<()> {
        self.post("search/stop", &[("id", &id.to_string())], "stop search")
            .await
    }

    pub async fn delete_search(&self, id: i64) -> Result<()> {
        self.post("search/delete", &[("id", &id.to_string())], "delete search")
            .await
    }

    pub async fn get_search_plugins(&self) -> Result<Vec<SearchPlugin>> {
        self.get_json("search/plugins", &[], "get search plugins")
            .await
    }

    pub async fn install_search_plugin(&self, url_source: &str) -> Result<()> {
        self.post(
            "search/installPlugin",
            &[("sources", url_source)],
            "install search plugin",
        )
        .await
    }

    pub async fn uninstall_search_plugin(&self, name: &str) -> Result<()> {
        self.post(
            "search/uninstallPlugin",
            &[("names", name)],
            "uninstall search plugin",
        )
        .await
    }

    pub async fn enable_search_plugin(&self, name: &str, enable: bool) -> Result<()> {
        let enable = enable.to_string();
        self.post(
            "search/enablePlugin",
            &[("names", name), ("enable", &enable)],
            "toggle search plugin",
        )
        .await
    }

    pub async fn update_search_plugins(&self) -> Result<()> {
        self.post("search/updatePlugins", &[], "update search plugins")
            .await
    }

    // ── RSS ───────────────────────────────────────────────────────────

    pub async fn add_rss_feed(&self, url_feed: &str, path: &str) -> Result<()> {
        self.post(
            "rss/addFeed",
            &[("url", url_feed), ("path", path)],
            "add RSS feed",
        )
        .await
    }

    pub async fn add_rss_folder(&self, path: &str) -> Result<()> {
        self.post("rss/addFolder", &[("path", path)], "add RSS folder")
            .await
    }

    pub async fn remove_rss_item(&self, path: &str) -> Result<()> {
        self.post("rss/removeItem", &[("path", path)], "remove RSS item")
            .await
    }

    pub async fn move_rss_item(&self, item_path: &str, dest_path: &str) -> Result<()> {
        self.post(
            "rss/moveItem",
            &[("itemPath", item_path), ("destPath", dest_path)],
            "move RSS item",
        )
        .await
    }

    pub async fn mark_rss_as_read(&self, item_path: &str, article_id: Option<&str>) -> Result<()> {
        let mut form = vec![("itemPath", item_path)];
        if let Some(id) = article_id {
            form.push(("articleId", id));
        }
        self.post("rss/markAsRead", &form, "mark RSS item as read")
            .await
    }

    pub async fn refresh_rss_item(&self, item_path: &str) -> Result<()> {
        self.post(
            "rss/refreshItem",
            &[("itemPath", item_path)],
            "refresh RSS item",
        )
        .await
    }

    pub async fn get_all_rss_feeds(&self) -> Result<HashMap<String, Value>> {
        self.get_json_v5_or_v4(
            "rss/items",
            &[("withData", "true")],
            "rss/allFeeds",
            "get RSS feeds",
        )
        .await
    }

    pub async fn set_rss_rule(&self, name: &str, definition: &str) -> Result<()> {
        self.post(
            "rss/setRule",
            &[("ruleName", name), ("ruleDef", definition)],
            "set RSS rule",
        )
        .await
    }

    pub async fn rename_rss_rule(&self, name: &str, new_name: &str) -> Result<()> {
        self.post(
            "rss/renameRule",
            &[("ruleName", name), ("newRuleName", new_name)],
            "rename RSS rule",
        )
        .await
    }

    pub async fn remove_rss_rule(&self, name: &str) -> Result<()> {
        self.post("rss/removeRule", &[("ruleName", name)], "remove RSS rule")
            .await
    }

    pub async fn get_rss_matching_articles(&self, name: &str) -> Result<Value> {
        self.get_json(
            "rss/matchingArticles",
            &[("ruleName", name)],
            "get RSS matching articles",
        )
        .await
    }

    pub async fn get_all_rss_rules(&self) -> Result<HashMap<String, RssRule>> {
        self.get_json_v5_or_v4("rss/rules", &[], "rss/allRules", "get RSS rules")
            .await
    }

    // ── App ───────────────────────────────────────────────────────────

    pub async fn get_app_preferences(&self) -> Result<Value> {
        self.get_json("app/preferences", &[], "get app preferences")
            .await
    }

    pub async fn set_app_preferences(&self, prefs: &Value) -> Result<()> {
        self.post(
            "app/setPreferences",
            &[("json", &prefs.to_string())],
            "set app preferences",
        )
        .await
    }

    pub async fn get_app_version(&self) -> Result<String> {
        self.get_text("app/version", "get app version").await
    }

    pub async fn get_webapi_version(&self) -> Result<String> {
        self.get_text("app/webapiVersion", "get Web API version")
            .await
    }

    pub async fn get_default_save_path(&self) -> Result<String> {
        self.get_text("app/defaultSavePath", "get default save path")
            .await
    }

    pub async fn get_build_info(&self) -> Result<BuildInfo> {
        self.get_json("app/buildInfo", &[], "get build info").await
    }

    pub async fn get_cookies(&self) -> Result<Value> {
        self.get_json("app/cookies", &[], "get cookies").await
    }

    pub async fn set_cookies(&self, cookies_json: &str) -> Result<()> {
        self.post(
            "app/setCookies",
            &[("cookies", cookies_json)],
            "set cookies",
        )
        .await
    }

    pub async fn shutdown_app(&self) -> Result<()> {
        self.post("app/shutdown", &[], "shutdown app").await
    }

    // ── Logs ──────────────────────────────────────────────────────────

    pub async fn get_main_log(
        &self,
        normal: bool,
        info: bool,
        warning: bool,
        critical: bool,
        last_id: Option<i64>,
    ) -> Result<Vec<LogEntry>> {
        let flags = [normal, info, warning, critical].map(|b| b.to_string());
        let last_id = last_id.map(|i| i.to_string());
        let mut query = vec![
            ("normal", flags[0].as_str()),
            ("info", &flags[1]),
            ("warning", &flags[2]),
            ("critical", &flags[3]),
        ];
        if let Some(id) = &last_id {
            query.push(("last_id", id));
        }
        self.get_json("log/main", &query, "get main log").await
    }

    pub async fn get_peer_log(&self, last_id: Option<i64>) -> Result<Vec<PeerLogEntry>> {
        let last_id = last_id.map(|i| i.to_string());
        let query: Vec<(&str, &str)> = last_id.iter().map(|i| ("last_id", i.as_str())).collect();
        self.get_json("log/peers", &query, "get peer log").await
    }
}
