//! qBittorrent API wrapper implementation

use qbit_rs::{model::{AddTorrentArg, Credential, Sep, Torrent}, Error, Qbit};
use std::sync::Arc;

/// Thread-safe wrapper around the qBittorrent API client
///
/// This struct provides a high-level interface to qBittorrent operations,
/// handling authentication and providing logging for all operations.
///
/// # Examples
///
/// ```no_run
/// use torrent::TorrentApi;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let api = TorrentApi::new();
///     api.login().await?;
///
///     let torrents = api.query().await?;
///     println!("Found {} torrents", torrents.len());
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct TorrentApi {
    /// The underlying qBittorrent client, wrapped in Arc for thread-safety
    pub client: Arc<Qbit>,
    /// HTTP client for fallback API calls (qBittorrent < 5.0 compatibility)
    http_client: reqwest::Client,
    /// Base endpoint URL
    endpoint: String,
    /// Credentials for fallback authentication
    username: String,
    password: String,
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
        let endpoint = std::env::var("QBIT_HOST")
            .expect("QBIT_HOST must be set in .env file, e.g., http://localhost:8080");
        let username = std::env::var("QBIT_USERNAME").expect("QBIT_USERNAME must be set in .env file");
        let password = std::env::var("QBIT_PASSWORD").expect("QBIT_PASSWORD must be set in .env file");
        let credential = Credential::new(username.clone(), password.clone());
        let client = Arc::new(Qbit::new(endpoint.as_str(), credential));
        let http_client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client");
        TorrentApi { client, http_client, endpoint, username, password }
    }

    /// Authenticate with the qBittorrent server
    ///
    /// # Errors
    /// Returns an error if authentication fails
    pub async fn login(&self) -> Result<(), Error> {
        // Login with the main qbit-rs client
        self.client.login(false).await.map_err(|e| {
            tracing::error!("Failed to login to qBittorrent: {}", e);
            e
        })?;

        // Also login with the fallback http_client for v4.x API compatibility
        let login_url = format!("{}/api/v2/auth/login", self.endpoint);
        let _ = self.http_client
            .post(&login_url)
            .form(&[("username", &self.username), ("password", &self.password)])
            .send()
            .await;

        Ok(())
    }

    pub async fn query(&self) -> Result<Vec<Torrent>, Error> {
        tracing::info!("Querying torrents from qBittorrent");
        let arg = qbit_rs::model::GetTorrentListArg {
            filter: None,
            category: None,
            tag: None,
            sort: None,
            reverse: None,
            limit: Some(10),
            offset: None,
            hashes: None,
        };

        match self.client.get_torrent_list(arg).await {
            Ok(resp) => Ok(resp),
            Err(err) => {
                tracing::error!("Error querying torrents: {}", err);
                Err(err)
            }
        }
    }

    /// Add torrents by URL (magnet links or HTTP URLs)
    ///
    /// # Arguments
    /// * `urls` - Array of magnet links or torrent URLs to add
    ///
    /// # Returns
    /// * `Ok(())` - Torrents added successfully
    /// * `Err(Error)` - Failed to add torrents
    pub async fn magnet(&self, urls: &[String]) -> Result<(), Error> {
        tracing::info!("Adding torrent with URLs: {:?}", urls);
        let url_objects: Vec<_> = urls.iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        let arg = AddTorrentArg {
            source: qbit_rs::model::TorrentSource::Urls { urls: Sep::from(url_objects) },
            ..Default::default()
        };
        match self.client.add_torrent(arg).await {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::error!("Error adding torrent: {}", err);
                Err(err)
            }
        }
    }

    /// Add torrent from file data
    ///
    /// # Arguments
    /// * `filename` - Original filename of the .torrent file
    /// * `file_data` - Raw bytes of the .torrent file
    ///
    /// # Returns
    /// * `Ok(())` - Torrent added successfully
    /// * `Err(Error)` - Failed to add torrent
    pub async fn add_torrent_file(&self, filename: &str, file_data: Vec<u8>) -> Result<(), Error> {
        tracing::info!("Adding torrent from file: {} ({} bytes)", filename, file_data.len());

        let torrent_file = qbit_rs::model::TorrentFile {
            filename: filename.to_string(),
            data: file_data,
        };

        let arg = AddTorrentArg {
            source: qbit_rs::model::TorrentSource::TorrentFiles {
                torrents: vec![torrent_file],
            },
            ..Default::default()
        };

        match self.client.add_torrent(arg).await {
            Ok(_) => {
                tracing::info!("Successfully added torrent file: {}", filename);
                Ok(())
            }
            Err(err) => {
                tracing::error!("Error adding torrent file {}: {}", filename, err);
                Err(err)
            }
        }
    }

    /// Check if torrents are duplicates before adding
    ///
    /// # Arguments
    /// * `urls` - URLs to check for duplicates
    ///
    /// # Returns
    /// * `Ok(DuplicateCheckResult)` - Result of duplicate check
    /// * `Err(Error)` - Failed to fetch existing torrents
    pub async fn check_duplicates(
        &self,
        urls: &[String],
    ) -> Result<crate::utils::DuplicateCheckResult, Error> {
        tracing::debug!("Checking for duplicate torrents");

        // Get all existing torrents (no limit)
        let arg = qbit_rs::model::GetTorrentListArg {
            filter: None,
            category: None,
            tag: None,
            sort: None,
            reverse: None,
            limit: None, // Get all torrents
            offset: None,
            hashes: None,
        };

        let existing_torrents = self.client.get_torrent_list(arg).await?;

        // Build set of existing hashes
        let existing_hashes: std::collections::HashSet<String> = existing_torrents
            .iter()
            .filter_map(|t| t.hash.as_ref().map(|h| h.to_lowercase()))
            .collect();

        tracing::debug!("Found {} existing torrents", existing_hashes.len());

        Ok(crate::utils::check_duplicates(urls, &existing_hashes))
    }

    pub async fn get_torrent_info(&self, hash: &str) -> Result<qbit_rs::model::TorrentProperty, Error> {
        tracing::info!("Getting torrent properties for hash: {}", hash);
        self.client.get_torrent_properties(hash).await
    }

    /// Resume/start torrents (compatible with qBittorrent v4.x and v5.x)
    pub async fn start_torrents(&self, hash: &str) -> Result<(), Error> {
        tracing::info!("Starting/resuming torrents: {}", hash);
        let hashes = vec![hash.to_string()];

        // Try v5.0 API first (torrents/start)
        match self.client.start_torrents(hashes).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Fallback to v4.x API (torrents/resume) if 404
                tracing::debug!("start_torrents failed, trying resume fallback: {}", e);
                self.legacy_resume_torrents(hash).await
            }
        }
    }

    /// Stop/pause torrents (compatible with qBittorrent v4.x and v5.x)
    pub async fn stop_torrents(&self, hash: &str) -> Result<(), Error> {
        tracing::info!("Stopping/pausing torrents: {}", hash);
        let hashes = vec![hash.to_string()];

        // Try v5.0 API first (torrents/stop)
        match self.client.stop_torrents(hashes).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Fallback to v4.x API (torrents/pause) if 404
                tracing::debug!("stop_torrents failed, trying pause fallback: {}", e);
                self.legacy_pause_torrents(hash).await
            }
        }
    }

    /// Fallback for qBittorrent < 5.0: use torrents/resume endpoint
    async fn legacy_resume_torrents(&self, hash: &str) -> Result<(), Error> {
        let url = format!("{}/api/v2/torrents/resume", self.endpoint);
        let resp = self.http_client
            .post(&url)
            .form(&[("hashes", hash)])
            .send()
            .await
            .map_err(Error::HttpError)?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Error::BadResponse { explain: "Resume failed" })
        }
    }

    /// Fallback for qBittorrent < 5.0: use torrents/pause endpoint
    async fn legacy_pause_torrents(&self, hash: &str) -> Result<(), Error> {
        let url = format!("{}/api/v2/torrents/pause", self.endpoint);
        let resp = self.http_client
            .post(&url)
            .form(&[("hashes", hash)])
            .send()
            .await
            .map_err(Error::HttpError)?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Error::BadResponse { explain: "Pause failed" })
        }
    }

    pub async fn delete_torrents(&self, hash: &str, delete_files: bool) -> Result<(), Error> {
        tracing::info!("Deleting torrents: {} (delete files: {})", hash, delete_files);
        let hashes = vec![hash.to_string()];
        self.client.delete_torrents(hashes, delete_files).await
    }

    pub async fn recheck_torrents(&self, hash: &str) -> Result<(), Error> {
        tracing::info!("Rechecking torrents: {}", hash);
        let hashes = vec![hash.to_string()];
        self.client.recheck_torrents(hashes).await
    }

    pub async fn reannounce_torrents(&self, hash: &str) -> Result<(), Error> {
        tracing::info!("Reannouncing torrents: {}", hash);
        let hashes = vec![hash.to_string()];
        self.client.reannounce_torrents(hashes).await
    }

    pub async fn set_top_priority(&self, hash: &str) -> Result<(), Error> {
        tracing::info!("Setting top priority for: {}", hash);
        let hashes = vec![hash.to_string()];
        self.client.maximal_priority(hashes).await
    }

    pub async fn set_bottom_priority(&self, hash: &str) -> Result<(), Error> {
        tracing::info!("Setting bottom priority for: {}", hash);
        let hashes = vec![hash.to_string()];
        self.client.minimal_priority(hashes).await
    }

    pub async fn get_transfer_info(&self) -> Result<qbit_rs::model::TransferInfo, Error> {
        tracing::info!("Getting transfer info");
        self.client.get_transfer_info().await
    }

    pub async fn get_version(&self) -> Result<String, Error> {
        tracing::info!("Getting qBittorrent version");
        self.client.get_version().await
    }

    pub async fn get_categories(&self) -> Result<std::collections::HashMap<String, qbit_rs::model::Category>, Error> {
        tracing::info!("Getting categories");
        self.client.get_categories().await
    }

    pub async fn get_tags(&self) -> Result<Vec<String>, Error> {
        tracing::info!("Getting all tags");
        self.client.get_all_tags().await
    }

    pub async fn get_download_limit(&self) -> Result<u64, Error> {
        tracing::info!("Getting global download limit");
        self.client.get_download_limit().await
    }

    pub async fn get_upload_limit(&self) -> Result<u64, Error> {
        tracing::info!("Getting global upload limit");
        self.client.get_upload_limit().await
    }

    pub async fn set_download_limit(&self, limit: u64) -> Result<(), Error> {
        tracing::info!("Setting global download limit to: {}", limit);
        self.client.set_download_limit(limit).await
    }

    pub async fn set_upload_limit(&self, limit: u64) -> Result<(), Error> {
        tracing::info!("Setting global upload limit to: {}", limit);
        self.client.set_upload_limit(limit).await
    }

    /// Get list of files in a torrent
    ///
    /// # Arguments
    /// * `hash` - Torrent hash
    ///
    /// # Returns
    /// * `Ok(Vec<TorrentContent>)` - List of files with metadata
    /// * `Err(Error)` - Failed to fetch file list
    pub async fn get_torrent_files(&self, hash: &str) -> Result<Vec<qbit_rs::model::TorrentContent>, Error> {
        tracing::info!("Getting file list for torrent: {}", hash);
        self.client.get_torrent_contents(hash, None).await
    }

    /// Set priority for specific files in a torrent
    ///
    /// # Arguments
    /// * `hash` - Torrent hash
    /// * `file_ids` - List of file indices to set priority for
    /// * `priority` - Priority level (use qbit_rs::model::Priority enum)
    ///
    /// # Returns
    /// * `Ok(())` - Priority set successfully
    /// * `Err(Error)` - Failed to set priority
    pub async fn set_file_priority(&self, hash: &str, file_ids: Vec<i64>, priority: qbit_rs::model::Priority) -> Result<(), Error> {
        tracing::info!("Setting file priority for torrent {}: {:?} -> {:?}", hash, file_ids, priority);
        self.client.set_file_priority(hash, file_ids, priority).await
    }

    /// Toggle sequential download mode for a torrent
    ///
    /// When enabled, pieces are downloaded in order (better for streaming)
    ///
    /// # Arguments
    /// * `hash` - Torrent hash
    ///
    /// # Returns
    /// * `Ok(())` - Sequential mode toggled successfully
    /// * `Err(Error)` - Failed to toggle sequential mode
    pub async fn toggle_sequential_download(&self, hash: &str) -> Result<(), Error> {
        tracing::info!("Toggling sequential download for torrent: {}", hash);
        let hashes = vec![hash.to_string()];
        self.client.toggle_sequential_download(hashes).await
    }

    /// Toggle first/last piece priority for a torrent
    ///
    /// When enabled, downloads first and last pieces first (useful for media file headers)
    ///
    /// # Arguments
    /// * `hash` - Torrent hash
    ///
    /// # Returns
    /// * `Ok(())` - First/last piece priority toggled successfully
    /// * `Err(Error)` - Failed to toggle priority
    pub async fn toggle_first_last_piece_priority(&self, hash: &str) -> Result<(), Error> {
        tracing::info!("Toggling first/last piece priority for torrent: {}", hash);
        let hashes = vec![hash.to_string()];
        self.client.toggle_first_last_piece_priority(hashes).await
    }

    /// Get the default save path from qBittorrent preferences
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - Default download directory
    /// * `Err(Error)` - Failed to fetch preferences
    pub async fn get_default_save_path(&self) -> Result<std::path::PathBuf, Error> {
        tracing::info!("Getting default save path");
        self.client.get_default_save_path().await
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;

    #[tokio::test]
    async fn test_login() {
        dotenv().ok();
        let api = TorrentApi::new();
        let result = api.login().await;
        assert!(result.is_ok(), "Login failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_query() {
        dotenv().ok();
        let api = TorrentApi::new();
        let login_result = api.login().await;
        assert!(login_result.is_ok(), "Login failed: {:?}", login_result.err());

        // Now test the query method
        let resp = api.query().await;
        println!("{:?}", resp);
        assert!(resp.is_ok(), "Query failed: {:?}", resp.err());
    }
}
