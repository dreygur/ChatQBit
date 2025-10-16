//! Command handlers for the Telegram bot
//!
//! This module contains all the command handler functions that respond to user commands.
//! Each handler follows a consistent pattern:
//! 1. Parse and validate arguments
//! 2. Execute the operation via TorrentApi
//! 3. Format and send the response

use crate::constants::{emoji, usage, MAX_TORRENTS_DISPLAY};
use crate::handlers::{self, execute_hash_command};
use crate::types::{Command, HandlerResult, MyDialogue, State};
use crate::utils;
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
    bot.send_message(msg.chat.id, "Operation cancelled.").await?;
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
    let text = match msg.text() {
        Some(t) => t,
        None => {
            handlers::send_response(
                bot,
                msg.chat.id,
                emoji::ERROR,
                "Please send a valid magnet link or torrent URL.",
            )
            .await?;
            return Ok(());
        }
    };

    let urls = [text.to_string()];
    match torrent.magnet(&urls).await {
        Ok(_) => {
            handlers::send_response(
                bot,
                msg.chat.id,
                emoji::SUCCESS,
                "Torrent added successfully to download queue!",
            )
            .await?;
        }
        Err(err) => {
            tracing::error!("Failed to add torrent: {}", err);
            bot.send_message(msg.chat.id, format!("{} Failed to add torrent: {}", emoji::ERROR, err))
                .await?;
        }
    }
    Ok(())
}

/// List all torrents with status and progress
pub async fn list(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    let torrents = match torrent.query().await {
        Ok(t) => t,
        Err(err) => {
            tracing::error!("Error fetching torrents: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error fetching torrents: {}", emoji::ERROR, err))
                .await?;
            return Ok(());
        }
    };

    if torrents.is_empty() {
        bot.send_message(msg.chat.id, "No torrents in queue.").await?;
        return Ok(());
    }

    let mut response = format!("{} Current Torrents:\n\n", emoji::DOWNLOAD);
    for torrent in torrents.iter().take(MAX_TORRENTS_DISPLAY) {
        response.push_str(&handlers::format_torrent_item(torrent));
    }

    if torrents.len() > MAX_TORRENTS_DISPLAY {
        response.push_str(&format!(
            "\n...and {} more torrents",
            torrents.len() - MAX_TORRENTS_DISPLAY
        ));
    }

    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}

/// Get detailed information about a specific torrent
pub async fn info(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    let args = utils::parse_args(msg.text().unwrap_or(""));

    let hash = match utils::extract_hash_arg(&args) {
        Ok(h) => h,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("{} {}\n{}", emoji::ERROR, e, usage::INFO))
                .await?;
            return Ok(());
        }
    };

    match torrent.get_torrent_info(hash).await {
        Ok(info) => {
            let response = handlers::format_torrent_info(&info);
            bot.send_message(msg.chat.id, response).await?;
        }
        Err(err) => {
            tracing::error!("Error getting torrent info: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
        }
    }

    Ok(())
}

/// Start/resume torrents
pub async fn start(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot,
        msg,
        torrent,
        usage::START,
        "Torrent(s) started successfully!",
        |api, hash| async move { api.start_torrents(&hash).await },
    )
    .await
}

/// Stop/pause torrents
pub async fn stop(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot,
        msg,
        torrent,
        usage::STOP,
        "Torrent(s) stopped successfully!",
        |api, hash| async move { api.stop_torrents(&hash).await },
    )
    .await
}

/// Delete torrent (keep files)
pub async fn delete(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot,
        msg,
        torrent,
        usage::DELETE,
        "Torrent deleted (files kept)!",
        |api, hash| async move { api.delete_torrents(&hash, false).await },
    )
    .await
}

/// Delete torrent with files
pub async fn delete_data(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot,
        msg,
        torrent,
        usage::DELETE_DATA,
        "Torrent and files deleted!",
        |api, hash| async move { api.delete_torrents(&hash, true).await },
    )
    .await
}

/// Recheck torrent
pub async fn recheck(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot,
        msg,
        torrent,
        usage::RECHECK,
        "Torrent recheck started!",
        |api, hash| async move { api.recheck_torrents(&hash).await },
    )
    .await
}

/// Reannounce torrent to trackers
pub async fn reannounce(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot,
        msg,
        torrent,
        usage::REANNOUNCE,
        "Torrent reannounced to trackers!",
        |api, hash| async move { api.reannounce_torrents(&hash).await },
    )
    .await
}

