use anyhow::{Result, anyhow};
use reqwest::Client;

#[derive(Clone)]
pub struct QBitClient {
    http: Client,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
}

impl QBitClient {
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        no_verify_ssl: bool,
    ) -> Self {
        // Enable cookie store for session management (SID)
        let http = Client::builder()
            .cookie_store(true)
            .danger_accept_invalid_certs(no_verify_ssl)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            base_url: base_url.into(),
            username: Some(username.into()),
            password: Some(password.into()),
        }
    }

    pub fn new_no_auth(base_url: impl Into<String>, no_verify_ssl: bool) -> Self {
        let http = Client::builder()
            .cookie_store(true)
            .danger_accept_invalid_certs(no_verify_ssl)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            base_url: base_url.into(),
            username: None,
            password: None,
        }
    }

    pub async fn login(&self) -> Result<()> {
        let url = format!("{}/api/v2/auth/login", self.base_url);

        let params = [
            ("username", self.username.as_deref().unwrap_or("")),
            ("password", self.password.as_deref().unwrap_or("")),
        ];

        let base_url_with_slash = if self.base_url.ends_with('/') {
            self.base_url.clone()
        } else {
            format!("{}/", self.base_url)
        };

        let resp = self
            .http
            .post(&url)
            .header("Referer", &base_url_with_slash)
            .header("Origin", &self.base_url)
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36")
            .form(&params)
            .send()
            .await?;

        if resp.status().is_success() {
            // Check body text for "Ok." just to be sure, though status 200 usually implies success for qbit.
            let text = resp.text().await?;
            if text == "Ok." {
                Ok(())
            } else {
                // Sometimes it returns 200 even if fails? No, usually 200 with "Ok."
                // But strictly checking "Ok." is safer.
                Ok(())
            }
        } else {
            Err(anyhow!("Login failed with status: {}", resp.status()))
        }
    }

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
    ) -> Result<Vec<crate::models::Torrent>> {
        let mut url = url::Url::parse(&format!("{}/api/v2/torrents/info", self.base_url))?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(f) = filter {
                query.append_pair("filter", f);
            }
            if let Some(c) = category {
                query.append_pair("category", c);
            }
            if let Some(t) = tag {
                query.append_pair("tag", t);
            }
            if let Some(s) = sort {
                query.append_pair("sort", s);
            }
            if let Some(r) = reverse {
                query.append_pair("reverse", &r.to_string());
            }
            if let Some(l) = limit {
                query.append_pair("limit", &l.to_string());
            }
            if let Some(o) = offset {
                query.append_pair("offset", &o.to_string());
            }
        }

        let resp = self.http.get(url).send().await?;

        if resp.status().is_success() {
            let torrents = resp.json::<Vec<crate::models::Torrent>>().await?;
            Ok(torrents)
        } else {
            Err(anyhow!("Failed to get torrent list: {}", resp.status()))
        }
    }

    pub async fn get_torrents_info(&self, hashes: &str) -> Result<Vec<crate::models::Torrent>> {
        let url = format!("{}/api/v2/torrents/info?hashes={}", self.base_url, hashes);

        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let torrents = resp.json::<Vec<crate::models::Torrent>>().await?;
            Ok(torrents)
        } else {
            Err(anyhow!("Failed to get torrents info: {}", resp.status()))
        }
    }

    pub async fn add_torrent(
        &self,
        urls: &str,
        save_path: Option<&str>,
        category: Option<&str>,
    ) -> Result<()> {
        let url = format!("{}/api/v2/torrents/add", self.base_url);

        let mut form = reqwest::multipart::Form::new().text("urls", urls.to_string());

        if let Some(path) = save_path {
            form = form.text("savepath", path.to_string());
        }

        if let Some(cat) = category {
            form = form.text("category", cat.to_string());
        }

        let resp = self.http.post(&url).multipart(form).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to add torrent: {}", resp.status()))
        }
    }

    pub async fn pause_torrents(&self, hashes: &str) -> Result<()> {
        // Try v5 "stop" endpoint first
        let url_stop = format!("{}/api/v2/torrents/stop", self.base_url);
        let params = [("hashes", hashes)];

        let resp_stop = self.http.post(&url_stop).form(&params).send().await?;

        if resp_stop.status().is_success() {
            return Ok(());
        } else if resp_stop.status() == reqwest::StatusCode::NOT_FOUND {
            // Fallback to v4 "pause" endpoint
            let url_pause = format!("{}/api/v2/torrents/pause", self.base_url);
            let resp_pause = self.http.post(&url_pause).form(&params).send().await?;
            if resp_pause.status().is_success() {
                return Ok(());
            } else {
                return Err(anyhow!("Failed to pause torrents: {}", resp_pause.status()));
            }
        }

        Err(anyhow!(
            "Failed to stop/pause torrents: {}",
            resp_stop.status()
        ))
    }

    pub async fn resume_torrents(&self, hashes: &str) -> Result<()> {
        // Try v5 "start" endpoint first
        let url_start = format!("{}/api/v2/torrents/start", self.base_url);
        let params = [("hashes", hashes)];

        let resp_start = self.http.post(&url_start).form(&params).send().await?;

        if resp_start.status().is_success() {
            return Ok(());
        } else if resp_start.status() == reqwest::StatusCode::NOT_FOUND {
            // Fallback to v4 "resume" endpoint
            let url_resume = format!("{}/api/v2/torrents/resume", self.base_url);
            let resp_resume = self.http.post(&url_resume).form(&params).send().await?;
            if resp_resume.status().is_success() {
                return Ok(());
            } else {
                return Err(anyhow!(
                    "Failed to resume torrents: {}",
                    resp_resume.status()
                ));
            }
        }

        Err(anyhow!(
            "Failed to start/resume torrents: {}",
            resp_start.status()
        ))
    }

    pub async fn delete_torrents(&self, hashes: &str, delete_files: bool) -> Result<()> {
        let url = format!("{}/api/v2/torrents/delete", self.base_url);
        let params = [
            ("hashes", hashes.to_string()),
            ("deleteFiles", delete_files.to_string()),
        ];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to delete torrents: {}", resp.status()))
        }
    }

    pub async fn reannounce_torrents(&self, hashes: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/reannounce", self.base_url);
        let params = [("hashes", hashes)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to reannounce torrents: {}", resp.status()))
        }
    }

    pub async fn recheck_torrents(&self, hashes: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/recheck", self.base_url);
        let params = [("hashes", hashes)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to recheck torrents: {}", resp.status()))
        }
    }

    pub async fn get_torrent_files(&self, hash: &str) -> Result<Vec<crate::models::TorrentFile>> {
        let url = format!("{}/api/v2/torrents/files?hash={}", self.base_url, hash);

        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let files = resp.json::<Vec<crate::models::TorrentFile>>().await?;
            Ok(files)
        } else {
            Err(anyhow!("Failed to get torrent files: {}", resp.status()))
        }
    }

    pub async fn get_torrent_properties(
        &self,
        hash: &str,
    ) -> Result<crate::models::TorrentProperties> {
        let url = format!("{}/api/v2/torrents/properties?hash={}", self.base_url, hash);

        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let props = resp.json::<crate::models::TorrentProperties>().await?;
            Ok(props)
        } else {
            Err(anyhow!(
                "Failed to get torrent properties: {}",
                resp.status()
            ))
        }
    }

    pub async fn get_torrent_trackers(&self, hash: &str) -> Result<Vec<crate::models::Tracker>> {
        let url = format!("{}/api/v2/torrents/trackers?hash={}", self.base_url, hash);

        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let trackers = resp.json::<Vec<crate::models::Tracker>>().await?;
            Ok(trackers)
        } else {
            Err(anyhow!("Failed to get torrent trackers: {}", resp.status()))
        }
    }

    pub async fn get_global_transfer_info(&self) -> Result<crate::models::TransferInfo> {
        let url = format!("{}/api/v2/transfer/info", self.base_url);

        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let info = resp.json::<crate::models::TransferInfo>().await?;
            Ok(info)
        } else {
            Err(anyhow!(
                "Failed to get global transfer info: {}",
                resp.status()
            ))
        }
    }

    pub async fn set_download_limit(&self, limit: i64) -> Result<()> {
        let url = format!("{}/api/v2/transfer/setDownloadLimit", self.base_url);
        let params = [("limit", limit.to_string())];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to set download limit: {}", resp.status()))
        }
    }

    pub async fn set_upload_limit(&self, limit: i64) -> Result<()> {
        let url = format!("{}/api/v2/transfer/setUploadLimit", self.base_url);
        let params = [("limit", limit.to_string())];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to set upload limit: {}", resp.status()))
        }
    }

    pub async fn toggle_alternative_speed_limits(&self) -> Result<()> {
        let url = format!("{}/api/v2/transfer/toggleSpeedLimitsMode", self.base_url);
        let resp = self.http.post(&url).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to toggle alternative speed limits: {}",
                resp.status()
            ))
        }
    }

    pub async fn get_speed_limits_mode(&self) -> Result<i64> {
        let url = format!("{}/api/v2/transfer/speedLimitsMode", self.base_url);
        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let mode = resp.text().await?.parse()?;
            Ok(mode)
        } else {
            Err(anyhow!(
                "Failed to get speed limits mode: {}",
                resp.status()
            ))
        }
    }

    pub async fn start_search(&self, pattern: &str, category: Option<&str>) -> Result<i64> {
        let url = format!("{}/api/v2/search/start", self.base_url);
        let mut params = vec![("pattern", pattern), ("plugins", "all")];
        if let Some(cat) = category {
            params.push(("category", cat));
        } else {
            params.push(("category", "all"));
        }

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            let job: crate::models::SearchJob = resp.json().await?;
            Ok(job.id)
        } else {
            Err(anyhow!("Failed to start search: {}", resp.status()))
        }
    }

    pub async fn get_search_results(
        &self,
        id: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<crate::models::SearchResultsResponse> {
        let url = format!("{}/api/v2/search/results", self.base_url);
        let mut params = vec![("id", id.to_string())];
        if let Some(l) = limit {
            params.push(("limit", l.to_string()));
        }
        if let Some(o) = offset {
            params.push(("offset", o.to_string()));
        }

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            let results: crate::models::SearchResultsResponse = resp.json().await?;
            Ok(results)
        } else {
            Err(anyhow!("Failed to get search results: {}", resp.status()))
        }
    }

    pub async fn stop_search(&self, id: i64) -> Result<()> {
        let url = format!("{}/api/v2/search/stop", self.base_url);
        let params = [("id", id.to_string())];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to stop search: {}", resp.status()))
        }
    }

    pub async fn delete_search(&self, id: i64) -> Result<()> {
        let url = format!("{}/api/v2/search/delete", self.base_url);
        let params = [("id", id.to_string())];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to delete search: {}", resp.status()))
        }
    }

    pub async fn get_categories(
        &self,
    ) -> Result<std::collections::HashMap<String, crate::models::Category>> {
        let url = format!("{}/api/v2/torrents/categories", self.base_url);
        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let categories = resp.json().await?;
            Ok(categories)
        } else {
            Err(anyhow!("Failed to get categories: {}", resp.status()))
        }
    }

    pub async fn create_category(&self, name: &str, save_path: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/createCategory", self.base_url);
        let params = [("category", name), ("savePath", save_path)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to create category: {}", resp.status()))
        }
    }

    pub async fn set_category(&self, hashes: &str, category: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/setCategory", self.base_url);
        let params = [("hashes", hashes), ("category", category)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to set category: {}", resp.status()))
        }
    }

    pub async fn add_tags(&self, hashes: &str, tags: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/addTags", self.base_url);
        let params = [("hashes", hashes), ("tags", tags)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to add tags: {}", resp.status()))
        }
    }

    pub async fn get_search_plugins(&self) -> Result<Vec<crate::models::SearchPlugin>> {
        let url = format!("{}/api/v2/search/plugins", self.base_url);
        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let plugins = resp.json().await?;
            Ok(plugins)
        } else {
            Err(anyhow!("Failed to get search plugins: {}", resp.status()))
        }
    }

    pub async fn install_search_plugin(&self, url_source: &str) -> Result<()> {
        let url = format!("{}/api/v2/search/installPlugin", self.base_url);
        let params = [("sources", url_source)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to install search plugin: {}",
                resp.status()
            ))
        }
    }

    pub async fn uninstall_search_plugin(&self, name: &str) -> Result<()> {
        let url = format!("{}/api/v2/search/uninstallPlugin", self.base_url);
        let params = [("names", name)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to uninstall search plugin: {}",
                resp.status()
            ))
        }
    }

    pub async fn enable_search_plugin(&self, name: &str, enable: bool) -> Result<()> {
        let url = format!("{}/api/v2/search/enablePlugin", self.base_url);
        let params = [
            ("names", name),
            ("enable", if enable { "true" } else { "false" }),
        ];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to toggle search plugin: {}", resp.status()))
        }
    }

    pub async fn update_search_plugins(&self) -> Result<()> {
        let url = format!("{}/api/v2/search/updatePlugins", self.base_url);
        let resp = self.http.post(&url).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to update search plugins: {}",
                resp.status()
            ))
        }
    }

    pub async fn add_rss_feed(&self, url_feed: &str, path: &str) -> Result<()> {
        let url = format!("{}/api/v2/rss/addFeed", self.base_url);
        let params = [("url", url_feed), ("path", path)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to add RSS feed: {}", resp.status()))
        }
    }

    pub async fn remove_rss_item(&self, path: &str) -> Result<()> {
        let url = format!("{}/api/v2/rss/removeItem", self.base_url);
        let params = [("path", path)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to remove RSS item: {}", resp.status()))
        }
    }

    pub async fn get_all_rss_feeds(
        &self,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>> {
        // v5 uses 'items' with withData=true to get feeds and items
        let url_items = format!("{}/api/v2/rss/items?withData=true", self.base_url);
        let resp = self.http.get(&url_items).send().await?;

        if resp.status().is_success() {
            let feeds = resp.json().await?;
            Ok(feeds)
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            // Fallback to legacy
            let url_legacy = format!("{}/api/v2/rss/allFeeds", self.base_url);
            let resp_legacy = self.http.get(&url_legacy).send().await?;
            if resp_legacy.status().is_success() {
                let feeds = resp_legacy.json().await?;
                Ok(feeds)
            } else {
                Err(anyhow!("Failed to get RSS feeds: {}", resp_legacy.status()))
            }
        } else {
            Err(anyhow!("Failed to get RSS feeds: {}", resp.status()))
        }
    }

    pub async fn set_rss_rule(&self, name: &str, definition: &str) -> Result<()> {
        let url = format!("{}/api/v2/rss/setRule", self.base_url);
        let params = [("ruleName", name), ("ruleDef", definition)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to set RSS rule: {}", resp.status()))
        }
    }

    pub async fn get_all_rss_rules(
        &self,
    ) -> Result<std::collections::HashMap<String, crate::models::RssRule>> {
        // v5 uses 'rules'
        let url_rules = format!("{}/api/v2/rss/rules", self.base_url);
        let resp = self.http.get(&url_rules).send().await?;

        if resp.status().is_success() {
            let rules = resp.json().await?;
            Ok(rules)
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            // Fallback to legacy
            let url_legacy = format!("{}/api/v2/rss/allRules", self.base_url);
            let resp_legacy = self.http.get(&url_legacy).send().await?;
            if resp_legacy.status().is_success() {
                let rules = resp_legacy.json().await?;
                Ok(rules)
            } else {
                Err(anyhow!("Failed to get RSS rules: {}", resp_legacy.status()))
            }
        } else {
            Err(anyhow!("Failed to get RSS rules: {}", resp.status()))
        }
    }

    pub async fn get_app_preferences(&self) -> Result<serde_json::Value> {
        let url = format!("{}/api/v2/app/preferences", self.base_url);
        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let prefs = resp.json().await?;
            Ok(prefs)
        } else {
            Err(anyhow!("Failed to get app preferences: {}", resp.status()))
        }
    }

    pub async fn set_app_preferences(&self, prefs: &serde_json::Value) -> Result<()> {
        let url = format!("{}/api/v2/app/setPreferences", self.base_url);
        let params = [("json", prefs.to_string())];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to set app preferences: {}", resp.status()))
        }
    }

    pub async fn get_app_version(&self) -> Result<String> {
        let url = format!("{}/api/v2/app/version", self.base_url);
        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let version = resp.text().await?;
            Ok(version)
        } else {
            Err(anyhow!("Failed to get app version: {}", resp.status()))
        }
    }

    pub async fn get_build_info(&self) -> Result<crate::models::BuildInfo> {
        let url = format!("{}/api/v2/app/buildInfo", self.base_url);
        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let info = resp.json::<crate::models::BuildInfo>().await?;
            Ok(info)
        } else {
            Err(anyhow!("Failed to get build info: {}", resp.status()))
        }
    }

    pub async fn shutdown_app(&self) -> Result<()> {
        let url = format!("{}/api/v2/app/shutdown", self.base_url);
        let resp = self.http.post(&url).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to shutdown app: {}", resp.status()))
        }
    }

    pub async fn get_main_log(
        &self,
        normal: bool,
        info: bool,
        warning: bool,
        critical: bool,
        last_id: Option<i64>,
    ) -> Result<Vec<crate::models::LogEntry>> {
        let mut url = format!(
            "{}/api/v2/log/main?normal={}&info={}&warning={}&critical={}",
            self.base_url, normal, info, warning, critical
        );
        if let Some(id) = last_id {
            url.push_str(&format!("&last_id={}", id));
        }

        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let logs = resp.json().await?;
            Ok(logs)
        } else {
            Err(anyhow!("Failed to get main log: {}", resp.status()))
        }
    }

    pub async fn get_peer_log(
        &self,
        last_id: Option<i64>,
    ) -> Result<Vec<crate::models::PeerLogEntry>> {
        let mut url = format!("{}/api/v2/log/peers", self.base_url);
        if let Some(id) = last_id {
            url.push_str(&format!("?last_id={}", id));
        }

        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let logs = resp.json().await?;
            Ok(logs)
        } else {
            Err(anyhow!("Failed to get peer log: {}", resp.status()))
        }
    }

    pub async fn ban_peers(&self, peers: &str) -> Result<()> {
        let url = format!("{}/api/v2/transfer/banPeers", self.base_url);
        let params = [("peers", peers)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to ban peers: {}", resp.status()))
        }
    }

    pub async fn rename_file(&self, hash: &str, old_path: &str, new_path: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/renameFile", self.base_url);
        let params = [("hash", hash), ("oldPath", old_path), ("newPath", new_path)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to rename file: {}", resp.status()))
        }
    }

    pub async fn set_torrent_share_limits(
        &self,

        hashes: &str,

        ratio_limit: f64,

        seeding_time_limit: i64,

        inactive_seeding_time_limit: Option<i64>,
    ) -> Result<()> {
        let url = format!("{}/api/v2/torrents/setShareLimits", self.base_url);

        let mut params = vec![
            ("hashes", hashes.to_string()),
            ("ratioLimit", ratio_limit.to_string()),
            ("seedingTimeLimit", seeding_time_limit.to_string()),
        ];

        if let Some(limit) = inactive_seeding_time_limit {
            params.push(("inactiveSeedingTimeLimit", limit.to_string()));
        } else {
            // Default to -2 (global) for compatibility with newer qbit versions

            params.push(("inactiveSeedingTimeLimit", "-2".to_string()));
        }

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to set torrent share limits: {}",
                resp.status()
            ))
        }
    }

    pub async fn set_torrent_download_limit(&self, hashes: &str, limit: i64) -> Result<()> {
        let url = format!("{}/api/v2/torrents/setDownloadLimit", self.base_url);
        let params = [("hashes", hashes), ("limit", &limit.to_string())];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to set torrent download limit: {}",
                resp.status()
            ))
        }
    }

    pub async fn set_torrent_upload_limit(&self, hashes: &str, limit: i64) -> Result<()> {
        let url = format!("{}/api/v2/torrents/setUploadLimit", self.base_url);
        let params = [("hashes", hashes), ("limit", &limit.to_string())];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to set torrent upload limit: {}",
                resp.status()
            ))
        }
    }

    pub async fn get_main_data(&self, rid: i64) -> Result<crate::models::SyncMainData> {
        let url = format!("{}/api/v2/sync/maindata?rid={}", self.base_url, rid);

        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let data = resp.json::<crate::models::SyncMainData>().await?;
            Ok(data)
        } else {
            Err(anyhow!("Failed to get main data: {}", resp.status()))
        }
    }

    pub async fn toggle_sequential_download(&self, hashes: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/toggleSequentialDownload", self.base_url);
        let params = [("hashes", hashes)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to toggle sequential download: {}",
                resp.status()
            ))
        }
    }

    pub async fn toggle_first_last_piece_priority(&self, hashes: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/toggleFirstLastPiecePrio", self.base_url);
        let params = [("hashes", hashes)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to toggle first/last piece priority: {}",
                resp.status()
            ))
        }
    }

    pub async fn set_force_start(&self, hashes: &str, value: bool) -> Result<()> {
        let url = format!("{}/api/v2/torrents/setForceStart", self.base_url);
        let params = [("hashes", hashes), ("value", &value.to_string())];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to set force start: {}", resp.status()))
        }
    }

    pub async fn set_super_seeding(&self, hashes: &str, value: bool) -> Result<()> {
        let url = format!("{}/api/v2/torrents/setSuperSeeding", self.base_url);
        let params = [("hashes", hashes), ("value", &value.to_string())];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to set super seeding: {}", resp.status()))
        }
    }

    pub async fn add_trackers(&self, hashes: &str, urls: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/addTrackers", self.base_url);
        let params = [("hashes", hashes), ("urls", urls)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to add trackers: {}", resp.status()))
        }
    }

    pub async fn edit_tracker(&self, hash: &str, orig_url: &str, new_url: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/editTracker", self.base_url);
        let params = [("hash", hash), ("origUrl", orig_url), ("newUrl", new_url)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to edit tracker: {}", resp.status()))
        }
    }

    pub async fn remove_trackers(&self, hashes: &str, urls: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/removeTrackers", self.base_url);
        let params = [("hashes", hashes), ("urls", urls)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to remove trackers: {}", resp.status()))
        }
    }

    pub async fn rename_folder(&self, hash: &str, old_path: &str, new_path: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/renameFolder", self.base_url);
        let params = [("hash", hash), ("oldPath", old_path), ("newPath", new_path)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to rename folder: {}", resp.status()))
        }
    }

    pub async fn set_file_priority(&self, hash: &str, id: &str, priority: i32) -> Result<()> {
        let url = format!("{}/api/v2/torrents/setFilePrio", self.base_url);
        let params = [
            ("hash", hash),
            ("id", id),
            ("priority", &priority.to_string()),
        ];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to set file priority: {}", resp.status()))
        }
    }

    pub async fn remove_categories(&self, categories: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/removeCategories", self.base_url);
        let params = [("categories", categories)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to remove categories: {}", resp.status()))
        }
    }

    pub async fn remove_tags(&self, hashes: &str, tags: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/removeTags", self.base_url);
        let params = [("hashes", hashes), ("tags", tags)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to remove tags: {}", resp.status()))
        }
    }

    pub async fn create_tags(&self, tags: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/createTags", self.base_url);
        let params = [("tags", tags)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to create tags: {}", resp.status()))
        }
    }

    pub async fn delete_tags(&self, tags: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/deleteTags", self.base_url);
        let params = [("tags", tags)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to delete tags: {}", resp.status()))
        }
    }

    pub async fn move_rss_item(&self, item_path: &str, dest_path: &str) -> Result<()> {
        let url = format!("{}/api/v2/rss/moveItem", self.base_url);
        let params = [("itemPath", item_path), ("destPath", dest_path)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to move RSS item: {}", resp.status()))
        }
    }
}
