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
use teloxide::{net::Download, prelude::*, utils::command::BotCommands};
use torrent::TorrentApi;

/// Welcome message when user starts the bot
pub async fn start(bot: Bot, msg: Message) -> HandlerResult {
    let welcome_text = format!(
        "👋 Welcome to ChatQBit!\n\n\
        I'm your personal qBittorrent remote control bot.\n\n\
        🎯 Quick Actions:\n\
        • /menu - Interactive menu\n\
        • /list - View all torrents\n\
        • /magnet - Add new torrent\n\
        • /help - See all commands\n\n\
        Let's get started! Try /menu for an interactive experience."
    );

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
pub async fn magnet(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    torrent: TorrentApi,
) -> HandlerResult {
    // Handle document (file) messages
    if let Some(document) = msg.document().cloned() {
        return handle_torrent_file(bot, dialogue, msg, torrent, &document).await;
    }

    // Handle text messages (magnet links/URLs)
    let text = match msg.text() {
        Some(t) => t,
        None => {
            handlers::send_response(
                bot,
                msg.chat.id,
                emoji::ERROR,
                "Please send a valid magnet link, torrent URL, or .torrent file.",
            )
            .await?;
            return Ok(());
        }
    };

    let urls = [text.to_string()];

    // Extract info hash from magnet link for duplicate checking and sequential mode
    let info_hash = extract_hash_from_magnet(text);

    // Check for duplicates if enabled
    if crate::constants::ENABLE_DUPLICATE_CHECK {
        match torrent.check_duplicates(&urls).await {
            Ok(torrent::DuplicateCheckResult::Duplicates(hashes)) => {
                let hash_list = hashes
                    .iter()
                    .map(|h| utils::truncate_hash(h, 8))
                    .collect::<Vec<_>>()
                    .join(", ");

                let message = format!(
                    "⚠️ Duplicate torrent detected!\n\n\
                    This torrent is already in your download queue:\n\
                    Hash: {}\n\n\
                    Torrent was not added to avoid duplicates.",
                    hash_list
                );

                bot.send_message(msg.chat.id, message).await?;
                dialogue.exit().await?;
                return Ok(());
            }
            Ok(torrent::DuplicateCheckResult::NoDuplicates) => {
                // Continue to add torrent
                tracing::debug!("No duplicates found, proceeding to add torrent");
            }
            Err(err) => {
                // Log error but continue with adding (fail-open behavior)
                tracing::warn!("Duplicate check failed, proceeding anyway: {}", err);
            }
        }
    }

    // Add the torrent
    match torrent.magnet(&urls).await {
        Ok(_) => {
            // Enable sequential download and first/last piece priority by default
            if let Some(hash) = info_hash {
                tracing::info!("Enabling sequential download and first/last piece priority for torrent: {}", hash);

                if let Err(err) = torrent.toggle_sequential_download(&hash).await {
                    tracing::warn!("Failed to enable sequential download: {}", err);
                }

                if let Err(err) = torrent.toggle_first_last_piece_priority(&hash).await {
                    tracing::warn!("Failed to enable first/last piece priority: {}", err);
                }
            } else {
                tracing::warn!("Could not extract info hash from magnet link, skipping sequential mode setup");
            }

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
            bot.send_message(
                msg.chat.id,
                format!("{} Failed to add torrent: {}", emoji::ERROR, err),
            )
            .await?;
        }
    }

    dialogue.exit().await?;
    Ok(())
}

/// Handle .torrent file uploads
async fn handle_torrent_file(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    torrent: TorrentApi,
    document: &teloxide::types::Document,
) -> HandlerResult {
    // Validate file extension
    let filename = document.file_name.as_deref().unwrap_or("unknown");
    if !filename.ends_with(".torrent") {
        bot.send_message(
            msg.chat.id,
            format!(
                "{} Invalid file type. Please send a .torrent file.\n\nReceived: {}",
                emoji::ERROR, filename
            ),
        )
        .await?;
        return Ok(());
    }

    tracing::info!("Received torrent file: {} ({} bytes)", filename, document.file.size);

    // Download file from Telegram servers
    let file = match bot.get_file(&document.file.id).await {
        Ok(f) => f,
        Err(err) => {
            tracing::error!("Failed to get file from Telegram: {}", err);
            bot.send_message(
                msg.chat.id,
                format!("{} Failed to retrieve file: {}", emoji::ERROR, err),
            )
            .await?;
            return Ok(());
        }
    };

    let mut file_data = Vec::new();
    match bot.download_file(&file.path, &mut file_data).await {
        Ok(_) => {
            tracing::info!("Downloaded file: {} bytes", file_data.len());
        }
        Err(err) => {
            tracing::error!("Failed to download file: {}", err);
            bot.send_message(
                msg.chat.id,
                format!("{} Failed to download file: {}", emoji::ERROR, err),
            )
            .await?;
            return Ok(());
        }
    }

    // Validate file is actually a .torrent file (basic check)
    if file_data.is_empty() {
        bot.send_message(
            msg.chat.id,
            format!("{} File is empty", emoji::ERROR),
        )
        .await?;
        return Ok(());
    }

    // Torrent files start with "d8:" (bencoded dictionary)
    if !file_data.starts_with(b"d") {
        bot.send_message(
            msg.chat.id,
            format!("{} Invalid .torrent file format", emoji::ERROR),
        )
        .await?;
        return Ok(());
    }

    // Extract info hash from torrent file for duplicate checking and sequential mode
    let info_hash = utils::extract_torrent_info_hash(&file_data);

    // Check for duplicates if enabled
    if crate::constants::ENABLE_DUPLICATE_CHECK {
        if let Some(ref hash) = info_hash {
            tracing::debug!("Extracted info hash from torrent file: {}", hash);

            let dummy_magnet = format!("magnet:?xt=urn:btih:{}", hash);
            let urls = [dummy_magnet];

            match torrent.check_duplicates(&urls).await {
                Ok(torrent::DuplicateCheckResult::Duplicates(hashes)) => {
                    let hash_list = hashes
                        .iter()
                        .map(|h| utils::truncate_hash(h, 8))
                        .collect::<Vec<_>>()
                        .join(", ");

                    let message = format!(
                        "⚠️ Duplicate torrent detected!\n\n\
                        This torrent is already in your download queue:\n\
                        Hash: {}\n\n\
                        Torrent was not added to avoid duplicates.",
                        hash_list
                    );

                    bot.send_message(msg.chat.id, message).await?;
                    dialogue.exit().await?;
                    return Ok(());
                }
                Ok(torrent::DuplicateCheckResult::NoDuplicates) => {
                    tracing::debug!("No duplicates found, proceeding to add torrent file");
                }
                Err(err) => {
                    tracing::warn!("Duplicate check failed for torrent file, proceeding anyway: {}", err);
                }
            }
        } else {
            tracing::warn!("Could not extract info hash from torrent file for duplicate checking");
        }
    }

    // Add the torrent file
    match torrent.add_torrent_file(filename, file_data).await {
        Ok(_) => {
            // Enable sequential download and first/last piece priority by default
            if let Some(hash) = info_hash {
                tracing::info!("Enabling sequential download and first/last piece priority for torrent: {}", hash);

                if let Err(err) = torrent.toggle_sequential_download(&hash).await {
                    tracing::warn!("Failed to enable sequential download: {}", err);
                }

                if let Err(err) = torrent.toggle_first_last_piece_priority(&hash).await {
                    tracing::warn!("Failed to enable first/last piece priority: {}", err);
                }
            } else {
                tracing::warn!("Could not extract info hash from torrent file, skipping sequential mode setup");
            }

            handlers::send_response(
                bot,
                msg.chat.id,
                emoji::SUCCESS,
                &format!("Torrent file '{}' added successfully to download queue!", filename),
            )
            .await?;
        }
        Err(err) => {
            tracing::error!("Failed to add torrent file: {}", err);
            bot.send_message(
                msg.chat.id,
                format!("{} Failed to add torrent file: {}", emoji::ERROR, err),
            )
            .await?;
        }
    }

    dialogue.exit().await?;
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

    // Add helpful tip about using hashes
    response.push_str("\n💡 Tip: Tap the hash (monospace text) to copy it for use in commands.");

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

/// Resume/start torrents
pub async fn resume(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot,
        msg,
        torrent,
        usage::RESUME,
        "Torrent(s) resumed successfully!",
        |api, hash| async move { api.start_torrents(&hash).await },
    )
    .await
}

/// Pause/stop torrents
pub async fn pause(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot,
        msg,
        torrent,
        usage::PAUSE,
        "Torrent(s) paused successfully!",
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

/// Show interactive menu
pub async fn menu(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "🤖 Main Menu - Choose an action:")
        .reply_markup(crate::keyboards::main_menu_keyboard())
        .await?;
    Ok(())
}

/// Generate streaming links for torrent files
pub async fn stream(
    bot: Bot,
    msg: Message,
    torrent: TorrentApi,
    file_server: fileserver::FileServerApi,
) -> HandlerResult {
    let args = utils::parse_args(msg.text().unwrap_or(""));

    let hash = match utils::extract_hash_arg(&args) {
        Ok(h) => h,
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!("{} {}\n\nUsage: /stream <torrent_hash>\n\nTip: Use /list to get full torrent hashes.", emoji::ERROR, e),
            )
            .await?;
            return Ok(());
        }
    };

    // Get torrent files
    let files = match torrent.get_torrent_files(hash).await {
        Ok(f) => f,
        Err(err) => {
            tracing::error!("Error getting torrent files: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
            return Ok(());
        }
    };

    if files.is_empty() {
        bot.send_message(msg.chat.id, "No files found in this torrent.")
            .await?;
        return Ok(());
    }

    // Get torrent info for save path
    let torrent_info = match torrent.get_torrent_info(hash).await {
        Ok(info) => info,
        Err(err) => {
            tracing::error!("Error getting torrent info: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
            return Ok(());
        }
    };

    let save_path = torrent_info.save_path;
    let mut response = String::from("*🎬 Streaming Links for Torrent*\n\n");

    // Generate streaming links for each file
    for (index, file) in files.iter().enumerate() {
        let filename = &file.name;

        // Skip small files (likely metadata/samples)
        let file_size = file.size;
        if file_size < 1_000_000 {  // Skip files smaller than 1MB
            continue;
        }

        // Generate streaming token
        let token = fileserver::generate_stream_token(hash, index, file_server.state().secret());

        // Construct full file path
        // Note: save_path is already absolute (e.g., /home/user/Downloads/)
        // filename includes relative path within torrent (e.g., "Folder/video.mkv")
        let save_path_str = save_path.as_ref().map(|s| s.as_str()).unwrap_or(".");
        let file_path = std::path::PathBuf::from(save_path_str).join(filename);

        tracing::debug!("Registering stream - save_path: {:?}, filename: {}, full_path: {}",
                        save_path, filename, file_path.display());

        // Register stream in server state
        let stream_info = fileserver::StreamInfo {
            torrent_hash: hash.to_string(),
            file_index: index,
            file_path,
            filename: filename.clone(),
            created_at: chrono::Utc::now(),
        };
        file_server.state().register_stream(token.clone(), stream_info);

        // Generate URL
        let stream_url = format!("{}/stream/{}/{}", file_server.base_url(), token, urlencoding::encode(&filename));

        // Escape filename and size for MarkdownV2
        let escaped_filename = utils::escape_markdown_v2(filename);
        let escaped_size = utils::escape_markdown_v2(&utils::format_size(file_size as u64));

        response.push_str(&format!(
            "📄 *{}*\n   Size: {}\n   🔗 [Click to Stream]({})\n   📋 `{}`\n\n",
            escaped_filename,
            escaped_size,
            stream_url,
            stream_url
        ));
    }

    response.push_str("💡 *Tip:* Click the link to open in your browser, or tap the monospace URL to copy and paste into VLC/MX Player\\!");

    bot.send_message(msg.chat.id, response)
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .disable_web_page_preview(true)
        .await?;
    Ok(())
}

/// List all files in a torrent
pub async fn files(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    let args = utils::parse_args(msg.text().unwrap_or(""));

    let hash = match utils::extract_hash_arg(&args) {
        Ok(h) => h,
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!("{} {}\n\nUsage: /files <torrent_hash>\n\nTip: Use /list to get full torrent hashes.", emoji::ERROR, e),
            )
            .await?;
            return Ok(());
        }
    };

    // Get torrent files
    let files = match torrent.get_torrent_files(hash).await {
        Ok(f) => f,
        Err(err) => {
            tracing::error!("Error getting torrent files: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err))
                .await?;
            return Ok(());
        }
    };

    if files.is_empty() {
        bot.send_message(msg.chat.id, "No files found in this torrent.")
            .await?;
        return Ok(());
    }

    let mut response = format!("{} Files in Torrent:\n\n", emoji::FOLDER);

    for (index, file) in files.iter().enumerate() {
        let filename = &file.name;
        let size = file.size;
        let progress = file.progress * 100.0;

        response.push_str(&format!(
            "{}. {}\n   Size: {} | Progress: {:.1}%\n\n",
            index + 1,
            filename,
            utils::format_size(size as u64),
            progress
        ));
    }

    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}

