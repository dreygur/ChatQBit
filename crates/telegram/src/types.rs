use teloxide::{
    dispatching::dialogue::{Dialogue, InMemStorage},
    macros::BotCommands,
};

/// Type alias for dialogue management with State and InMemStorage
pub type MyDialogue = Dialogue<State, InMemStorage<State>>;

/// Type alias for handler result types
pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Represents the dialogue state for the bot conversation
#[derive(Clone, Default, Debug)]
pub enum State {
    /// Initial state when conversation starts
    #[default]
    Start,
    /// State when waiting for magnet link input
    GetMagnet,
}

/// Available bot commands
#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "Display help information")]
    Help,
    #[command(description = "Add a torrent via magnet link or URL")]
    Magnet,
    #[command(description = "List all torrents with status and progress")]
    List,
    #[command(description = "Get detailed info about a torrent (usage: /info <hash>)")]
    Info,
    #[command(description = "Start/resume torrents (usage: /start <hash> or /start all)")]
    Start,
    #[command(description = "Stop/pause torrents (usage: /stop <hash> or /stop all)")]
    Stop,
    #[command(description = "Delete torrent (usage: /delete <hash> or /deletedata <hash>)")]
    Delete,
    #[command(description = "Delete torrent with files (usage: /deletedata <hash>)")]
    DeleteData,
    #[command(description = "Recheck torrent (usage: /recheck <hash>)")]
    Recheck,
    #[command(description = "Reannounce torrent (usage: /reannounce <hash>)")]
    Reannounce,
    #[command(description = "Set top priority (usage: /topprio <hash>)")]
    TopPrio,
    #[command(description = "Set bottom priority (usage: /bottomprio <hash>)")]
    BottomPrio,
    #[command(description = "Get transfer info (speeds, data usage)")]
    TransferInfo,
    #[command(description = "Get qBittorrent version info")]
    Version,
    #[command(description = "List all categories")]
    Categories,
    #[command(description = "List all tags")]
    Tags,
    #[command(description = "Get global speed limits")]
    SpeedLimits,
    #[command(description = "Set download limit (usage: /setdllimit <bytes/s> or 0 for unlimited)")]
    SetDlLimit,
    #[command(description = "Set upload limit (usage: /setupllimit <bytes/s> or 0 for unlimited)")]
    SetUpLimit,
    #[command(description = "Cancel the current operation")]
    Cancel,
}
