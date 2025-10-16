//! qBittorrent API wrapper implementation

use qbit_rs::{model::{AddTorrentArg, Credential, Sep, Torrent}, Error, Qbit};
use std::{sync::Arc};

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
        let credential = Credential::new(username, password);
        let client = Arc::new(Qbit::new(endpoint.as_str(), credential));
        TorrentApi { client }
    }

    /// Authenticate with the qBittorrent server
    ///
    /// # Errors
    /// Returns an error if authentication fails
    pub async fn login(&self) -> Result<(), Error> {
        self.client.login(false).await.map_err(|e| {
            tracing::error!("Failed to login to qBittorrent: {}", e);
            e
        })
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

    pub async fn get_torrent_info(&self, hash: &str) -> Result<qbit_rs::model::TorrentProperty, Error> {
        tracing::info!("Getting torrent properties for hash: {}", hash);
        self.client.get_torrent_properties(hash).await
    }

    pub async fn start_torrents(&self, hash: &str) -> Result<(), Error> {
        tracing::info!("Starting torrents: {}", hash);
        let hashes = vec![hash.to_string()];
        self.client.start_torrents(hashes).await
    }

    pub async fn stop_torrents(&self, hash: &str) -> Result<(), Error> {
        tracing::info!("Stopping torrents: {}", hash);
        let hashes = vec![hash.to_string()];
        self.client.stop_torrents(hashes).await
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
