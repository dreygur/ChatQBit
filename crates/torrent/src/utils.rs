//! Utility functions for torrent operations

use std::collections::HashSet;

/// Result of duplicate check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuplicateCheckResult {
    /// No duplicates found
    NoDuplicates,
    /// Duplicates found with their info hashes
    Duplicates(Vec<String>),
}

/// Extract info hash from magnet link
///
/// Magnet links have the format: magnet:?xt=urn:btih:HASH&...
/// This function extracts the info hash (HASH) from the link
pub fn extract_info_hash(magnet_url: &str) -> Option<String> {
    if !magnet_url.starts_with("magnet:?") {
        return None;
    }

    // Find the xt parameter which contains the info hash
    for param in magnet_url.split('&') {
        if param.starts_with("xt=urn:btih:") || param.contains("xt=urn:btih:") {
            // Extract hash after "xt=urn:btih:"
            if let Some(hash_start) = param.find("xt=urn:btih:") {
                let hash = &param[hash_start + 12..];
                // Hash can be 32 or 40 characters (base32 or hex)
                // Take until next parameter or end
                let hash_end = hash.find('&').unwrap_or(hash.len());
                let extracted_hash = &hash[..hash_end];

                if !extracted_hash.is_empty() {
                    return Some(extracted_hash.to_lowercase());
                }
            }
        }
    }

    None
}

/// Check if any of the provided URLs are duplicates of existing torrents
///
/// # Arguments
/// * `urls` - URLs to check (magnet links or torrent URLs)
/// * `existing_hashes` - Set of existing torrent hashes in the client
///
/// # Returns
/// `DuplicateCheckResult` indicating if duplicates were found
pub fn check_duplicates(urls: &[String], existing_hashes: &HashSet<String>) -> DuplicateCheckResult {
    let mut duplicates = Vec::new();

    for url in urls {
        if let Some(hash) = extract_info_hash(url) {
            // Check both lowercase and uppercase variants
            if existing_hashes.contains(&hash) || existing_hashes.contains(&hash.to_uppercase()) {
                duplicates.push(hash);
            }
        }
    }

    if duplicates.is_empty() {
        DuplicateCheckResult::NoDuplicates
    } else {
        DuplicateCheckResult::Duplicates(duplicates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_info_hash() {
        // Standard magnet link
        let magnet = "magnet:?xt=urn:btih:abc123def456&dn=Test";
        assert_eq!(extract_info_hash(magnet), Some("abc123def456".to_string()));

        // Magnet with multiple parameters
        let magnet = "magnet:?dn=Test&xt=urn:btih:abc123def456&tr=http://tracker.example.com";
        assert_eq!(extract_info_hash(magnet), Some("abc123def456".to_string()));

        // Invalid magnet
        assert_eq!(extract_info_hash("http://example.com/file.torrent"), None);
        assert_eq!(extract_info_hash("not a magnet link"), None);

        // Uppercase hash should be lowercased
        let magnet = "magnet:?xt=urn:btih:ABC123DEF456";
        assert_eq!(extract_info_hash(magnet), Some("abc123def456".to_string()));
    }

    #[test]
    fn test_check_duplicates() {
        let mut existing = HashSet::new();
        existing.insert("abc123".to_string());
        existing.insert("def456".to_string());

        // No duplicates
        let urls = vec!["magnet:?xt=urn:btih:xyz789".to_string()];
        assert_eq!(check_duplicates(&urls, &existing), DuplicateCheckResult::NoDuplicates);

        // One duplicate
        let urls = vec!["magnet:?xt=urn:btih:abc123".to_string()];
        match check_duplicates(&urls, &existing) {
            DuplicateCheckResult::Duplicates(hashes) => {
                assert_eq!(hashes.len(), 1);
                assert_eq!(hashes[0], "abc123");
            }
            _ => panic!("Expected duplicates"),
        }

        // Mixed duplicates and new
        let urls = vec![
            "magnet:?xt=urn:btih:abc123".to_string(),
            "magnet:?xt=urn:btih:xyz789".to_string(),
        ];
        match check_duplicates(&urls, &existing) {
            DuplicateCheckResult::Duplicates(hashes) => {
                assert_eq!(hashes.len(), 1);
                assert_eq!(hashes[0], "abc123");
            }
            _ => panic!("Expected duplicates"),
        }
    }

    #[test]
    fn test_check_duplicates_case_insensitive() {
        let mut existing = HashSet::new();
        existing.insert("ABC123".to_string());

        // Lowercase hash should match uppercase existing
        let urls = vec!["magnet:?xt=urn:btih:abc123".to_string()];
        match check_duplicates(&urls, &existing) {
            DuplicateCheckResult::Duplicates(hashes) => {
                assert_eq!(hashes.len(), 1);
            }
            _ => panic!("Expected duplicates"),
        }
    }
}
