//! Configuration commands (speed limits, categories, tags, version)

use crate::constants::{emoji, usage};
use crate::handlers;
use crate::types::HandlerResult;
use crate::utils;
use teloxide::prelude::*;
use torrent::TorrentApi;

/// Get transfer information (speeds, data usage)
pub async fn transfer_info(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    match torrent.get_transfer_info().await {
        Ok(info) => {
            bot.send_message(msg.chat.id, handlers::format_transfer_info(&info)).await?;
        }
        Err(err) => {
            tracing::error!("Error getting transfer info: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err)).await?;
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
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err)).await?;
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
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err)).await?;
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

            bot.send_message(msg.chat.id, format!("{} Tags:\n\n{}", emoji::TAG, tag_list.join(", ")))
                .await?;
        }
        Err(err) => {
            tracing::error!("Error getting tags: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err)).await?;
        }
    }
    Ok(())
}

/// Get global speed limits
pub async fn speed_limits(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    match (torrent.get_download_limit().await, torrent.get_upload_limit().await) {
        (Ok(dl), Ok(ul)) => {
            let response = format!(
                "{} Global Speed Limits:\n\nDownload: {}\nUpload: {}",
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
            bot.send_message(msg.chat.id, format!("{} {}\n{}", emoji::ERROR, e, usage::SET_DL_LIMIT))
                .await?;
            return Ok(());
        }
    };

    match torrent.set_download_limit(limit).await {
        Ok(_) => {
            bot.send_message(
                msg.chat.id,
                format!("{} Download limit set to: {}", emoji::SUCCESS, utils::format_limit(limit)),
            )
            .await?;
        }
        Err(err) => {
            tracing::error!("Error setting download limit: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err)).await?;
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
            bot.send_message(msg.chat.id, format!("{} {}\n{}", emoji::ERROR, e, usage::SET_UP_LIMIT))
                .await?;
            return Ok(());
        }
    };

    match torrent.set_upload_limit(limit).await {
        Ok(_) => {
            bot.send_message(
                msg.chat.id,
                format!("{} Upload limit set to: {}", emoji::SUCCESS, utils::format_limit(limit)),
            )
            .await?;
        }
        Err(err) => {
            tracing::error!("Error setting upload limit: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err)).await?;
        }
    }

    Ok(())
}
