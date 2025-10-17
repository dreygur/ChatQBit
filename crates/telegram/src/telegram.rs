use crate::{callbacks, commands};
use crate::types::Command;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    utils::command::BotCommands,
};

pub use crate::types::State;

/// Register bot commands in Telegram menu
pub async fn set_bot_commands(bot: &Bot) -> Result<(), teloxide::RequestError> {
    bot.set_my_commands(Command::bot_commands()).await?;
    Ok(())
}

pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![State::Start]
                .branch(case![Command::Start].endpoint(commands::start))
                .branch(case![Command::Help].endpoint(commands::help))
                .branch(case![Command::Menu].endpoint(commands::menu))
                .branch(case![Command::Magnet].endpoint(commands::get_magnet))
                .branch(case![Command::List].endpoint(commands::list))
                .branch(case![Command::Info].endpoint(commands::info))
                .branch(case![Command::Resume].endpoint(commands::resume))
                .branch(case![Command::Pause].endpoint(commands::pause))
                .branch(case![Command::Delete].endpoint(commands::delete))
                .branch(case![Command::DeleteData].endpoint(commands::delete_data))
                .branch(case![Command::Recheck].endpoint(commands::recheck))
                .branch(case![Command::Reannounce].endpoint(commands::reannounce))
                .branch(case![Command::TopPrio].endpoint(commands::top_prio))
                .branch(case![Command::BottomPrio].endpoint(commands::bottom_prio))
                .branch(case![Command::TransferInfo].endpoint(commands::transfer_info))
                .branch(case![Command::Version].endpoint(commands::version))
                .branch(case![Command::Categories].endpoint(commands::categories))
                .branch(case![Command::Tags].endpoint(commands::tags))
                .branch(case![Command::SpeedLimits].endpoint(commands::speed_limits))
                .branch(case![Command::SetDlLimit].endpoint(commands::set_dl_limit))
                .branch(case![Command::SetUpLimit].endpoint(commands::set_up_limit))
        )
        .branch(case![Command::Cancel].endpoint(commands::cancel));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::GetMagnet].endpoint(commands::magnet))
        .branch(dptree::endpoint(commands::invalid_state));

    // Handle callback queries from inline keyboards
    let callback_handler = Update::filter_callback_query()
        .endpoint(callbacks::handle_callback);

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(callback_handler)
}
