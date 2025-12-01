//! Inline keyboard builders for interactive bot menus
//!
//! This module provides helper functions to create inline keyboards
//! for better user experience with interactive buttons.

use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// Create an inline keyboard for torrent actions
///
/// # Arguments
/// * `hash` - The torrent hash to perform actions on
///
/// # Returns
/// An inline keyboard with common torrent operations
pub fn torrent_actions_keyboard(hash: &str) -> InlineKeyboardMarkup {
    let buttons = vec![
        vec![
            InlineKeyboardButton::callback("▶️ Resume", format!("resume:{}", hash)),
            InlineKeyboardButton::callback("⏸️ Pause", format!("pause:{}", hash)),
        ],
        vec![
            InlineKeyboardButton::callback("🔍 Info", format!("info:{}", hash)),
            InlineKeyboardButton::callback("🔄 Recheck", format!("recheck:{}", hash)),
        ],
        vec![
            InlineKeyboardButton::callback("⬆️ Top Priority", format!("topprio:{}", hash)),
            InlineKeyboardButton::callback("⬇️ Bottom Priority", format!("bottomprio:{}", hash)),
        ],
        vec![
            InlineKeyboardButton::callback("🗑️ Delete", format!("delete:{}", hash)),
            InlineKeyboardButton::callback("🗑️💥 Delete + Data", format!("deletedata:{}", hash)),
        ],
    ];

    InlineKeyboardMarkup::new(buttons)
}

/// Create a confirmation keyboard for destructive operations
///
/// # Arguments
/// * `action` - The action to confirm (e.g., "delete", "deletedata")
/// * `hash` - The torrent hash
///
/// # Returns
/// A simple Yes/No confirmation keyboard
pub fn confirm_keyboard(action: &str, hash: &str) -> InlineKeyboardMarkup {
    let buttons = vec![vec![
        InlineKeyboardButton::callback("✅ Yes, proceed", format!("confirm:{}:{}", action, hash)),
        InlineKeyboardButton::callback("❌ Cancel", "cancel".to_string()),
    ]];

    InlineKeyboardMarkup::new(buttons)
}

/// Create a main menu keyboard
pub fn main_menu_keyboard() -> InlineKeyboardMarkup {
    let buttons = vec![
        vec![
            InlineKeyboardButton::callback("📥 List Torrents", "cmd:list"),
            InlineKeyboardButton::callback("➕ Add Magnet", "cmd:magnet"),
        ],
        vec![
            InlineKeyboardButton::callback("📊 Transfer Info", "cmd:transferinfo"),
            InlineKeyboardButton::callback("⚡ Speed Limits", "cmd:speedlimits"),
        ],
        vec![
            InlineKeyboardButton::callback("📂 Categories", "cmd:categories"),
            InlineKeyboardButton::callback("🏷️ Tags", "cmd:tags"),
        ],
        vec![InlineKeyboardButton::callback("🔧 Version", "cmd:version")],
    ];

    InlineKeyboardMarkup::new(buttons)
}

/// Create pagination keyboard for torrent list
///
/// # Arguments
/// * `current_page` - Current page number (0-indexed)
/// * `total_pages` - Total number of pages
///
/// # Returns
/// Pagination controls with prev/next buttons
pub fn pagination_keyboard(current_page: usize, total_pages: usize) -> InlineKeyboardMarkup {
    let mut buttons = vec![];

    if total_pages > 1 {
        let mut nav_row = vec![];

        if current_page > 0 {
            nav_row.push(InlineKeyboardButton::callback(
                "⬅️ Previous",
                format!("page:{}", current_page - 1),
            ));
        }

        nav_row.push(InlineKeyboardButton::callback(
            format!("📄 {} / {}", current_page + 1, total_pages),
            "noop".to_string(),
        ));

        if current_page < total_pages - 1 {
            nav_row.push(InlineKeyboardButton::callback(
                "Next ➡️",
                format!("page:{}", current_page + 1),
            ));
        }

        buttons.push(nav_row);
    }

    // Add refresh button
    buttons.push(vec![InlineKeyboardButton::callback(
        "🔄 Refresh",
        "cmd:list",
    )]);

    InlineKeyboardMarkup::new(buttons)
}

/// Create a torrent selection keyboard for a specific action
///
/// Shows up to 10 torrents with buttons to perform the specified action
pub fn torrent_select_keyboard(
    torrents: &[qbit_rs::model::Torrent],
    action: &str,
    action_emoji: &str,
) -> InlineKeyboardMarkup {
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = torrents
        .iter()
        .take(10)
        .filter_map(|t| {
            let hash = t.hash.as_ref()?;
            let name = t.name.as_deref().unwrap_or("Unknown");
            // Truncate name to fit button
            let display_name = if name.len() > 25 {
                format!("{}...", &name[..22])
            } else {
                name.to_string()
            };
            Some(vec![InlineKeyboardButton::callback(
                format!("{} {}", action_emoji, display_name),
                format!("{}:{}", action, hash),
            )])
        })
        .collect();

    // Add cancel button
    buttons.push(vec![InlineKeyboardButton::callback("❌ Cancel", "cancel".to_string())]);

    InlineKeyboardMarkup::new(buttons)
}

