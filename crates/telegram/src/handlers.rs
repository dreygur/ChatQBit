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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test torrent
    fn create_test_torrent(name: Option<&str>, hash: Option<&str>, progress: Option<f64>, size: Option<i64>) -> qbit_rs::model::Torrent {
        qbit_rs::model::Torrent {
            added_on: None,
            amount_left: None,
            auto_tmm: None,
            availability: None,
            category: None,
            completed: None,
            completion_on: None,
            content_path: None,
            dl_limit: None,
            dlspeed: None,
            downloaded: None,
            downloaded_session: None,
            eta: None,
            f_l_piece_prio: None,
            force_start: None,
            hash: hash.map(|s| s.to_string()),
            last_activity: None,
            magnet_uri: None,
            max_ratio: None,
            max_seeding_time: None,
            name: name.map(|s| s.to_string()),
            num_complete: None,
            num_incomplete: None,
            num_leechs: None,
            num_seeds: None,
            priority: None,
            progress,
            ratio: None,
            ratio_limit: None,
            save_path: None,
            seeding_time: None,
            seeding_time_limit: None,
            seen_complete: None,
            seq_dl: None,
            size,
            state: None,
            super_seeding: None,
            tags: None,
            time_active: None,
            total_size: None,
            tracker: None,
            up_limit: None,
            uploaded: None,
            uploaded_session: None,
            upspeed: None,
        }
    }

    #[test]
    fn test_format_torrent_item() {
        let torrent = create_test_torrent(
            Some("Test Torrent"),
            Some("abc123def456"),
            Some(0.5),
            Some(1073741824),
        );

        let formatted = format_torrent_item(&torrent);
        assert!(formatted.contains("Test Torrent"));
        assert!(formatted.contains("abc123def456"));
        assert!(formatted.contains("50.00%"));
    }

    #[test]
    fn test_format_torrent_item_unknown() {
        let torrent = create_test_torrent(None, None, None, None);

        let formatted = format_torrent_item(&torrent);
        assert!(formatted.contains("Unknown"));
    }

    #[test]
    fn test_format_torrent_info() {
        let info = qbit_rs::model::TorrentProperty {
            save_path: Some("/downloads".to_string()),
            total_size: Some(1073741824),
            total_downloaded: Some(536870912),
            total_uploaded: Some(268435456),
            total_uploaded_session: None,
            total_downloaded_session: None,
            dl_speed: Some(1048576),
            up_speed: Some(524288),
            seeds: Some(10),
            seeds_total: Some(100),
            peers: Some(5),
            peers_total: Some(50),
            share_ratio: Some(0.5),
            eta: Some(3600),
            addition_date: Some(1704067200),
            completion_date: Some(0),
            creation_date: None,
            comment: None,
            created_by: None,
            dl_limit: None,
            dl_speed_avg: None,
            last_seen: None,
            nb_connections: None,
            nb_connections_limit: None,
            piece_size: None,
            pieces_have: None,
            pieces_num: None,
            reannounce: None,
            seeding_time: None,
            time_elapsed: None,
            total_wasted: None,
            up_limit: None,
            up_speed_avg: None,
        };

        let formatted = format_torrent_info(&info);
        assert!(formatted.contains("/downloads"));
        assert!(formatted.contains("1.00 GB")); // total_size
        assert!(formatted.contains("10 (100)")); // seeds
        assert!(formatted.contains("5 (50)")); // peers
        assert!(formatted.contains("0.50")); // ratio
    }

    #[test]
    fn test_format_transfer_info() {
        let info = qbit_rs::model::TransferInfo {
            dl_info_speed: 1048576, // 1 MB/s
            up_info_speed: 524288,  // 512 KB/s
            dl_info_data: 1073741824, // 1 GB
            up_info_data: 536870912,  // 512 MB
            dl_rate_limit: 0,
            up_rate_limit: 1048576,
            dht_nodes: 0,
            connection_status: qbit_rs::model::ConnectionStatus::Connected,
        };

        let formatted = format_transfer_info(&info);
        assert!(formatted.contains("1.00 MB/s")); // dl speed
        assert!(formatted.contains("512.00 KB/s")); // up speed
        assert!(formatted.contains("Unlimited")); // dl limit
    }

    #[test]
    fn test_parse_action_from_usage() {
        assert_eq!(parse_action_from_usage("Usage: /resume <hash>"), ("resume", "▶️"));
        assert_eq!(parse_action_from_usage("Usage: /pause <hash>"), ("pause", "⏸️"));
        assert_eq!(parse_action_from_usage("Usage: /delete <hash>"), ("delete", "🗑️"));
        assert_eq!(parse_action_from_usage("Usage: /deletedata <hash>"), ("deletedata", "🗑️💥"));
        assert_eq!(parse_action_from_usage("Usage: /recheck <hash>"), ("recheck", "🔄"));
        assert_eq!(parse_action_from_usage("Usage: /reannounce <hash>"), ("reannounce", "📢"));
        assert_eq!(parse_action_from_usage("Usage: /topprio <hash>"), ("topprio", "⬆️"));
        assert_eq!(parse_action_from_usage("Usage: /bottomprio <hash>"), ("bottomprio", "⬇️"));
        assert_eq!(parse_action_from_usage("Usage: /info <hash>"), ("info", "🔍"));
        assert_eq!(parse_action_from_usage("Unknown command"), ("action", "⚡"));
    }
}
