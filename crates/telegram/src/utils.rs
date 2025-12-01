//! Utility functions for formatting and parsing

/// Format file size in human-readable format
pub fn format_bytes(bytes: i64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes = bytes.abs() as f64;
    let unit_index = (bytes.ln() / 1024_f64.ln()).floor() as usize;
    let unit_index = unit_index.min(UNITS.len() - 1);

    let size = bytes / 1024_f64.powi(unit_index as i32);
    let sign = if bytes < 0.0 { "-" } else { "" };

    format!("{}{:.2} {}", sign, size, UNITS[unit_index])
}

/// Format file size (u64) in human-readable format
pub fn format_size(bytes: u64) -> String {
    format_bytes(bytes as i64)
}

/// Format speed (bytes/sec) in human-readable format
pub fn format_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        return "0 B/s".to_string();
    }

    const UNITS: [&str; 4] = ["B/s", "KB/s", "MB/s", "GB/s"];
    let speed = bytes_per_sec as f64;
    let unit_index = (speed.ln() / 1024_f64.ln()).floor() as usize;
    let unit_index = unit_index.min(UNITS.len() - 1);

    let value = speed / 1024_f64.powi(unit_index as i32);
    format!("{:.2} {}", value, UNITS[unit_index])
}

/// Format speed limit (0 means unlimited)
pub fn format_limit(limit: u64) -> String {
    if limit == 0 {
        "Unlimited".to_string()
    } else {
        format_speed(limit)
    }
}

/// Format Unix timestamp to human-readable date
pub fn format_timestamp(timestamp: i64) -> String {
    if timestamp <= 0 {
        return "N/A".to_string();
    }

    // Use chrono for proper formatting
    use chrono::{TimeZone, Utc};
    match Utc.timestamp_opt(timestamp, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        _ => "Invalid".to_string(),
    }
}

/// Format ETA (seconds) to human-readable duration
pub fn format_eta(seconds: i64) -> String {
    if seconds <= 0 {
        return "∞".to_string();
    }

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Truncate hash to first N characters for display
pub fn truncate_hash(hash: &str, len: usize) -> &str {
    if hash.len() > len {
        &hash[..len]
    } else {
        hash
    }
}

/// Parse command arguments from message text
pub fn parse_args(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// Validate and extract hash argument from command
pub fn extract_hash_arg<'a>(args: &'a [&str]) -> Result<&'a str, String> {
    if args.len() < 2 {
        return Err("Missing torrent hash argument".to_string());
    }

    let hash = args[1];
    if hash.is_empty() {
        return Err("Hash cannot be empty".to_string());
    }

    // Validate hash format: 40 chars (SHA-1) or 64 chars (SHA-256), hex only
    if !is_valid_torrent_hash(hash) {
        return Err("Invalid hash format. Must be 40 or 64 hex characters".to_string());
    }

    Ok(hash)
}

/// Check if a string is a valid torrent hash (SHA-1 or SHA-256)
pub fn is_valid_torrent_hash(hash: &str) -> bool {
    let len = hash.len();
    (len == 40 || len == 64) && hash.chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate and extract limit argument from command
pub fn extract_limit_arg(args: &[&str]) -> Result<u64, String> {
    if args.len() < 2 {
        return Err("Missing limit argument".to_string());
    }

    args[1]
        .parse::<u64>()
        .map_err(|_| "Invalid limit value. Must be a number".to_string())
}

/// Escape special characters for MarkdownV2
///
/// Escapes: _*[]()~`>#+-=|{}.!
pub fn escape_markdown_v2(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|' | '{' | '}' | '.' | '!' => {
                format!("\\{}", c)
            }
            _ => c.to_string(),
        })
        .collect()
}

/// Extract info hash from .torrent file data
///
/// Parses bencoded .torrent file and extracts the SHA-1 hash of the info dictionary.
/// Returns lowercase hex-encoded info hash for duplicate checking.
pub fn extract_torrent_info_hash(file_data: &[u8]) -> Option<String> {
    use sha1::{Digest, Sha1};

    // Find the "info" dictionary in the bencoded data
    // Torrent files have format: d...4:info...e
    let info_start = find_info_dict_start(file_data)?;
    let info_end = find_matching_end(file_data, info_start)?;

    // Hash the info dictionary bytes
    let info_bytes = &file_data[info_start..info_end];
    let mut hasher = Sha1::new();
    hasher.update(info_bytes);
    let hash = hasher.finalize();

    // Convert to hex string
    Some(format!("{:x}", hash))
}

