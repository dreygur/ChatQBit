use qbit_api_rs::{client::QbitClient, error::ClientError};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TorrentApi {
  pub client: Arc<QbitClient>,
}

impl TorrentApi {
  pub fn new(host: &str, username: &str, password: &str) -> Self {
    let client = Arc::new(QbitClient::new_with_user_pwd(host, username, password).unwrap());
    TorrentApi { client }
  }

  pub async fn login(&self) -> Result<String, ClientError> {
    self.client.auth_login().await
  }
}
