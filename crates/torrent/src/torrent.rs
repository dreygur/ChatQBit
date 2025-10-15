use qbit_rs::{model::{AddTorrentArg, Credential, Sep, Torrent}, Error, Qbit};
use std::{sync::Arc};

/// Wrapper around qBittorrent API client
#[derive(Clone)]
pub struct TorrentApi {
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
        let password = std::env::var("QBIT_PASSWORD").expect("QBIT_PASSWORD must be set in .env file");\
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
        let arg = AddTorrentArg {
            source: qbit_rs::model::TorrentSource::Urls { urls: Sep::from(urls, "\n") },
            ..Default::default()
        }
        match self.client.add_torrent(arg).await {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::error!("Error adding torrent: {}", err);
                Err(err)
            }
        }
        // match self.client.torrents_add_by_url(urls).await {
        //     Ok(_) => Ok(()),
        //     Err(err) => {
        //         tracing::error!("Error adding torrent: {}", err);
        //         Err(err)
        //     }
        // }
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
