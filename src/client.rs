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
    ) -> Self {
        // Enable cookie store for session management (SID)
        let http = Client::builder()
            .cookie_store(true)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http,
            base_url: base_url.into(),
            username: Some(username.into()),
            password: Some(password.into()),
        }
    }

    pub fn new_no_auth(base_url: impl Into<String>) -> Self {
        let http = Client::builder()
            .cookie_store(true)
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

        let resp = self.http.post(&url).form(&params).send().await?;

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

    pub async fn get_torrent_list(&self) -> Result<Vec<crate::models::Torrent>> {
        let url = format!("{}/api/v2/torrents/info", self.base_url);

        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let torrents = resp.json::<Vec<crate::models::Torrent>>().await?;
            Ok(torrents)
        } else {
            Err(anyhow!("Failed to get torrent list: {}", resp.status()))
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
        let url = format!("{}/api/v2/torrents/pause", self.base_url);
        let params = [("hashes", hashes)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to pause torrents: {}", resp.status()))
        }
    }

    pub async fn resume_torrents(&self, hashes: &str) -> Result<()> {
        let url = format!("{}/api/v2/torrents/resume", self.base_url);
        let params = [("hashes", hashes)];

        let resp = self.http.post(&url).form(&params).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to resume torrents: {}", resp.status()))
        }
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

    pub async fn get_torrent_properties(&self, hash: &str) -> Result<crate::models::TorrentProperties> {
        let url = format!("{}/api/v2/torrents/properties?hash={}", self.base_url, hash);

        let resp = self.http.get(&url).send().await?;

        if resp.status().is_success() {
            let props = resp.json::<crate::models::TorrentProperties>().await?;
            Ok(props)
        } else {
            Err(anyhow!("Failed to get torrent properties: {}", resp.status()))
        }
    }
}
