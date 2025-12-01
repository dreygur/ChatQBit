//! Callback query handlers for inline keyboard interactions
//!
//! This module handles all callback queries from inline keyboards,
//! providing interactive responses to button presses.

use crate::constants::{emoji, MAX_CALLBACK_DATA_LEN, MIN_STREAM_FILE_SIZE, TORRENTS_PER_PAGE};
use crate::handlers;
use crate::keyboards;
use crate::rate_limit;
use crate::types::HandlerResult;
use crate::utils;
use teloxide::prelude::*;
use torrent::TorrentApi;

/// Handle all callback queries from inline keyboards
pub async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    torrent: TorrentApi,
    file_server: fileserver::FileServerApi,
) -> HandlerResult {
    // Answer callback query to remove loading state
    bot.answer_callback_query(&q.id).await?;

    let data = match q.data {
        Some(ref data) => data,
        None => return Ok(()),
    };

    // Validate callback data length to prevent abuse
    if data.len() > MAX_CALLBACK_DATA_LEN {
        tracing::warn!("Callback data too long: {} bytes", data.len());
        return Ok(());
    }

    // Rate limiting check
    if !rate_limit::check_rate_limit(q.from.id.0) {
        tracing::debug!("Rate limited user: {}", q.from.id);
        return Ok(());
    }

    let message = match q.message {
        Some(msg) => msg,
        None => return Ok(()),
    };

    // Parse callback data
    let parts: Vec<&str> = data.split(':').collect();

    match parts.as_slice() {
        // Pagination callbacks
        ["page", page_str] => {
            if let Ok(page) = page_str.parse::<usize>() {
                handle_list_page_callback(bot, message, torrent, page).await?;
            }
        }

        // Command callbacks (main menu actions)
        ["cmd", "list"] => {
            handle_list_page_callback(bot, message, torrent, 0).await?;
        }
        ["cmd", "magnet"] => {
            bot.send_message(message.chat.id, "Please send me a magnet link or torrent URL.")
                .await?;
        }
        ["cmd", "transferinfo"] => {
            handle_transfer_info_callback(bot, message, torrent).await?;
        }
        ["cmd", "speedlimits"] => {
            handle_speed_limits_callback(bot, message, torrent).await?;
        }
        ["cmd", "categories"] => {
            handle_categories_callback(bot, message, torrent).await?;
        }
        ["cmd", "tags"] => {
            handle_tags_callback(bot, message, torrent).await?;
        }
        ["cmd", "version"] => {
            handle_version_callback(bot, message, torrent).await?;
        }
        ["cmd", "menu"] => {
            bot.send_message(message.chat.id, "🤖 Main Menu")
                .reply_markup(keyboards::main_menu_keyboard())
                .await?;
        }

        // Torrent actions
        ["resume", hash] | ["start", hash] => {
            execute_torrent_action(bot, message, torrent, hash, "resume", "resumed").await?;
        }
        ["pause", hash] | ["stop", hash] => {
            execute_torrent_action(bot, message, torrent, hash, "pause", "paused").await?;
        }
        ["recheck", hash] => {
            execute_torrent_action(bot, message, torrent, hash, "recheck", "rechecking").await?;
        }
        ["reannounce", hash] => {
            execute_torrent_action(bot, message, torrent, hash, "reannounce", "reannounced").await?;
        }
        ["topprio", hash] => {
            execute_torrent_action(bot, message, torrent, hash, "topprio", "priority set to top").await?;
        }
        ["bottomprio", hash] => {
            execute_torrent_action(bot, message, torrent, hash, "bottomprio", "priority set to bottom").await?;
        }
        ["info", hash] => {
            handle_info_callback(bot, message, torrent, hash).await?;
        }
        ["files", hash] => {
            handle_files_callback(bot, message, torrent, hash).await?;
        }
        ["stream", hash] => {
            handle_stream_callback(bot, message, torrent, file_server, hash).await?;
        }
        ["sequential", hash] => {
            handle_sequential_callback(bot, message, torrent, hash).await?;
        }

        // Destructive actions - show confirmation
        ["delete", hash] => {
            bot.edit_message_text(
                message.chat.id,
                message.id,
                format!("⚠️ Are you sure you want to delete this torrent?\n\nHash: `{}`\n\nFiles will be kept.", hash),
            )
            .reply_markup(keyboards::confirm_keyboard("delete", hash))
            .await?;
        }
        ["deletedata", hash] => {
            bot.edit_message_text(
                message.chat.id,
                message.id,
                format!("⚠️ Are you sure you want to delete this torrent AND its files?\n\nHash: `{}`\n\n🔥 This action cannot be undone!", hash),
            )
            .reply_markup(keyboards::confirm_keyboard("deletedata", hash))
            .await?;
        }

        // Confirmed actions
        ["confirm", action, hash] => {
            match *action {
                "delete" => {
                    if let Err(e) = torrent.delete_torrents(hash, false).await {
                        bot.edit_message_text(
                            message.chat.id,
                            message.id,
                            format!("{} Error deleting torrent: {}", emoji::ERROR, e),
                        )
                        .await?;
                    } else {
                        bot.edit_message_text(
                            message.chat.id,
                            message.id,
                            format!("{} Torrent deleted (files kept)", emoji::SUCCESS),
                        )
                        .await?;
                    }
                }
                "deletedata" => {
                    if let Err(e) = torrent.delete_torrents(hash, true).await {
                        bot.edit_message_text(
                            message.chat.id,
                            message.id,
                            format!("{} Error deleting torrent: {}", emoji::ERROR, e),
                        )
                        .await?;
                    } else {
                        bot.edit_message_text(
                            message.chat.id,
                            message.id,
                            format!("{} Torrent and files deleted", emoji::SUCCESS),
                        )
                        .await?;
                    }
                }
                _ => {}
            }
        }

        // Speed limit actions
        ["setlimit", "dl"] => {
            bot.send_message(message.chat.id, "Please use command: /setdllimit <bytes_per_second>")
                .await?;
        }
        ["setlimit", "ul"] => {
            bot.send_message(message.chat.id, "Please use command: /setupllimit <bytes_per_second>")
                .await?;
        }
        ["removelimit", "dl"] => {
            if let Err(e) = torrent.set_download_limit(0).await {
                bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                    .await?;
            } else {
                bot.send_message(
                    message.chat.id,
                    format!("{} Download limit removed (unlimited)", emoji::SUCCESS),
                )
                .await?;
            }
        }
        ["removelimit", "ul"] => {
            if let Err(e) = torrent.set_upload_limit(0).await {
                bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                    .await?;
            } else {
                bot.send_message(
                    message.chat.id,
                    format!("{} Upload limit removed (unlimited)", emoji::SUCCESS),
                )
                .await?;
            }
        }

        // Cancel confirmation
        ["cancel"] => {
            bot.edit_message_text(message.chat.id, message.id, "❌ Operation cancelled")
                .await?;
        }

        // No-op (for disabled buttons like page counter)
        ["noop"] => {}

        _ => {
            tracing::warn!("Unknown callback data: {}", data);
        }
    }

    Ok(())
}

