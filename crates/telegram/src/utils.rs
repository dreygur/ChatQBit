/// Utility functions for formatting and parsing

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
    if timestamp == 0 {
        return "Never".to_string();
    }

    use std::time::{Duration, UNIX_EPOCH};
    let duration = Duration::from_secs(timestamp as u64);
    let datetime = UNIX_EPOCH + duration;

    // For simplicity, just return timestamp
    // In production, use chrono crate for proper formatting
    timestamp.to_string()
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

    Ok(hash)
}

/// Validate and extract limit argument from command
pub fn extract_limit_arg(args: &[&str]) -> Result<u64, String> {
    if args.len() < 2 {
        return Err("Missing limit argument".to_string());
    }

    args[1]
        .parse::<u64>()
        .map_err(|_| "Invalid limit value. Must be a number.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(0), "0 B/s");
        assert_eq!(format_speed(1024), "1.00 KB/s");
        assert_eq!(format_speed(1048576), "1.00 MB/s");
    }

    #[test]
    fn test_format_eta() {
        assert_eq!(format_eta(0), "∞");
        assert_eq!(format_eta(30), "30s");
        assert_eq!(format_eta(90), "1m 30s");
        assert_eq!(format_eta(3661), "1h 1m");
    }

    #[test]
    fn test_truncate_hash() {
        assert_eq!(truncate_hash("abcdefgh", 4), "abcd");
        assert_eq!(truncate_hash("abc", 4), "abc");
    }

    #[test]
    fn test_extract_hash_arg() {
        assert!(extract_hash_arg(&["cmd"]).is_err());
        assert!(extract_hash_arg(&["cmd", ""]).is_err());
        assert_eq!(extract_hash_arg(&["cmd", "abc123"]).unwrap(), "abc123");
    }

    #[test]
    fn test_extract_limit_arg() {
        assert!(extract_limit_arg(&["cmd"]).is_err());
        assert!(extract_limit_arg(&["cmd", "invalid"]).is_err());
        assert_eq!(extract_limit_arg(&["cmd", "1024"]).unwrap(), 1024);
    }
}
