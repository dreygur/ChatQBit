//! Token generation for secure streaming URLs

use sha2::{Sha256, Digest};

/// Generate a secure token for streaming a specific file
///
/// # Arguments
/// * `torrent_hash` - The torrent's info hash
/// * `file_index` - Index of the file within the torrent
/// * `secret` - Secret key for token generation
///
/// # Returns
/// * 16-character hexadecimal token
pub fn generate_stream_token(torrent_hash: &str, file_index: usize, secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(torrent_hash.as_bytes());
    hasher.update(file_index.to_string().as_bytes());
    hasher.update(secret.as_bytes());
    let result = hasher.finalize();

    // Take first 8 bytes (16 hex characters) for shorter URLs
    hex::encode(&result[..8])
}

/// Verify a stream token
///
/// # Arguments
/// * `token` - Token to verify
/// * `torrent_hash` - The torrent's info hash
/// * `file_index` - Index of the file within the torrent
/// * `secret` - Secret key for token generation
///
/// # Returns
/// * `true` if token is valid, `false` otherwise
pub fn verify_stream_token(token: &str, torrent_hash: &str, file_index: usize, secret: &str) -> bool {
    let expected_token = generate_stream_token(torrent_hash, file_index, secret);
    token == expected_token
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let token1 = generate_stream_token("abc123", 0, "secret");
        let token2 = generate_stream_token("abc123", 0, "secret");
        assert_eq!(token1, token2);
    }

    #[test]
    fn test_token_verification() {
        let token = generate_stream_token("abc123", 0, "secret");
        assert!(verify_stream_token(&token, "abc123", 0, "secret"));
        assert!(!verify_stream_token(&token, "abc123", 1, "secret"));
        assert!(!verify_stream_token(&token, "different", 0, "secret"));
    }
}