/// Execute a torrent action via callback
async fn execute_torrent_action(
    bot: Bot,
    message: Message,
    torrent: TorrentApi,
    hash: &str,
    action: &str,
    success_msg: &str,
) -> HandlerResult {
    let result = match action {
        "resume" | "start" => torrent.start_torrents(hash).await,
        "pause" | "stop" => torrent.stop_torrents(hash).await,
        "recheck" => torrent.recheck_torrents(hash).await,
        "reannounce" => torrent.reannounce_torrents(hash).await,
        "topprio" => torrent.set_top_priority(hash).await,
        "bottomprio" => torrent.set_bottom_priority(hash).await,
        _ => return Ok(()),
    };

    match result {
        Ok(_) => {
            bot.send_message(
                message.chat.id,
                format!("{} Torrent {}", emoji::SUCCESS, success_msg),
            )
            .await?;
        }
        Err(e) => {
            bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                .await?;
        }
    }

    Ok(())
}

/// Handle paginated list callback
async fn handle_list_page_callback(
    bot: Bot,
    message: Message,
    torrent: TorrentApi,
    page: usize,
) -> HandlerResult {
    let torrents = torrent.query().await.map_err(|e| {
        tracing::error!("Error fetching torrents: {}", e);
        e
    })?;

    if torrents.is_empty() {
        bot.send_message(message.chat.id, "No torrents in queue.").await?;
        return Ok(());
    }

    let total_pages = torrents.len().div_ceil(TORRENTS_PER_PAGE);
    let page = page.min(total_pages.saturating_sub(1));
    let start = page * TORRENTS_PER_PAGE;
    let end = (start + TORRENTS_PER_PAGE).min(torrents.len());

    let mut response = format!(
        "{} Torrents ({}-{} of {}):\n\n",
        emoji::DOWNLOAD,
        start + 1,
        end,
        torrents.len()
    );

    for t in torrents.iter().skip(start).take(TORRENTS_PER_PAGE) {
        response.push_str(&handlers::format_torrent_item(t));
    }

    bot.send_message(message.chat.id, response)
        .reply_markup(keyboards::pagination_keyboard(page, total_pages))
        .await?;
    Ok(())
}

