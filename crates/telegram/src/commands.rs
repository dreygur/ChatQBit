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
            match torrent.magnet(&urls).await {
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

pub async fn query(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    let resp = torrent.query().await;
    match resp {
        Ok(torrents) => {
            let mut response = String::from("Current Torrents:\n");
            for t in torrents.into_iter() {
                response.push_str(&format!(
                    "- Name: {}\n  Status: {:?}\n  Progress: {:.2}%\n",
                    t.name,
                    t.state,
                    t.progress * 100.0
                ));
            }
            bot.send_message(msg.chat.id, response).await?;
        }
        Err(err) => {
            tracing::error!("Error fetching torrents: {}", err);
            bot.send_message(msg.chat.id, format!("Error fetching torrents: {}", err))
                .await?;
        }
    }
    Ok(())
}


pub async fn test(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    let resp = torrent.test().await;
    match resp {
        Ok(prefs) => {
            bot.send_message(msg.chat.id, format!("App Preferences: {:?}", prefs)).await?;
        }
        Err(err) => {
            tracing::error!("Error fetching app preferences: {}", err);
            bot.send_message(msg.chat.id, format!("Error fetching app preferences: {}", err))
                .await?;
        }
    }
    Ok(())
}
