//! Command handler helper functions
//!
//! This module provides reusable patterns for command handlers,
//! reducing code duplication and improving maintainability.

use crate::constants::emoji;
use crate::types::HandlerResult;
use crate::utils;
use teloxide::prelude::*;
use torrent::TorrentApi;

/// Execute a torrent operation with a hash argument
///
/// This helper handles:
/// - Argument parsing and validation
/// - Operation execution
/// - Success/error response formatting
pub async fn execute_hash_command<F, Fut>(
    bot: Bot,
    msg: Message,
    torrent: TorrentApi,
    usage_msg: &str,
    success_msg: &str,
    operation: F,
) -> HandlerResult
where
    F: FnOnce(TorrentApi, String) -> Fut,
    Fut: std::future::Future<Output = Result<(), qbit_rs::Error>>,
{
    let args = utils::parse_args(msg.text().unwrap_or(""));

    let hash = match utils::extract_hash_arg(&args) {
        Ok(h) => h.to_string(),
        Err(e) => {
            bot.send_message(msg.chat.id, format!("{} {}\n{}", emoji::ERROR, e, usage_msg))
                .await?;
            return Ok(());
        }
    };

    match operation(torrent, hash).await {
        Ok(_) => {
            bot.send_message(msg.chat.id, format!("{} {}", emoji::SUCCESS, success_msg))
                .await?;
        }
        Err(err) => {
            tracing::error!("Operation failed: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
        }
    }

    Ok(())
}

/// Send a formatted message with emoji prefix
pub async fn send_response(bot: Bot, chat_id: ChatId, emoji: &str, message: &str) -> HandlerResult {
    bot.send_message(chat_id, format!("{} {}", emoji, message))
        .await?;
    Ok(())
}

/// Format torrent list item for display
pub fn format_torrent_item(torrent: &qbit_rs::model::Torrent) -> String {
    let status = torrent
        .state
        .as_ref()
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "Unknown".to_string());

    let hash = torrent.hash.as_deref().unwrap_or("Unknown");

    format!(
        "{} {}\n   Hash: `{}`\n   Status: {}\n   Progress: {:.2}%\n   Size: {}\n\n",
        emoji::FOLDER,
        torrent.name.as_deref().unwrap_or("Unknown"),
        hash,
        status,
        torrent.progress.unwrap_or(0.0) * 100.0,
        utils::format_bytes(torrent.size.unwrap_or(0))
    )
}

/// Format detailed torrent information
pub fn format_torrent_info(info: &qbit_rs::model::TorrentProperty) -> String {
    format!(
        "{} Torrent Information:\n\n\
        Save Path: {}\n\
        Size: {}\n\
        Downloaded: {}\n\
        Uploaded: {}\n\
        Download Speed: {}\n\
        Upload Speed: {}\n\
        Seeds: {} ({})\n\
        Peers: {} ({})\n\
        Ratio: {:.2}\n\
        ETA: {}\n\
        Added: {}\n\
        Completed: {}",
        emoji::INFO,
        info.save_path.as_deref().unwrap_or("N/A"),
        utils::format_bytes(info.total_size.unwrap_or(0)),
        utils::format_bytes(info.total_downloaded.unwrap_or(0)),
        utils::format_bytes(info.total_uploaded.unwrap_or(0)),
        utils::format_speed(info.dl_speed.unwrap_or(0) as u64),
        utils::format_speed(info.up_speed.unwrap_or(0) as u64),
        info.seeds.unwrap_or(0),
        info.seeds_total.unwrap_or(0),
        info.peers.unwrap_or(0),
        info.peers_total.unwrap_or(0),
        info.share_ratio.unwrap_or(0.0),
        utils::format_eta(info.eta.unwrap_or(0)),
        utils::format_timestamp(info.addition_date.unwrap_or(0)),
        utils::format_timestamp(info.completion_date.unwrap_or(0))
    )
}

/// Format transfer information
pub fn format_transfer_info(info: &qbit_rs::model::TransferInfo) -> String {
    format!(
        "{} Transfer Information:\n\n\
        Download Speed: {}\n\
        Upload Speed: {}\n\
        Downloaded (session): {}\n\
        Uploaded (session): {}\n\
        Download Limit: {}\n\
        Upload Limit: {}",
        emoji::INFO,
        utils::format_speed(info.dl_info_speed),
        utils::format_speed(info.up_info_speed),
        utils::format_bytes(info.dl_info_data as i64),
        utils::format_bytes(info.up_info_data as i64),
        utils::format_limit(info.dl_rate_limit),
        utils::format_limit(info.up_rate_limit)
    )
}

/// Macro to reduce boilerplate in hash-based commands
#[macro_export]
macro_rules! hash_command {
    ($bot:expr, $msg:expr, $torrent:expr, $usage:expr, $success:expr, $method:ident) => {{
        execute_hash_command(
            $bot,
            $msg,
            $torrent,
            $usage,
            $success,
            |api, hash| async move { api.$method(&hash).await },
        )
        .await
    }};
}