/// Handle info callback
async fn handle_info_callback(
    bot: Bot,
    message: Message,
    torrent: TorrentApi,
    hash: &str,
) -> HandlerResult {
    match torrent.get_torrent_info(hash).await {
        Ok(info) => {
            let response = handlers::format_torrent_info(&info);
            bot.send_message(message.chat.id, response).await?;
        }
        Err(e) => {
            bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                .await?;
        }
    }
    Ok(())
}

/// Handle transfer info callback
async fn handle_transfer_info_callback(
    bot: Bot,
    message: Message,
    torrent: TorrentApi,
) -> HandlerResult {
    match torrent.get_transfer_info().await {
        Ok(info) => {
            let response = handlers::format_transfer_info(&info);
            bot.send_message(message.chat.id, response).await?;
        }
        Err(e) => {
            bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                .await?;
        }
    }
    Ok(())
}

/// Handle speed limits callback
async fn handle_speed_limits_callback(
    bot: Bot,
    message: Message,
    torrent: TorrentApi,
) -> HandlerResult {
    match (
        torrent.get_download_limit().await,
        torrent.get_upload_limit().await,
    ) {
        (Ok(dl), Ok(ul)) => {
            let response = format!(
                "{} Global Speed Limits:\n\n\
                Download: {}\n\
                Upload: {}",
                emoji::SPEED,
                utils::format_limit(dl),
                utils::format_limit(ul)
            );
            bot.send_message(message.chat.id, response)
                .reply_markup(keyboards::speed_limit_keyboard())
                .await?;
        }
        _ => {
            bot.send_message(message.chat.id, format!("{} Error getting speed limits", emoji::ERROR))
                .await?;
        }
    }
    Ok(())
}

/// Handle categories callback
async fn handle_categories_callback(
    bot: Bot,
    message: Message,
    torrent: TorrentApi,
) -> HandlerResult {
    match torrent.get_categories().await {
        Ok(cats) => {
            if cats.is_empty() {
                bot.send_message(message.chat.id, "No categories found.").await?;
                return Ok(());
            }

            let mut response = format!("{} Categories:\n\n", emoji::CATEGORY);
            for (name, cat) in cats {
                response.push_str(&format!("• {}\n  Path: {}\n\n", name, cat.save_path.display()));
            }
            bot.send_message(message.chat.id, response).await?;
        }
        Err(e) => {
            bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                .await?;
        }
    }
    Ok(())
}

/// Handle tags callback
async fn handle_tags_callback(bot: Bot, message: Message, torrent: TorrentApi) -> HandlerResult {
    match torrent.get_tags().await {
        Ok(tag_list) => {
            if tag_list.is_empty() {
                bot.send_message(message.chat.id, "No tags found.").await?;
                return Ok(());
            }

            let response = format!("{} Tags:\n\n{}", emoji::TAG, tag_list.join(", "));
            bot.send_message(message.chat.id, response).await?;
        }
        Err(e) => {
            bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                .await?;
        }
    }
    Ok(())
}

/// Handle version callback
async fn handle_version_callback(
    bot: Bot,
    message: Message,
    torrent: TorrentApi,
) -> HandlerResult {
    match torrent.get_version().await {
        Ok(ver) => {
            bot.send_message(message.chat.id, format!("{} qBittorrent version: {}", emoji::TOOL, ver))
                .await?;
        }
        Err(e) => {
            bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                .await?;
        }
    }
    Ok(())
}

