use crate::commands;
use crate::types::Command;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
};

pub use crate::types::State;

pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![State::Start]
                .branch(case![Command::Help].endpoint(commands::help))
                // .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Magnet].endpoint(commands::get_magnet))
                .branch(case![Command::Query].endpoint(commands::query))
                .branch(case![Command::Test].endpoint(commands::test))
        )
        .branch(case![Command::Cancel].endpoint(commands::cancel));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::GetMagnet].endpoint(commands::magnet))
        .branch(dptree::endpoint(commands::invalid_state));

    dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(message_handler)
}
