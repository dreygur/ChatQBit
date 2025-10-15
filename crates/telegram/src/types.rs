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
    #[command(description = "Cancel the current operation")]
    Cancel,
    #[command(description = "Query")]
    Query,
    #[command(description = "Test the bot")]
    Test,
}