/// Handle files callback
async fn handle_files_callback(
    bot: Bot,
    message: Message,
    torrent: TorrentApi,
    hash: &str,
) -> HandlerResult {
    let files = match torrent.get_torrent_files(hash).await {
        Ok(f) => f,
        Err(e) => {
            bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                .await?;
            return Ok(());
        }
    };

    if files.is_empty() {
        bot.send_message(message.chat.id, "No files found in this torrent.")
            .await?;
        return Ok(());
    }

    let mut response = format!("{} Files in Torrent:\n\n", emoji::FOLDER);
    for (index, file) in files.iter().enumerate() {
        response.push_str(&format!(
            "{}. {}\n   Size: {} | Progress: {:.1}%\n\n",
            index + 1,
            file.name,
            utils::format_size(file.size),
            file.progress * 100.0
        ));
    }

    bot.send_message(message.chat.id, response).await?;
    Ok(())
}

/// Handle stream callback
async fn handle_stream_callback(
    bot: Bot,
    message: Message,
    torrent: TorrentApi,
    file_server: fileserver::FileServerApi,
    hash: &str,
) -> HandlerResult {
    // Get torrent files
    let files = match torrent.get_torrent_files(hash).await {
        Ok(f) => f,
        Err(e) => {
            bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                .await?;
            return Ok(());
        }
    };

    if files.is_empty() {
        bot.send_message(message.chat.id, "No files found in this torrent.")
            .await?;
        return Ok(());
    }

    // Get torrent info for save path
    let torrent_info = match torrent.get_torrent_info(hash).await {
        Ok(info) => info,
        Err(e) => {
            bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                .await?;
            return Ok(());
        }
    };

    let save_path = torrent_info.save_path;
    let mut response = String::from("*🎬 Streaming Links*\n\n");

    for (index, file) in files.iter().enumerate() {
        let filename = &file.name;

        // Skip small files
        if file.size < MIN_STREAM_FILE_SIZE {
            continue;
        }

        // Generate streaming token
        let token = fileserver::generate_stream_token(hash, index, file_server.state().secret());

        // Construct file path
        let save_path_str = save_path.as_deref().unwrap_or(".");
        let file_path = std::path::PathBuf::from(save_path_str).join(filename);

        // Register stream
        let stream_info = fileserver::StreamInfo {
            torrent_hash: hash.to_string(),
            file_index: index,
            file_path,
            filename: filename.clone(),
            created_at: chrono::Utc::now(),
        };
        file_server.state().register_stream(token.clone(), stream_info);

        // Generate URL
        let stream_url = format!(
            "{}/stream/{}/{}",
            file_server.base_url(),
            token,
            urlencoding::encode(filename)
        );

        let escaped_filename = utils::escape_markdown_v2(filename);
        let escaped_size = utils::escape_markdown_v2(&utils::format_size(file.size));

        response.push_str(&format!(
            "📄 *{}*\n   Size: {}\n   🔗 [Stream]({})\n   📋 `{}`\n\n",
            escaped_filename, escaped_size, stream_url, stream_url
        ));
    }

    response.push_str("💡 *Tip:* Click link to stream or copy URL for VLC/MX Player\\!");

    bot.send_message(message.chat.id, response)
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .disable_web_page_preview(true)
        .await?;
    Ok(())
}

/// Handle sequential callback
async fn handle_sequential_callback(
    bot: Bot,
    message: Message,
    torrent: TorrentApi,
    hash: &str,
) -> HandlerResult {
    match torrent.toggle_sequential_download(hash).await {
        Ok(_) => {
            // Also toggle first/last piece priority
            let _ = torrent.toggle_first_last_piece_priority(hash).await;

            bot.send_message(
                message.chat.id,
                format!(
                    "{} Sequential download mode toggled!\n\n\
                    ℹ️ Sequential mode downloads pieces in order for streaming.",
                    emoji::SUCCESS
                ),
            )
            .await?;
        }
        Err(e) => {
            bot.send_message(message.chat.id, format!("{} Error: {}", emoji::ERROR, e))
                .await?;
        }
    }
    Ok(())
}
