//! Constants used throughout the telegram bot

/// Default hash truncation length for display
pub const HASH_DISPLAY_LENGTH: usize = 8;

/// Maximum number of torrents to display in list command
pub const MAX_TORRENTS_DISPLAY: usize = 50;

/// Enable duplicate torrent detection
/// Set to false to disable duplicate checking and allow re-adding torrents
pub const ENABLE_DUPLICATE_CHECK: bool = true;

/// Emoji constants for consistent UI
pub mod emoji {
    pub const SUCCESS: &str = "✅";
    pub const ERROR: &str = "❌";
    pub const INFO: &str = "📊";
    pub const FOLDER: &str = "📁";
    pub const DOWNLOAD: &str = "📥";
    pub const UPLOAD: &str = "📤";
    pub const SPEED: &str = "⚡";
    pub const TAG: &str = "🏷️";
    pub const CATEGORY: &str = "📂";
    pub const TOOL: &str = "🔧";
}

/// Usage messages for commands
pub mod usage {
    pub const INFO: &str = "Usage: /info <torrent_hash>\n\nTip: Use /list to get full torrent hashes. Tap the monospace hash to copy it.";
    pub const RESUME: &str = "Usage: /resume <torrent_hash> or /resume all\n\nTip: Get the hash from /list command.";
    pub const PAUSE: &str = "Usage: /pause <torrent_hash> or /pause all\n\nTip: Get the hash from /list command.";
    pub const DELETE: &str = "Usage: /delete <torrent_hash>\n\nTip: Get the hash from /list command.";
    pub const DELETE_DATA: &str = "Usage: /deletedata <torrent_hash>\n\nTip: Get the hash from /list command.";
    pub const RECHECK: &str = "Usage: /recheck <torrent_hash>\n\nTip: Get the hash from /list command.";
    pub const REANNOUNCE: &str = "Usage: /reannounce <torrent_hash>\n\nTip: Get the hash from /list command.";
    pub const TOP_PRIO: &str = "Usage: /topprio <torrent_hash>\n\nTip: Get the hash from /list command.";
    pub const BOTTOM_PRIO: &str = "Usage: /bottomprio <torrent_hash>\n\nTip: Get the hash from /list command.";
    pub const SET_DL_LIMIT: &str = "Usage: /setdllimit <bytes_per_second> (0 for unlimited)";
    pub const SET_UP_LIMIT: &str = "Usage: /setupllimit <bytes_per_second> (0 for unlimited)";
}
