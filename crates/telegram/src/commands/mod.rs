//! Command handlers for the Telegram bot
//!
//! This module contains all command handler functions organized by category:
//! - `basic`: Start, help, cancel, menu commands
//! - `torrent`: Add/list/info commands for torrents
//! - `control`: Resume, pause, delete, recheck commands
//! - `config`: Speed limits, categories, tags, version
//! - `stream`: Streaming and sequential download commands

mod basic;
mod config;
mod control;
mod stream;
mod torrent;

pub use basic::*;
pub use config::*;
pub use control::*;
pub use stream::*;
pub use torrent::*;