/// Create a speed limit configuration keyboard
pub fn speed_limit_keyboard() -> InlineKeyboardMarkup {
    let buttons = vec![
        vec![
            InlineKeyboardButton::callback("📥 Set Download Limit", "setlimit:dl"),
            InlineKeyboardButton::callback("📤 Set Upload Limit", "setlimit:ul"),
        ],
        vec![
            InlineKeyboardButton::callback("🚫 Remove Download Limit", "removelimit:dl"),
            InlineKeyboardButton::callback("🚫 Remove Upload Limit", "removelimit:ul"),
        ],
        vec![InlineKeyboardButton::callback("◀️ Back to Menu", "cmd:menu")],
    ];

    InlineKeyboardMarkup::new(buttons)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test torrent
    fn create_test_torrent(hash: Option<&str>, name: Option<&str>) -> qbit_rs::model::Torrent {
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
            progress: None,
            ratio: None,
            ratio_limit: None,
            save_path: None,
            seeding_time: None,
            seeding_time_limit: None,
            seen_complete: None,
            seq_dl: None,
            size: None,
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
    fn test_torrent_actions_keyboard() {
        let keyboard = torrent_actions_keyboard("abc123");
        assert!(!keyboard.inline_keyboard.is_empty());
        assert_eq!(keyboard.inline_keyboard.len(), 4); // 4 rows of buttons

        // Check callback data format
        let first_row = &keyboard.inline_keyboard[0];
        assert_eq!(first_row.len(), 2); // Resume and Pause
    }

    #[test]
    fn test_confirm_keyboard() {
        let keyboard = confirm_keyboard("delete", "abc123");
        assert_eq!(keyboard.inline_keyboard.len(), 1);
        assert_eq!(keyboard.inline_keyboard[0].len(), 2); // Yes and No buttons

        // Test with different action
        let keyboard = confirm_keyboard("deletedata", "xyz789");
        assert_eq!(keyboard.inline_keyboard.len(), 1);
    }

    #[test]
    fn test_main_menu_keyboard() {
        let keyboard = main_menu_keyboard();
        assert!(!keyboard.inline_keyboard.is_empty());
        // Should have multiple rows
        assert!(keyboard.inline_keyboard.len() >= 3);
    }

    #[test]
    fn test_pagination_keyboard() {
        // Single page - should only have refresh
        let keyboard = pagination_keyboard(0, 1);
        assert_eq!(keyboard.inline_keyboard.len(), 1);

        // First page of multiple - should have next + refresh
        let keyboard = pagination_keyboard(0, 3);
        assert_eq!(keyboard.inline_keyboard.len(), 2);
        assert!(keyboard.inline_keyboard[0].len() >= 2); // Page counter + Next

        // Middle page - should have prev + page + next + refresh
        let keyboard = pagination_keyboard(1, 3);
        assert_eq!(keyboard.inline_keyboard.len(), 2);
        assert_eq!(keyboard.inline_keyboard[0].len(), 3); // Prev + Page + Next

        // Last page - should have prev + refresh
        let keyboard = pagination_keyboard(2, 3);
        assert_eq!(keyboard.inline_keyboard.len(), 2);
        assert!(keyboard.inline_keyboard[0].len() >= 2); // Prev + Page counter
    }

    #[test]
    fn test_torrent_select_keyboard() {
        // Empty list
        let empty: Vec<qbit_rs::model::Torrent> = vec![];
        let keyboard = torrent_select_keyboard(&empty, "resume", "▶️");
        assert_eq!(keyboard.inline_keyboard.len(), 1); // Just cancel button

        // Single torrent
        let torrents = vec![create_test_torrent(Some("abc123"), Some("Test Torrent"))];
        let keyboard = torrent_select_keyboard(&torrents, "resume", "▶️");
        assert_eq!(keyboard.inline_keyboard.len(), 2); // 1 torrent + cancel

        // Multiple torrents
        let torrents: Vec<qbit_rs::model::Torrent> = (0..5)
            .map(|i| create_test_torrent(Some(&format!("hash{}", i)), Some(&format!("Torrent {}", i))))
            .collect();
        let keyboard = torrent_select_keyboard(&torrents, "pause", "⏸️");
        assert_eq!(keyboard.inline_keyboard.len(), 6); // 5 torrents + cancel

        // Long name truncation
        let torrents = vec![create_test_torrent(
            Some("abc123"),
            Some("This is a very long torrent name that should be truncated"),
        )];
        let keyboard = torrent_select_keyboard(&torrents, "info", "🔍");
        // Button text should be truncated
        assert_eq!(keyboard.inline_keyboard.len(), 2);
    }

    #[test]
    fn test_torrent_select_keyboard_max_10() {
        // More than 10 torrents - should only show 10
        let torrents: Vec<qbit_rs::model::Torrent> = (0..15)
            .map(|i| create_test_torrent(Some(&format!("hash{:02}", i)), Some(&format!("Torrent {}", i))))
            .collect();
        let keyboard = torrent_select_keyboard(&torrents, "stream", "🎬");
        assert_eq!(keyboard.inline_keyboard.len(), 11); // 10 torrents + cancel
    }

    #[test]
    fn test_torrent_select_keyboard_missing_hash() {
        // Torrent without hash should be skipped
        let torrents = vec![
            create_test_torrent(None, Some("No Hash")),
            create_test_torrent(Some("abc123"), Some("Has Hash")),
        ];
        let keyboard = torrent_select_keyboard(&torrents, "files", "📁");
        assert_eq!(keyboard.inline_keyboard.len(), 2); // 1 valid torrent + cancel
    }

    #[test]
    fn test_speed_limit_keyboard() {
        let keyboard = speed_limit_keyboard();
        assert!(!keyboard.inline_keyboard.is_empty());
        assert!(keyboard.inline_keyboard.len() >= 2); // At least set + remove rows
    }
}
