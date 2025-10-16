//! qBittorrent API client wrapper
//!
//! This crate provides a simplified interface to the qBittorrent Web API
//! using the qbit-rs library. It handles authentication, error logging,
//! and provides convenient methods for common torrent operations.

pub mod torrent;
pub mod utils;

pub use torrent::TorrentApi;
pub use utils::{check_duplicates, extract_info_hash, DuplicateCheckResult};
