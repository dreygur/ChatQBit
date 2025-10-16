use std::fmt;

/// Custom error type for telegram bot operations
#[derive(Debug)]
pub enum BotError {
    /// Telegram API error
    TelegramError(teloxide::RequestError),
    /// Torrent API error
    TorrentError(qbit_rs::Error),
    /// Invalid command arguments
    InvalidArguments(String),
    /// Generic error with message
    Message(String),
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BotError::TelegramError(e) => write!(f, "Telegram error: {}", e),
            BotError::TorrentError(e) => write!(f, "qBittorrent error: {}", e),
            BotError::InvalidArguments(msg) => write!(f, "Invalid arguments: {}", msg),
            BotError::Message(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for BotError {}

impl From<teloxide::RequestError> for BotError {
    fn from(err: teloxide::RequestError) -> Self {
        BotError::TelegramError(err)
    }
}

impl From<qbit_rs::Error> for BotError {
    fn from(err: qbit_rs::Error) -> Self {
        BotError::TorrentError(err)
    }
}

/// Result type alias for bot operations
pub type BotResult<T> = Result<T, BotError>;

/// Helper trait to convert results into user-friendly messages
pub trait UserMessage {
    fn user_message(&self) -> String;
}

impl UserMessage for BotError {
    fn user_message(&self) -> String {
        match self {
            BotError::TelegramError(e) => format!("❌ Communication error: {}", e),
            BotError::TorrentError(e) => format!("❌ qBittorrent error: {}", e),
            BotError::InvalidArguments(msg) => format!("❌ {}", msg),
            BotError::Message(msg) => format!("❌ {}", msg),
        }
    }
}
