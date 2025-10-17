pub mod callbacks;
pub mod commands;
pub mod constants;
pub mod error;
pub mod handlers;
pub mod keyboards;
pub mod telegram;
pub mod types;
pub mod utils;

pub use error::{BotError, BotResult};
pub use teloxide::prelude::Dispatcher;
pub use telegram::set_bot_commands;
pub use types::{Command, HandlerResult, MyDialogue, State};
