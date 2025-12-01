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
    fn test_token_generation_deterministic() {
        // Same inputs should produce same token
        let token1 = generate_stream_token("abc123", 0, "secret");
        let token2 = generate_stream_token("abc123", 0, "secret");
        assert_eq!(token1, token2);
    }

    #[test]
    fn test_token_generation_different_hash() {
        let token1 = generate_stream_token("abc123", 0, "secret");
        let token2 = generate_stream_token("xyz789", 0, "secret");
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_token_generation_different_index() {
        let token1 = generate_stream_token("abc123", 0, "secret");
        let token2 = generate_stream_token("abc123", 1, "secret");
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_token_generation_different_secret() {
        let token1 = generate_stream_token("abc123", 0, "secret1");
        let token2 = generate_stream_token("abc123", 0, "secret2");
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_token_length() {
        let token = generate_stream_token("abc123", 0, "secret");
        // Token should be 16 hex characters (8 bytes)
        assert_eq!(token.len(), 16);
        // Token should be valid hex
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_token_verification_valid() {
        let token = generate_stream_token("abc123", 0, "secret");
        assert!(verify_stream_token(&token, "abc123", 0, "secret"));
    }

    #[test]
    fn test_token_verification_wrong_hash() {
        let token = generate_stream_token("abc123", 0, "secret");
        assert!(!verify_stream_token(&token, "different", 0, "secret"));
    }

    #[test]
    fn test_token_verification_wrong_index() {
        let token = generate_stream_token("abc123", 0, "secret");
        assert!(!verify_stream_token(&token, "abc123", 1, "secret"));
    }

    #[test]
    fn test_token_verification_wrong_secret() {
        let token = generate_stream_token("abc123", 0, "secret");
        assert!(!verify_stream_token(&token, "abc123", 0, "wrong_secret"));
    }

    #[test]
    fn test_token_verification_invalid_token() {
        assert!(!verify_stream_token("invalid", "abc123", 0, "secret"));
        assert!(!verify_stream_token("", "abc123", 0, "secret"));
    }

    #[test]
    fn test_token_with_empty_inputs() {
        // Empty hash
        let token1 = generate_stream_token("", 0, "secret");
        assert_eq!(token1.len(), 16);

        // Empty secret
        let token2 = generate_stream_token("abc123", 0, "");
        assert_eq!(token2.len(), 16);

        // Both should be different
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_token_with_large_index() {
        let token = generate_stream_token("abc123", usize::MAX, "secret");
        assert_eq!(token.len(), 16);
        assert!(verify_stream_token(&token, "abc123", usize::MAX, "secret"));
    }

    #[test]
    fn test_token_with_special_characters() {
        // Hash with special characters
        let token = generate_stream_token("abc123!@#$%^&*()", 0, "secret!@#");
        assert_eq!(token.len(), 16);
        assert!(verify_stream_token(&token, "abc123!@#$%^&*()", 0, "secret!@#"));
    }

    #[test]
    fn test_token_with_unicode() {
        let token = generate_stream_token("abc123", 0, "秘密");
        assert_eq!(token.len(), 16);
        assert!(verify_stream_token(&token, "abc123", 0, "秘密"));
    }
}
