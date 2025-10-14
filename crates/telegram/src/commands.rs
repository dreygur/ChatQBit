use crate::types::{Command, HandlerResult, MyDialogue, State};
use teloxide::{prelude::*, utils::command::BotCommands};
use torrent::TorrentApi;

/// Display help message with available commands
pub async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

/// Cancel the current operation and reset dialogue state
pub async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Operation cancelled.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

/// Request magnet link from user
pub async fn get_magnet(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Please send me the magnet link or torrent URL to download.",
    )
    .await?;
    dialogue.update(State::GetMagnet).await?;
    Ok(())
}

/// Process magnet link or torrent URL and add to qBittorrent
pub async fn magnet(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let urls = [text.to_string()];
            match torrent.client.torrents_add_by_url(&urls).await {
                Ok(_) => {
                    bot.send_message(
                        msg.chat.id,
                        "✅ Torrent added successfully to download queue!",
                    )
                    .await?;
                }
                Err(err) => {
                    bot.send_message(msg.chat.id, format!("❌ Failed to add torrent: {}", err))
                        .await?;
                }
            }
        }
        None => {
            bot.send_message(
                msg.chat.id,
                "❌ Please send a valid magnet link or torrent URL.",
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Unable to handle the message. Type /help to see the usage.",
    )
    .await?;
    Ok(())
}
