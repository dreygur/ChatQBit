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

    #[test]
    fn test_torrent_actions_keyboard() {
        let keyboard = torrent_actions_keyboard("abc123");
        assert!(!keyboard.inline_keyboard.is_empty());
        assert_eq!(keyboard.inline_keyboard.len(), 4); // 4 rows of buttons
    }

    #[test]
    fn test_confirm_keyboard() {
        let keyboard = confirm_keyboard("delete", "abc123");
        assert_eq!(keyboard.inline_keyboard.len(), 1);
        assert_eq!(keyboard.inline_keyboard[0].len(), 2); // Yes and No buttons
    }

    #[test]
    fn test_pagination_keyboard() {
        // Single page - should only have refresh
        let keyboard = pagination_keyboard(0, 1);
        assert_eq!(keyboard.inline_keyboard.len(), 1);

        // Multiple pages - should have navigation + refresh
        let keyboard = pagination_keyboard(1, 3);
        assert_eq!(keyboard.inline_keyboard.len(), 2);
    }
}
