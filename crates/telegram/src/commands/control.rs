//! Torrent control commands (resume, pause, delete, recheck, etc.)

use crate::constants::usage;
use crate::handlers::execute_hash_command;
use crate::types::HandlerResult;
use teloxide::prelude::*;
use torrent::TorrentApi;

/// Resume/start torrents
pub async fn resume(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot, msg, torrent,
        usage::RESUME,
        "Torrent(s) resumed successfully!",
        |api, hash| async move { api.start_torrents(&hash).await },
    )
    .await
}

/// Pause/stop torrents
pub async fn pause(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot, msg, torrent,
        usage::PAUSE,
        "Torrent(s) paused successfully!",
        |api, hash| async move { api.stop_torrents(&hash).await },
    )
    .await
}

/// Delete torrent (keep files)
pub async fn delete(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot, msg, torrent,
        usage::DELETE,
        "Torrent deleted (files kept)!",
        |api, hash| async move { api.delete_torrents(&hash, false).await },
    )
    .await
}

/// Delete torrent with files
pub async fn delete_data(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot, msg, torrent,
        usage::DELETE_DATA,
        "Torrent and files deleted!",
        |api, hash| async move { api.delete_torrents(&hash, true).await },
    )
    .await
}

/// Recheck torrent
pub async fn recheck(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot, msg, torrent,
        usage::RECHECK,
        "Torrent recheck started!",
        |api, hash| async move { api.recheck_torrents(&hash).await },
    )
    .await
}

/// Reannounce torrent to trackers
pub async fn reannounce(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot, msg, torrent,
        usage::REANNOUNCE,
        "Torrent reannounced to trackers!",
        |api, hash| async move { api.reannounce_torrents(&hash).await },
    )
    .await
}

/// Set torrent priority to top
pub async fn top_prio(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot, msg, torrent,
        usage::TOP_PRIO,
        "Torrent priority set to top!",
        |api, hash| async move { api.set_top_priority(&hash).await },
    )
    .await
}

/// Set torrent priority to bottom
pub async fn bottom_prio(bot: Bot, msg: Message, torrent: TorrentApi) -> HandlerResult {
    execute_hash_command(
        bot, msg, torrent,
        usage::BOTTOM_PRIO,
        "Torrent priority set to bottom!",
        |api, hash| async move { api.set_bottom_priority(&hash).await },
    )
    .await
}
