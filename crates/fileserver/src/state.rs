//! Server state management for tracking active streams

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use torrent::TorrentApi;

/// Information about an active stream
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// Torrent hash
    pub torrent_hash: String,
    /// File index within the torrent
    pub file_index: usize,
    /// Full path to the file on disk
    pub file_path: PathBuf,
    /// Original filename
    pub filename: String,
    /// When this stream was registered
    pub created_at: DateTime<Utc>,
}

/// Server state holding all active streams
#[derive(Clone)]
pub struct ServerState {
    /// Active streams mapped by token
    streams: Arc<RwLock<HashMap<String, StreamInfo>>>,
    /// Base download path from qBittorrent
    download_path: PathBuf,
    /// Secret for token generation
    secret: String,
    /// qBittorrent API client for querying file locations
    torrent_api: TorrentApi,
}

impl ServerState {
    /// Create new server state
    ///
    /// # Arguments
    /// * `download_path` - Base directory where qBittorrent saves files
    /// * `secret` - Secret key for token generation
    /// * `torrent_api` - qBittorrent API client for querying file locations
    pub fn new(download_path: PathBuf, secret: String, torrent_api: TorrentApi) -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            download_path,
            secret,
            torrent_api,
        }
    }

    /// Register a new stream
    ///
    /// # Arguments
    /// * `token` - Unique token for this stream
    /// * `info` - Stream information
    pub fn register_stream(&self, token: String, info: StreamInfo) {
        let mut streams = self.streams.write().unwrap_or_else(|e| e.into_inner());
        streams.insert(token, info);
    }

    /// Get stream information by token
    ///
    /// # Arguments
    /// * `token` - Stream token
    ///
    /// # Returns
    /// * `Some(StreamInfo)` if found and not expired, `None` otherwise
    pub fn get_stream(&self, token: &str) -> Option<StreamInfo> {
        let streams = self.streams.read().unwrap_or_else(|e| e.into_inner());
        streams.get(token).cloned()
    }

    /// Get stream information by token with expiration check
    ///
    /// # Arguments
    /// * `token` - Stream token
    /// * `max_age_hours` - Maximum age in hours before considering expired
    ///
    /// # Returns
    /// * `Some(StreamInfo)` if found and not expired, `None` otherwise
    pub fn get_stream_if_valid(&self, token: &str, max_age_hours: i64) -> Option<StreamInfo> {
        let streams = self.streams.read().unwrap_or_else(|e| e.into_inner());
        streams.get(token).and_then(|info| {
            let age = Utc::now().signed_duration_since(info.created_at);
            if age.num_hours() < max_age_hours {
                Some(info.clone())
            } else {
                tracing::debug!("Stream token expired: {} hours old", age.num_hours());
                None
            }
        })
    }

    /// Remove a stream registration
    ///
    /// # Arguments
    /// * `token` - Stream token to remove
    pub fn unregister_stream(&self, token: &str) {
        let mut streams = self.streams.write().unwrap_or_else(|e| e.into_inner());
        streams.remove(token);
    }

    /// Get the download path
    pub fn download_path(&self) -> &PathBuf {
        &self.download_path
    }

    /// Get the secret
    pub fn secret(&self) -> &str {
        &self.secret
    }

    /// Get count of active streams
    pub fn stream_count(&self) -> usize {
        let streams = self.streams.read().unwrap_or_else(|e| e.into_inner());
        streams.len()
    }

    /// Clean up old streams (older than specified duration)
    ///
    /// # Arguments
    /// * `max_age_hours` - Maximum age in hours before cleanup
    ///
    /// # Returns
    /// * Number of streams cleaned up
    pub fn cleanup_old_streams(&self, max_age_hours: i64) -> usize {
        let mut streams = self.streams.write().unwrap_or_else(|e| e.into_inner());
        let now = Utc::now();
        let initial_count = streams.len();

        streams.retain(|_, info| {
            let age = now.signed_duration_since(info.created_at);
            age.num_hours() < max_age_hours
        });

        initial_count - streams.len()
    }

    /// Query qBittorrent for the current file path
    ///
    /// This is used as a fallback when the cached file path doesn't exist.
    ///
    /// # Arguments
    /// * `torrent_hash` - Hash of the torrent
    /// * `file_index` - Index of the file within the torrent
    /// * `filename` - Name of the file (for logging)
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - Current file path from qBittorrent
    /// * `Err(String)` - Error message if query fails
    pub async fn query_file_path(
        &self,
        torrent_hash: &str,
        file_index: usize,
        filename: &str,
    ) -> Result<PathBuf, String> {
        tracing::info!(
            "Querying qBittorrent for file location: {} (index: {})",
            filename,
            file_index
        );

        // Get torrent properties for save path
        let torrent_info = self
            .torrent_api
            .get_torrent_info(torrent_hash)
            .await
            .map_err(|e| format!("Failed to get torrent info: {}", e))?;

        // Get file list
        let files = self
            .torrent_api
            .get_torrent_files(torrent_hash)
            .await
            .map_err(|e| format!("Failed to get torrent files: {}", e))?;

        // Find the file by index
        let file = files
            .get(file_index)
            .ok_or_else(|| format!("File index {} not found in torrent", file_index))?;

        // Construct file path: save_path + file.name
        let save_path = torrent_info.save_path.unwrap_or_else(|| ".".to_string());
        let save_path_buf = PathBuf::from(save_path);
        let file_path = save_path_buf.join(&file.name);

        tracing::info!("Resolved file path from qBittorrent: {}", file_path.display());

        Ok(file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_state() -> ServerState {
        dotenv::dotenv().ok();
        let torrent_api = TorrentApi::new();
        ServerState::new(PathBuf::from("/downloads"), "secret".to_string(), torrent_api)
    }

    fn create_test_stream_info(hash: &str, filename: &str) -> StreamInfo {
        StreamInfo {
            torrent_hash: hash.to_string(),
            file_index: 0,
            file_path: PathBuf::from(format!("/downloads/{}", filename)),
            filename: filename.to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_state_operations() {
        let state = create_test_state();
        let info = create_test_stream_info("abc123", "video.mp4");

        state.register_stream("token1".to_string(), info);
        assert_eq!(state.stream_count(), 1);

        let retrieved = state.get_stream("token1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().torrent_hash, "abc123");

        state.unregister_stream("token1");
        assert_eq!(state.stream_count(), 0);
    }

    #[test]
    fn test_get_stream_nonexistent() {
        let state = create_test_state();
        assert!(state.get_stream("nonexistent").is_none());
    }

    #[test]
    fn test_multiple_streams() {
        let state = create_test_state();

        state.register_stream("token1".to_string(), create_test_stream_info("hash1", "file1.mp4"));
        state.register_stream("token2".to_string(), create_test_stream_info("hash2", "file2.mp4"));
        state.register_stream("token3".to_string(), create_test_stream_info("hash3", "file3.mp4"));

        assert_eq!(state.stream_count(), 3);

        // Check each stream
        assert_eq!(state.get_stream("token1").unwrap().torrent_hash, "hash1");
        assert_eq!(state.get_stream("token2").unwrap().torrent_hash, "hash2");
        assert_eq!(state.get_stream("token3").unwrap().torrent_hash, "hash3");

        // Remove one
        state.unregister_stream("token2");
        assert_eq!(state.stream_count(), 2);
        assert!(state.get_stream("token2").is_none());
    }

    #[test]
    fn test_get_stream_if_valid_not_expired() {
        let state = create_test_state();
        let info = create_test_stream_info("abc123", "video.mp4");

        state.register_stream("token1".to_string(), info);

        // Fresh stream should be valid
        let result = state.get_stream_if_valid("token1", 24);
        assert!(result.is_some());
        assert_eq!(result.unwrap().torrent_hash, "abc123");
    }

    #[test]
    fn test_get_stream_if_valid_expired() {
        let state = create_test_state();

        // Create an old stream (25 hours old)
        let old_time = Utc::now() - Duration::hours(25);
        let info = StreamInfo {
            torrent_hash: "abc123".to_string(),
            file_index: 0,
            file_path: PathBuf::from("/downloads/video.mp4"),
            filename: "video.mp4".to_string(),
            created_at: old_time,
        };

        state.register_stream("token1".to_string(), info);

        // Stream should be expired with 24 hour limit
        let result = state.get_stream_if_valid("token1", 24);
        assert!(result.is_none());

        // But still accessible via get_stream
        assert!(state.get_stream("token1").is_some());
    }

    #[test]
    fn test_get_stream_if_valid_nonexistent() {
        let state = create_test_state();
        assert!(state.get_stream_if_valid("nonexistent", 24).is_none());
    }

    #[test]
    fn test_cleanup_old_streams() {
        let state = create_test_state();

        // Add fresh stream
        state.register_stream("fresh".to_string(), create_test_stream_info("hash1", "fresh.mp4"));

        // Add old stream (25 hours old)
        let old_time = Utc::now() - Duration::hours(25);
        let old_info = StreamInfo {
            torrent_hash: "old_hash".to_string(),
            file_index: 0,
            file_path: PathBuf::from("/downloads/old.mp4"),
            filename: "old.mp4".to_string(),
            created_at: old_time,
        };
        state.register_stream("old".to_string(), old_info);

        assert_eq!(state.stream_count(), 2);

        // Cleanup with 24 hour threshold
        let cleaned = state.cleanup_old_streams(24);
        assert_eq!(cleaned, 1);
        assert_eq!(state.stream_count(), 1);

        // Fresh stream should still exist
        assert!(state.get_stream("fresh").is_some());
        // Old stream should be gone
        assert!(state.get_stream("old").is_none());
    }

    #[test]
    fn test_cleanup_no_old_streams() {
        let state = create_test_state();

        state.register_stream("token1".to_string(), create_test_stream_info("hash1", "file1.mp4"));
        state.register_stream("token2".to_string(), create_test_stream_info("hash2", "file2.mp4"));

        let cleaned = state.cleanup_old_streams(24);
        assert_eq!(cleaned, 0);
        assert_eq!(state.stream_count(), 2);
    }

    #[test]
    fn test_download_path() {
        dotenv::dotenv().ok();
        let torrent_api = TorrentApi::new();
        let state = ServerState::new(PathBuf::from("/custom/path"), "secret".to_string(), torrent_api);
        assert_eq!(state.download_path(), &PathBuf::from("/custom/path"));
    }

    #[test]
    fn test_secret() {
        dotenv::dotenv().ok();
        let torrent_api = TorrentApi::new();
        let state = ServerState::new(PathBuf::from("/downloads"), "my_secret".to_string(), torrent_api);
        assert_eq!(state.secret(), "my_secret");
    }

    #[test]
    fn test_overwrite_stream() {
        let state = create_test_state();

        let info1 = create_test_stream_info("hash1", "file1.mp4");
        let info2 = create_test_stream_info("hash2", "file2.mp4");

        state.register_stream("token1".to_string(), info1);
        assert_eq!(state.get_stream("token1").unwrap().torrent_hash, "hash1");

        // Overwrite with new info
        state.register_stream("token1".to_string(), info2);
        assert_eq!(state.get_stream("token1").unwrap().torrent_hash, "hash2");
        assert_eq!(state.stream_count(), 1);
    }
}
