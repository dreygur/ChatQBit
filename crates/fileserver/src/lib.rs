//! HTTP File Server for streaming torrent files
//!
//! This crate provides an HTTP server that serves files from the qBittorrent
//! download directory with proper range request support for video streaming.

mod server;
mod state;
mod token;
mod tunnel;

pub use server::FileServerApi;
pub use state::{StreamInfo, ServerState};
pub use token::generate_stream_token;
pub use tunnel::{TunnelProvider, TunnelInfo, start_tunnel};

/// Result type alias for file server operations
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
