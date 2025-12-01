//! Streaming commands (stream, sequential)

use crate::constants::{emoji, MIN_STREAM_FILE_SIZE};
use crate::types::HandlerResult;
use crate::utils;
use teloxide::prelude::*;
use torrent::TorrentApi;

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
        Err(_) => {
            // No hash - show torrent selection
            let torrents = torrent.query().await.unwrap_or_default();
            if torrents.is_empty() {
                bot.send_message(msg.chat.id, "No torrents in queue.").await?;
                return Ok(());
            }
            let keyboard = crate::keyboards::torrent_select_keyboard(&torrents, "stream", "🎬");
            bot.send_message(msg.chat.id, "Select a torrent to stream:")
                .reply_markup(keyboard)
                .await?;
            return Ok(());
        }
    };

    // Get torrent files
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

    // Get torrent info for save path
    let torrent_info = match torrent.get_torrent_info(hash).await {
        Ok(info) => info,
        Err(err) => {
            tracing::error!("Error getting torrent info: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err)).await?;
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

        tracing::debug!("Registering stream: {}", file_path.display());

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

    bot.send_message(msg.chat.id, response)
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .disable_web_page_preview(true)
        .await?;
    Ok(())
}

/// Toggle sequential download mode
pub async fn sequential(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    let args = utils::parse_args(msg.text().unwrap_or(""));

    let hash = match utils::extract_hash_arg(&args) {
        Ok(h) => h,
        Err(_) => {
            // No hash - show torrent selection
            let torrents = torrent.query().await.unwrap_or_default();
            if torrents.is_empty() {
                bot.send_message(msg.chat.id, "No torrents in queue.").await?;
                return Ok(());
            }
            let keyboard = crate::keyboards::torrent_select_keyboard(&torrents, "sequential", "📶");
            bot.send_message(msg.chat.id, "Select a torrent to toggle sequential mode:")
                .reply_markup(keyboard)
                .await?;
            return Ok(());
        }
    };

    match torrent.toggle_sequential_download(hash).await {
        Ok(_) => {
            // Also toggle first/last piece priority
            let _ = torrent.toggle_first_last_piece_priority(hash).await;

            bot.send_message(
                msg.chat.id,
                format!(
                    "{} Sequential download mode toggled!\n\n\
                    ℹ️ Sequential mode downloads pieces in order for streaming.",
                    emoji::SUCCESS
                ),
            )
            .await?;
        }
        Err(err) => {
            tracing::error!("Error toggling sequential mode: {}", err);
            bot.send_message(msg.chat.id, format!("{} Error: {}", emoji::ERROR, err)).await?;
        }
    }

    Ok(())
}
