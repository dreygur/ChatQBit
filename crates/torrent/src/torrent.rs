use qbit_api_rs::{client::QbitClient, error::ClientError};
use std::sync::Arc;

/// Wrapper around qBittorrent API client
#[derive(Debug, Clone)]
pub struct TorrentApi {
    pub client: Arc<QbitClient>,
}

impl Default for TorrentApi {
    fn default() -> Self {
        Self::new()
    }
}

impl TorrentApi {
    /// Create a new TorrentApi instance from environment variables
    ///
    /// # Panics
    /// Panics if the required environment variables are not set
    pub fn new() -> Self {
        let client = Arc::new(QbitClient::new_from_env().unwrap());
        TorrentApi { client }
    }

    /// Authenticate with the qBittorrent server
    ///
    /// # Errors
    /// Returns an error if authentication fails
    pub async fn login(&self) -> Result<String, ClientError> {
        self.client.auth_login().await
    }
}