/// Toggle sequential download mode for a torrent
pub async fn sequential(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    let args = utils::parse_args(msg.text().unwrap_or(""));

    let hash = match utils::extract_hash_arg(&args) {
        Ok(h) => h,
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!("{} {}\n\nUsage: /sequential <torrent_hash>\n\nTip: Use /list to get full torrent hashes.", emoji::ERROR, e),
            )
            .await?;
            return Ok(());
        }
    };

    // Toggle sequential download
    match torrent.toggle_sequential_download(hash).await {
        Ok(_) => {
            // Also toggle first/last piece priority for better streaming
            if let Err(err) = torrent.toggle_first_last_piece_priority(hash).await {
                tracing::warn!("Failed to toggle first/last piece priority: {}", err);
            }

            bot.send_message(
                msg.chat.id,
                format!(
                    "{} Sequential download mode toggled!\n\n\
                    ℹ️ Sequential mode downloads pieces in order, which is better for streaming.\n\
                    First and last pieces will be prioritized to load file headers quickly.",
                    emoji::SUCCESS
                ),
            )
            .await?;
        }
        Err(err) => {
            tracing::error!("Error toggling sequential mode: {}", err);
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

/// Extract info hash from magnet link
///
/// Parses magnet links and extracts the info hash (btih parameter).
/// Returns lowercase hex-encoded hash.
fn extract_hash_from_magnet(magnet: &str) -> Option<String> {
    // Magnet links have format: magnet:?xt=urn:btih:<hash>&...
    if !magnet.starts_with("magnet:?") {
        return None;
    }

    // Find the btih parameter
    for param in magnet.split('&') {
        if param.contains("xt=urn:btih:") {
            // Extract hash after "xt=urn:btih:"
            if let Some(hash_start) = param.find("xt=urn:btih:") {
                let hash = &param[hash_start + 12..]; // Skip "xt=urn:btih:"

                // Hash might have additional parameters after it (like &dn=...)
                let hash = hash.split('&').next().unwrap_or(hash);

                // Validate hash length (40 chars for SHA-1 hex, 32 for base32)
                if !hash.is_empty() {
                    return Some(hash.to_lowercase());
                }
            }
        }
    }

    None
}
