//! Torrent management commands (add, list, info, files)

use crate::constants::{emoji, usage, MAX_TORRENT_FILE_SIZE, TORRENTS_PER_PAGE};
use crate::handlers;
use crate::types::{HandlerResult, MyDialogue, State};
use crate::utils;
use teloxide::{net::Download, prelude::*};
use torrent::TorrentApi;

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
    let info_hash = extract_hash_from_magnet(text);

    // Check for duplicates
    if let Some(duplicate_msg) = handlers::check_for_duplicates(&torrent, &urls).await {
        bot.send_message(msg.chat.id, duplicate_msg).await?;
        dialogue.exit().await?;
        return Ok(());
    }

    // Add the torrent
    match torrent.magnet(&urls).await {
        Ok(_) => {
            if let Some(ref hash) = info_hash {
                handlers::enable_sequential_mode(&torrent, hash).await;
            } else {
                tracing::warn!("Could not extract info hash from magnet link");
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
    let filename = document.file_name.as_deref().unwrap_or("unknown");

    // Validate file extension
    if !filename.ends_with(".torrent") {
        bot.send_message(
            msg.chat.id,
            format!("{} Invalid file type. Please send a .torrent file.", emoji::ERROR),
        )
        .await?;
        return Ok(());
    }

    // Validate file size
    if document.file.size > MAX_TORRENT_FILE_SIZE {
        bot.send_message(
            msg.chat.id,
            format!(
                "{} File too large. Maximum size is {} MB.",
                emoji::ERROR,
                MAX_TORRENT_FILE_SIZE / (1024 * 1024)
            ),
        )
        .await?;
        return Ok(());
    }

    tracing::info!("Received torrent file: {} ({} bytes)", filename, document.file.size);

    // Download file from Telegram
    let file = match bot.get_file(&document.file.id).await {
        Ok(f) => f,
        Err(err) => {
            tracing::error!("Failed to get file from Telegram: {}", err);
            bot.send_message(msg.chat.id, format!("{} Failed to retrieve file: {}", emoji::ERROR, err))
                .await?;
            return Ok(());
        }
    };

    let mut file_data = Vec::new();
    if let Err(err) = bot.download_file(&file.path, &mut file_data).await {
        tracing::error!("Failed to download file: {}", err);
        bot.send_message(msg.chat.id, format!("{} Failed to download file: {}", emoji::ERROR, err))
            .await?;
        return Ok(());
    }

    // Validate torrent file format
    if file_data.is_empty() || !file_data.starts_with(b"d") {
        bot.send_message(msg.chat.id, format!("{} Invalid .torrent file format", emoji::ERROR))
            .await?;
        return Ok(());
    }

    let info_hash = utils::extract_torrent_info_hash(&file_data);

    // Check for duplicates
    if let Some(ref hash) = info_hash {
        tracing::debug!("Extracted info hash: {}", hash);
        let dummy_magnet = format!("magnet:?xt=urn:btih:{}", hash);
        let urls = [dummy_magnet];

        if let Some(duplicate_msg) = handlers::check_for_duplicates(&torrent, &urls).await {
            bot.send_message(msg.chat.id, duplicate_msg).await?;
            dialogue.exit().await?;
            return Ok(());
        }
    }

    // Add the torrent file
    match torrent.add_torrent_file(filename, file_data).await {
        Ok(_) => {
            if let Some(ref hash) = info_hash {
                handlers::enable_sequential_mode(&torrent, hash).await;
            }

            handlers::send_response(
                bot,
                msg.chat.id,
                emoji::SUCCESS,
                &format!("Torrent file '{}' added successfully!", filename),
            )
            .await?;
        }
        Err(err) => {
            tracing::error!("Failed to add torrent file: {}", err);
            bot.send_message(msg.chat.id, format!("{} Failed to add torrent file: {}", emoji::ERROR, err))
                .await?;
        }
    }

    dialogue.exit().await?;
    Ok(())
}

/// List all torrents with pagination
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

    let total_pages = torrents.len().div_ceil(TORRENTS_PER_PAGE);
    let end = TORRENTS_PER_PAGE.min(torrents.len());

    let mut response = format!("{} Torrents (1-{} of {}):\n\n", emoji::DOWNLOAD, end, torrents.len());
    for t in torrents.iter().take(TORRENTS_PER_PAGE) {
        response.push_str(&handlers::format_torrent_item(t));
    }
    response.push_str("\n💡 Tip: Tap the hash to copy it.");

    bot.send_message(msg.chat.id, response)
        .reply_markup(crate::keyboards::pagination_keyboard(0, total_pages))
        .await?;
    Ok(())
}

/// Get detailed information about a torrent
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
            bot.send_message(msg.chat.id, handlers::format_torrent_info(&info)).await?;
        }
        Err(err) => {
            tracing::error!("Error getting torrent info: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err)).await?;
        }
    }

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
                format!("{} {}\n\nUsage: /files <torrent_hash>", emoji::ERROR, e),
            )
            .await?;
            return Ok(());
        }
    };

    let files = match torrent.get_torrent_files(hash).await {
        Ok(f) => f,
        Err(err) => {
            tracing::error!("Error getting torrent files: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err)).await?;
            return Ok(());
        }
    };

    if files.is_empty() {
        bot.send_message(msg.chat.id, "No files found in this torrent.").await?;
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

    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}

/// Extract info hash from magnet link
fn extract_hash_from_magnet(magnet: &str) -> Option<String> {
    if !magnet.starts_with("magnet:?") {
        return None;
    }

    for param in magnet.split('&') {
        if param.contains("xt=urn:btih:") {
            if let Some(hash_start) = param.find("xt=urn:btih:") {
                let hash = &param[hash_start + 12..];
                let hash = hash.split('&').next().unwrap_or(hash);
                if !hash.is_empty() {
                    return Some(hash.to_lowercase());
                }
            }
        }
    }

    None
}
