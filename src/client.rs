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
}
