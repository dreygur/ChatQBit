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
/// If no hash is provided, shows a torrent selection list.
/// This helper handles:
/// - Argument parsing and validation
/// - Showing torrent list if no argument
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
        Err(_) => {
            // No hash provided - show torrent selection list
            return show_torrent_selection(bot, msg, torrent, usage_msg).await;
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

/// Show torrent selection keyboard when no hash argument provided
async fn show_torrent_selection(
    bot: Bot,
    msg: Message,
    torrent: TorrentApi,
    usage_msg: &str,
) -> HandlerResult {
    // Extract action and emoji from usage message
    let (action, action_emoji) = parse_action_from_usage(usage_msg);

    // Fetch torrents
    let torrents = match torrent.query().await {
        Ok(t) => t,
        Err(err) => {
            bot.send_message(msg.chat.id, format!("{} Error fetching torrents: {}", emoji::ERROR, err))
                .await?;
            return Ok(());
        }
    };

    if torrents.is_empty() {
        bot.send_message(msg.chat.id, "No torrents in queue.").await?;
        return Ok(());
    }

    let keyboard = crate::keyboards::torrent_select_keyboard(&torrents, action, action_emoji);
    bot.send_message(msg.chat.id, format!("Select a torrent to {}:", action))
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

/// Parse action name and emoji from usage message
fn parse_action_from_usage(usage_msg: &str) -> (&str, &str) {
    // Map usage messages to actions and emojis
    if usage_msg.contains("/resume") {
        ("resume", "▶️")
    } else if usage_msg.contains("/pause") {
        ("pause", "⏸️")
    } else if usage_msg.contains("/deletedata") {
        ("deletedata", "🗑️💥")
    } else if usage_msg.contains("/delete") {
        ("delete", "🗑️")
    } else if usage_msg.contains("/recheck") {
        ("recheck", "🔄")
    } else if usage_msg.contains("/reannounce") {
        ("reannounce", "📢")
    } else if usage_msg.contains("/topprio") {
        ("topprio", "⬆️")
    } else if usage_msg.contains("/bottomprio") {
        ("bottomprio", "⬇️")
    } else if usage_msg.contains("/info") {
        ("info", "🔍")
    } else {
        ("action", "⚡")
    }
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

/// Check for duplicate torrents before adding
///
/// Returns `Some(message)` if duplicates are found, `None` otherwise
pub async fn check_for_duplicates(
    torrent: &TorrentApi,
    urls: &[String],
) -> Option<String> {
    if !crate::constants::ENABLE_DUPLICATE_CHECK {
        return None;
    }

    match torrent.check_duplicates(urls).await {
        Ok(torrent::DuplicateCheckResult::Duplicates(hashes)) => {
            let hash_list = hashes
                .iter()
                .map(|h| utils::truncate_hash(h, 8))
                .collect::<Vec<_>>()
                .join(", ");

            Some(format!(
                "⚠️ Duplicate torrent detected!\n\n\
                This torrent is already in your download queue:\n\
                Hash: {}\n\n\
                Torrent was not added to avoid duplicates.",
                hash_list
            ))
        }
        Ok(torrent::DuplicateCheckResult::NoDuplicates) => {
            tracing::debug!("No duplicates found, proceeding to add torrent");
            None
        }
        Err(err) => {
            // Log error but continue with adding (fail-open behavior)
            tracing::warn!("Duplicate check failed, proceeding anyway: {}", err);
            None
        }
    }
}

/// Enable sequential download mode for better streaming
///
/// This enables sequential piece download and first/last piece priority.
pub async fn enable_sequential_mode(torrent: &TorrentApi, hash: &str) {
    tracing::info!("Enabling sequential download and first/last piece priority for torrent: {}", hash);

    if let Err(err) = torrent.toggle_sequential_download(hash).await {
        tracing::warn!("Failed to enable sequential download: {}", err);
    }

    if let Err(err) = torrent.toggle_first_last_piece_priority(hash).await {
        tracing::warn!("Failed to enable first/last piece priority: {}", err);
    }
}
