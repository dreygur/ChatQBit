/// Constants used throughout the telegram bot

/// Default hash truncation length for display
pub const HASH_DISPLAY_LENGTH: usize = 8;

/// Maximum number of torrents to display in list command
pub const MAX_TORRENTS_DISPLAY: usize = 50;

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
    pub const INFO: &str = "Usage: /info <torrent_hash>";
    pub const START: &str = "Usage: /start <torrent_hash> or /start all";
    pub const STOP: &str = "Usage: /stop <torrent_hash> or /stop all";
    pub const DELETE: &str = "Usage: /delete <torrent_hash>";
    pub const DELETE_DATA: &str = "Usage: /deletedata <torrent_hash>";
    pub const RECHECK: &str = "Usage: /recheck <torrent_hash>";
    pub const REANNOUNCE: &str = "Usage: /reannounce <torrent_hash>";
    pub const TOP_PRIO: &str = "Usage: /topprio <torrent_hash>";
    pub const BOTTOM_PRIO: &str = "Usage: /bottomprio <torrent_hash>";
    pub const SET_DL_LIMIT: &str = "Usage: /setdllimit <bytes_per_second> (0 for unlimited)";
    pub const SET_UP_LIMIT: &str = "Usage: /setupllimit <bytes_per_second> (0 for unlimited)";
}
