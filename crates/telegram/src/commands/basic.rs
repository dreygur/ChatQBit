//! Basic bot commands (start, help, menu, cancel)

use crate::types::{Command, HandlerResult, MyDialogue};
use teloxide::{prelude::*, utils::command::BotCommands};

/// Welcome message when user starts the bot
pub async fn start(bot: Bot, msg: Message) -> HandlerResult {
    let welcome_text = "👋 Welcome to ChatQBit!\n\n\
        I'm your personal qBittorrent remote control bot.\n\n\
        🎯 Quick Actions:\n\
        • /menu - Interactive menu\n\
        • /list - View all torrents\n\
        • /magnet - Add new torrent\n\
        • /help - See all commands\n\n\
        Let's get started! Try /menu for an interactive experience.";

    bot.send_message(msg.chat.id, welcome_text)
        .reply_markup(crate::keyboards::main_menu_keyboard())
        .await?;
    Ok(())
}

/// Display help message with available commands
pub async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

/// Cancel the current operation and reset dialogue state
pub async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Operation cancelled.").await?;
    dialogue.exit().await?;
    Ok(())
}

/// Show interactive menu
pub async fn menu(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "🤖 Main Menu - Choose an action:")
        .reply_markup(crate::keyboards::main_menu_keyboard())
        .await?;
    Ok(())
}

/// Handle invalid state
pub async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Unable to handle the message. Type /help to see the usage.",
    )
    .await?;
    Ok(())
}