/// Set torrent priority to top
pub async fn top_prio(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot,
        msg,
        torrent,
        usage::TOP_PRIO,
        "Torrent priority set to top!",
        |api, hash| async move { api.set_top_priority(&hash).await },
    )
    .await
}

/// Set torrent priority to bottom
pub async fn bottom_prio(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot,
        msg,
        torrent,
        usage::BOTTOM_PRIO,
        "Torrent priority set to bottom!",
        |api, hash| async move { api.set_bottom_priority(&hash).await },
    )
    .await
}

/// Get transfer information (speeds, data usage)
pub async fn transfer_info(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    match torrent.get_transfer_info().await {
        Ok(info) => {
            let response = handlers::format_transfer_info(&info);
            bot.send_message(msg.chat.id, response).await?;
        }
        Err(err) => {
            tracing::error!("Error getting transfer info: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
        }
    }
    Ok(())
}

/// Get qBittorrent version
pub async fn version(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    match torrent.get_version().await {
        Ok(ver) => {
            bot.send_message(msg.chat.id, format!("{} qBittorrent version: {}", emoji::TOOL, ver))
                .await?;
        }
        Err(err) => {
            tracing::error!("Error getting version: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
        }
    }
    Ok(())
}

/// List all categories
pub async fn categories(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    match torrent.get_categories().await {
        Ok(cats) => {
            if cats.is_empty() {
                bot.send_message(msg.chat.id, "No categories found.").await?;
                return Ok(());
            }

            let mut response = format!("{} Categories:\n\n", emoji::CATEGORY);
            for (name, cat) in cats {
                response.push_str(&format!("• {}\n  Path: {}\n\n", name, cat.save_path.display()));
            }
            bot.send_message(msg.chat.id, response).await?;
        }
        Err(err) => {
            tracing::error!("Error getting categories: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
        }
    }
    Ok(())
}

/// List all tags
pub async fn tags(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    match torrent.get_tags().await {
        Ok(tag_list) => {
            if tag_list.is_empty() {
                bot.send_message(msg.chat.id, "No tags found.").await?;
                return Ok(());
            }

            let response = format!("{} Tags:\n\n{}", emoji::TAG, tag_list.join(", "));
            bot.send_message(msg.chat.id, response).await?;
        }
        Err(err) => {
            tracing::error!("Error getting tags: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
        }
    }
    Ok(())
}

/// Get global speed limits
pub async fn speed_limits(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    match (
        torrent.get_download_limit().await,
        torrent.get_upload_limit().await,
    ) {
        (Ok(dl), Ok(ul)) => {
            let response = format!(
                "{} Global Speed Limits:\n\n\
                Download Limit: {}\n\
                Upload Limit: {}",
                emoji::SPEED,
                utils::format_limit(dl),
                utils::format_limit(ul)
            );
            bot.send_message(msg.chat.id, response).await?;
        }
        _ => {
            bot.send_message(msg.chat.id, format!("{} Error getting speed limits", emoji::ERROR))
                .await?;
        }
    }
    Ok(())
}

/// Set global download limit
pub async fn set_dl_limit(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    let args = utils::parse_args(msg.text().unwrap_or(""));

    let limit = match utils::extract_limit_arg(&args) {
        Ok(l) => l,
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!("{} {}\n{}", emoji::ERROR, e, usage::SET_DL_LIMIT),
            )
            .await?;
            return Ok(());
        }
    };

    match torrent.set_download_limit(limit).await {
        Ok(_) => {
            bot.send_message(
                msg.chat.id,
                format!(
                    "{} Download limit set to: {}",
                    emoji::SUCCESS,
                    utils::format_limit(limit)
                ),
            )
            .await?;
        }
        Err(err) => {
            tracing::error!("Error setting download limit: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
        }
    }

    Ok(())
}

/// Set global upload limit
pub async fn set_up_limit(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    let args = utils::parse_args(msg.text().unwrap_or(""));

    let limit = match utils::extract_limit_arg(&args) {
        Ok(l) => l,
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!("{} {}\n{}", emoji::ERROR, e, usage::SET_UP_LIMIT),
            )
            .await?;
            return Ok(());
        }
    };

    match torrent.set_upload_limit(limit).await {
        Ok(_) => {
            bot.send_message(
                msg.chat.id,
                format!(
                    "{} Upload limit set to: {}",
                    emoji::SUCCESS,
                    utils::format_limit(limit)
                ),
            )
            .await?;
        }
        Err(err) => {
            tracing::error!("Error setting upload limit: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
        }
    }

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