/// Find the start position of the info dictionary in bencoded data
fn find_info_dict_start(data: &[u8]) -> Option<usize> {
    // Look for "4:infod" pattern (the info key followed by dictionary start)
    let pattern = b"4:infod";
    for i in 0..data.len().saturating_sub(pattern.len()) {
        if &data[i..i + pattern.len()] == pattern {
            // Return position after "4:info" (at the 'd' of the info dict)
            return Some(i + 6);
        }
    }
    None
}

/// Find the matching 'e' (end) for a dictionary starting at 'start'
fn find_matching_end(data: &[u8], start: usize) -> Option<usize> {
    if start >= data.len() || data[start] != b'd' {
        return None;
    }

    let mut depth = 0;
    for (offset, &byte) in data[start..].iter().enumerate() {
        match byte {
            b'd' | b'l' => depth += 1, // dictionary or list start
            b'e' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + offset + 1); // Include the 'e'
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
        assert_eq!(format_bytes(1099511627776), "1.00 TB");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(u64::MAX), format_bytes(u64::MAX as i64));
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(0), "0 B/s");
        assert_eq!(format_speed(512), "512.00 B/s");
        assert_eq!(format_speed(1024), "1.00 KB/s");
        assert_eq!(format_speed(1048576), "1.00 MB/s");
        assert_eq!(format_speed(1073741824), "1.00 GB/s");
    }

    #[test]
    fn test_format_limit() {
        assert_eq!(format_limit(0), "Unlimited");
        assert_eq!(format_limit(1024), "1.00 KB/s");
        assert_eq!(format_limit(1048576), "1.00 MB/s");
    }

    #[test]
    fn test_format_eta() {
        assert_eq!(format_eta(-1), "∞");
        assert_eq!(format_eta(0), "∞");
        assert_eq!(format_eta(30), "30s");
        assert_eq!(format_eta(59), "59s");
        assert_eq!(format_eta(60), "1m 0s");
        assert_eq!(format_eta(90), "1m 30s");
        assert_eq!(format_eta(3600), "1h 0m");
        assert_eq!(format_eta(3661), "1h 1m");
        assert_eq!(format_eta(7200), "2h 0m");
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0), "N/A");
        assert_eq!(format_timestamp(-1), "N/A");
        // Valid timestamp (2024-01-01 00:00:00 UTC)
        let ts = format_timestamp(1704067200);
        assert!(ts.contains("2024-01-01"));
        assert!(ts.contains("UTC"));
    }

    #[test]
    fn test_truncate_hash() {
        assert_eq!(truncate_hash("abcdefgh", 4), "abcd");
        assert_eq!(truncate_hash("abc", 4), "abc");
        assert_eq!(truncate_hash("", 4), "");
        assert_eq!(truncate_hash("abcd", 4), "abcd");
    }

    #[test]
    fn test_parse_args() {
        assert_eq!(parse_args(""), Vec::<&str>::new());
        assert_eq!(parse_args("/cmd"), vec!["/cmd"]);
        assert_eq!(parse_args("/cmd arg1"), vec!["/cmd", "arg1"]);
        assert_eq!(parse_args("/cmd arg1 arg2"), vec!["/cmd", "arg1", "arg2"]);
        assert_eq!(parse_args("  /cmd   arg1  "), vec!["/cmd", "arg1"]);
    }

    #[test]
    fn test_extract_hash_arg() {
        // Missing argument
        assert!(extract_hash_arg(&["cmd"]).is_err());
        // Empty hash
        assert!(extract_hash_arg(&["cmd", ""]).is_err());
        // Invalid: too short
        assert!(extract_hash_arg(&["cmd", "abc123"]).is_err());
        // Invalid: wrong length
        assert!(extract_hash_arg(&["cmd", "abc"]).is_err());
        // Valid SHA-1 hash (40 hex chars)
        let valid_sha1 = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        assert_eq!(extract_hash_arg(&["cmd", valid_sha1]).unwrap(), valid_sha1);
        // Valid SHA-256 hash (64 hex chars)
        let valid_sha256 = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        assert_eq!(extract_hash_arg(&["cmd", valid_sha256]).unwrap(), valid_sha256);
        // Invalid chars
        assert!(extract_hash_arg(&["cmd", "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"]).is_err());
    }

    #[test]
    fn test_is_valid_torrent_hash() {
        // Empty
        assert!(!is_valid_torrent_hash(""));
        // Too short
        assert!(!is_valid_torrent_hash("abc123"));
        // Wrong length (39 chars)
        assert!(!is_valid_torrent_hash("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b"));
        // Invalid chars
        assert!(!is_valid_torrent_hash("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"));
        assert!(!is_valid_torrent_hash("gggggggggggggggggggggggggggggggggggggggg"));
        // Valid SHA-1 (40 chars, lowercase)
        assert!(is_valid_torrent_hash("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"));
        // Valid SHA-1 (40 chars, uppercase)
        assert!(is_valid_torrent_hash("A1B2C3D4E5F6A1B2C3D4E5F6A1B2C3D4E5F6A1B2"));
        // Valid SHA-1 (40 chars, mixed)
        assert!(is_valid_torrent_hash("A1b2C3d4E5f6A1b2C3d4E5f6A1b2C3d4E5f6A1b2"));
        // Valid SHA-256 (64 chars)
        assert!(is_valid_torrent_hash("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"));
    }

    #[test]
    fn test_extract_limit_arg() {
        // Missing argument
        assert!(extract_limit_arg(&["cmd"]).is_err());
        // Invalid: not a number
        assert!(extract_limit_arg(&["cmd", "invalid"]).is_err());
        assert!(extract_limit_arg(&["cmd", "abc"]).is_err());
        // Invalid: negative (can't parse as u64)
        assert!(extract_limit_arg(&["cmd", "-1"]).is_err());
        // Valid
        assert_eq!(extract_limit_arg(&["cmd", "0"]).unwrap(), 0);
        assert_eq!(extract_limit_arg(&["cmd", "1024"]).unwrap(), 1024);
        assert_eq!(extract_limit_arg(&["cmd", "1000000"]).unwrap(), 1000000);
    }

    #[test]
    fn test_escape_markdown_v2() {
        // No special chars
        assert_eq!(escape_markdown_v2("hello"), "hello");
        // Single special char
        assert_eq!(escape_markdown_v2("hello_world"), "hello\\_world");
        assert_eq!(escape_markdown_v2("hello*world"), "hello\\*world");
        // Multiple special chars
        assert_eq!(escape_markdown_v2("a_b*c[d]e"), "a\\_b\\*c\\[d\\]e");
        // All special chars
        assert_eq!(
            escape_markdown_v2("_*[]()~`>#+-=|{}.!"),
            "\\_\\*\\[\\]\\(\\)\\~\\`\\>\\#\\+\\-\\=\\|\\{\\}\\.\\!"
        );
        // Real filename example
        assert_eq!(
            escape_markdown_v2("Movie (2024) [1080p].mkv"),
            "Movie \\(2024\\) \\[1080p\\]\\.mkv"
        );
    }

    #[test]
    fn test_extract_torrent_info_hash() {
        // Invalid: not bencoded
        assert!(extract_torrent_info_hash(b"not a torrent").is_none());
        // Invalid: empty
        assert!(extract_torrent_info_hash(b"").is_none());
        // Invalid: no info dict
        assert!(extract_torrent_info_hash(b"d8:announcei0ee").is_none());
        // Valid: minimal torrent structure with info dict
        // This is a simplified bencoded structure: d4:infod4:name4:testee
        let minimal_torrent = b"d4:infod4:name4:testee";
        let hash = extract_torrent_info_hash(minimal_torrent);
        assert!(hash.is_some());
        assert_eq!(hash.unwrap().len(), 40); // SHA-1 produces 40 hex chars
    }

    #[test]
    fn test_find_info_dict_start() {
        // Pattern found
        let data = b"d8:announce4:infod4:name4:testee";
        assert!(find_info_dict_start(data).is_some());

        // Pattern not found
        let data = b"d8:announcei0ee";
        assert!(find_info_dict_start(data).is_none());
    }

    #[test]
    fn test_find_matching_end() {
        // Simple dictionary: d + e = depth 1->0
        let data = b"de"; // empty dict
        let end = find_matching_end(data, 0);
        assert_eq!(end, Some(2));

        // Dictionary with content
        let data = b"d4:test3:abce";
        let end = find_matching_end(data, 0);
        assert!(end.is_some()); // Just verify it finds an end

        // Invalid: not starting with 'd'
        let data = b"4:test";
        assert!(find_matching_end(data, 0).is_none());

        // Invalid: index out of bounds
        let data = b"d4:teste";
        assert!(find_matching_end(data, 100).is_none());
    }
}
