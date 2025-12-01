//! HTTP server implementation with range request support

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::state::ServerState;
use crate::token::verify_stream_token;

/// Stream token expiration time in hours
const STREAM_TOKEN_EXPIRY_HOURS: i64 = 24;

/// File server API for managing the HTTP server
#[derive(Clone)]
pub struct FileServerApi {
    state: ServerState,
    base_url: String,
}

impl FileServerApi {
    /// Create a new file server API
    ///
    /// # Arguments
    /// * `download_path` - Base directory where qBittorrent saves files
    /// * `secret` - Secret key for token generation
    /// * `base_url` - Base URL for generating stream links (e.g., http://localhost:8081)
    /// * `torrent_api` - qBittorrent API client for querying file locations
    pub fn new(download_path: PathBuf, secret: String, base_url: String, torrent_api: torrent::TorrentApi) -> Self {
        let state = ServerState::new(download_path, secret, torrent_api);
        Self { state, base_url }
    }

    /// Get the server state
    pub fn state(&self) -> &ServerState {
        &self.state
    }

    /// Get the base URL for generating links
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Create the axum router with all routes configured
    pub fn router(&self) -> Router {
        Router::new()
            .route("/stream/:token/:filename", get(stream_file))
            .route("/health", get(health_check))
            .with_state(self.state.clone())
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
    }

    /// Start the file server
    ///
    /// # Arguments
    /// * `host` - Host to bind to (e.g., "0.0.0.0")
    /// * `port` - Port to bind to (e.g., 8081)
    pub async fn serve(self, host: &str, port: u16) -> crate::Result<()> {
        let addr = format!("{}:{}", host, port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        tracing::info!("File server listening on {}", addr);

        // Spawn background task to clean up old streams every hour
        let cleanup_state = self.state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                let cleaned = cleanup_state.cleanup_old_streams(24); // Remove streams older than 24 hours
                if cleaned > 0 {
                    tracing::info!("Cleaned up {} expired streams", cleaned);
                }
            }
        });

        axum::serve(listener, self.router()).await?;

        Ok(())
    }
}

/// Health check endpoint
async fn health_check(State(state): State<ServerState>) -> impl IntoResponse {
    let stream_count = state.stream_count();
    (
        StatusCode::OK,
        format!("File server running. Active streams: {}", stream_count),
    )
}

/// Stream file handler with range request support
async fn stream_file(
    State(state): State<ServerState>,
    Path((token, _filename)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    // Get stream info from state (with expiration check)
    let stream_info = state
        .get_stream_if_valid(&token, STREAM_TOKEN_EXPIRY_HOURS)
        .ok_or_else(|| AppError::NotFound("Stream not found or expired".to_string()))?;

    // Verify token
    if !verify_stream_token(&token, &stream_info.torrent_hash, stream_info.file_index, state.secret()) {
        return Err(AppError::Unauthorized("Invalid token".to_string()));
    }

    // Try to open file from cached path
    let mut file_path = stream_info.file_path.clone();
    let mut file = File::open(&file_path).await;

    // If file not found, query qBittorrent for current location (failsafe)
    if let Err(ref e) = file {
        if e.kind() == std::io::ErrorKind::NotFound {
            tracing::warn!(
                "File not found at cached path: {}. Querying qBittorrent for current location...",
                file_path.display()
            );

            // Query qBittorrent for current file path
            match state
                .query_file_path(&stream_info.torrent_hash, stream_info.file_index, &stream_info.filename)
                .await
            {
                Ok(new_path) => {
                    tracing::info!("Resolved new file path: {}", new_path.display());
                    file_path = new_path;
                    file = File::open(&file_path).await;
                }
                Err(query_err) => {
                    tracing::error!("Failed to query file path from qBittorrent: {}", query_err);
                    return Err(AppError::Internal(format!(
                        "File not found and failed to query qBittorrent: {}",
                        query_err
                    )));
                }
            }
        }
    }

    // Final check if file opened successfully
    let file = file.map_err(|e| AppError::Internal(format!("Failed to open file: {}", e)))?;

    // Get file metadata
    let metadata = file
        .metadata()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get file metadata: {}", e)))?;
    let file_size = metadata.len();

    // Detect MIME type
    let mime_type = mime_guess::from_path(file_path)
        .first_or_octet_stream()
        .to_string();

    // Handle range requests
    if let Some(range_header) = headers.get(header::RANGE) {
        return handle_range_request(file, file_size, range_header, &mime_type).await;
    }

    // Full file response
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::CONTENT_LENGTH, file_size)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(body)
        .map_err(|e| AppError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// Handle HTTP range requests for video seeking
async fn handle_range_request(
    mut file: File,
    file_size: u64,
    range_header: &header::HeaderValue,
    mime_type: &str,
) -> Result<Response, AppError> {
    let range_str = range_header
        .to_str()
        .map_err(|_| AppError::BadRequest("Invalid range header".to_string()))?;

    // Parse range header (e.g., "bytes=0-1023")
    let range_str = range_str
        .strip_prefix("bytes=")
        .ok_or_else(|| AppError::BadRequest("Invalid range format".to_string()))?;

    let parts: Vec<&str> = range_str.split('-').collect();
    if parts.len() != 2 {
        return Err(AppError::BadRequest("Invalid range format".to_string()));
    }

    let start: u64 = parts[0]
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid range start".to_string()))?;

    let end: u64 = if parts[1].is_empty() {
        file_size - 1
    } else {
        parts[1]
            .parse::<u64>()
            .map_err(|_| AppError::BadRequest("Invalid range end".to_string()))?
            .min(file_size - 1)
    };

    if start > end || start >= file_size {
        return Err(AppError::RangeNotSatisfiable(file_size));
    }

    let content_length = end - start + 1;

    // Seek to start position
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|e| AppError::Internal(format!("Failed to seek file: {}", e)))?;

    // Read the requested range
    let mut buffer = vec![0; content_length as usize];
    file.read_exact(&mut buffer)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read file: {}", e)))?;

    let response = Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::CONTENT_LENGTH, content_length)
        .header(
            header::CONTENT_RANGE,
            format!("bytes {}-{}/{}", start, end, file_size),
        )
        .header(header::ACCEPT_RANGES, "bytes")
        .body(Body::from(buffer))
        .map_err(|e| AppError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// Application error types
#[derive(Debug)]
enum AppError {
    NotFound(String),
    Unauthorized(String),
    BadRequest(String),
    Internal(String),
    RangeNotSatisfiable(u64),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::RangeNotSatisfiable(size) => (
                StatusCode::RANGE_NOT_SATISFIABLE,
                format!("Range not satisfiable. File size: {}", size),
            ),
        };

        (status, message).into_response()
    }
}
