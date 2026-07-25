#![allow(clippy::module_inception)]

#[cfg(test)]
mod security_tests {
    use crate::security::security::*;

    #[test]
    fn test_generate_api_key_length() {
        let key = generate_secure_api_key();
        assert!(key.len() >= 32);
    }

    #[test]
    fn test_api_key_strength_valid() {
        assert!(validate_api_key_strength("ThisIsAVeryStrongKey123!@#abcXYZ456"));
        assert!(validate_api_key_strength("abc123DEF456ghi789JKLmnopqrsTUV1234!"));
    }

    #[test]
    fn test_api_key_weak() {
        assert!(!validate_api_key_strength("weak"));
        assert!(!validate_api_key_strength("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    }

    #[test]
    fn test_constant_time_eq_equal() {
        assert!(constant_time_eq("test_key", "test_key"));
    }

    #[test]
    fn test_constant_time_eq_not_equal() {
        assert!(!constant_time_eq("test_key", "different_key"));
    }

    #[test]
    fn test_constant_time_eq_different_length() {
        assert!(!constant_time_eq("test", "test_key"));
        assert!(!constant_time_eq("test_key", "test"));
        assert!(!constant_time_eq("", "test"));
        assert!(!constant_time_eq("test", ""));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn test_validate_file_path_reject_traversal() {
        assert!(validate_file_path("../../../etc/passwd").is_err());
        assert!(validate_file_path("./config").is_err());
        assert!(validate_file_path(".\\config").is_err());
    }

    #[test]
    fn test_validate_file_path_reject_absolute() {
        assert!(validate_file_path("/etc/cert.pem").is_err());
        assert!(validate_file_path("C:\\certs\\cert.pem").is_err());
    }

    #[test]
    fn test_validate_file_path_accept_valid() {
        assert!(validate_file_path("certs/cert.pem").is_ok());
        assert!(validate_file_path("cert.pem").is_ok());
    }

    #[test]
    fn test_validate_peer_message_size() {
        let large_message = "A".repeat(300_000);
        assert!(validate_peer_message(&large_message).is_err());
    }

    #[test]
    fn test_validate_peer_message_content() {
        assert!(validate_peer_message("normal message").is_ok());
        // An SDP body is opaque to the tracker: only its size is bounded.
        assert!(validate_peer_message("v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\na=candidate:1 1 UDP 1 10.0.0.1 9 typ host\r\n").is_ok());
    }

    #[test]
    fn test_validate_info_hash() {
        assert!(validate_info_hash_hex("3b245504cf5f11bb3ee84da598e4e5b78e5c2dde").is_ok());
        assert!(validate_info_hash_hex("3B245504CF5F11BB3EE84DA598E4E5B78E5C2DDE").is_ok());
        assert!(validate_info_hash_hex("invalid!hash").is_err());
        // Only exactly 40 hex characters may pass.
        assert!(validate_info_hash_hex("3b245504cf5f11bb3ee84da598e4e5b78e5c2dd").is_err());
        assert!(validate_info_hash_hex("3b245504cf5f11bb3ee84da598e4e5b78e5c2ddez").is_err());
        assert!(validate_info_hash_hex("not hex but twenty five ch").is_err());
    }

    #[test]
    fn test_validate_peer_id_hex() {
        assert!(validate_peer_id_hex("2d7142343235302d6b6568786f6272736e397a").is_err());
        assert!(validate_peer_id_hex("2d7142343235302d6b6568786f6272736e397a30").is_ok());
        assert!(validate_peer_id_hex("this is not hex at all but is long enough").is_err());
    }

    #[test]
    fn test_validate_query_string_length() {
        assert!(validate_query_string_length(&"a".repeat(MAX_QUERY_STRING_LENGTH)).is_ok());
        assert!(validate_query_string_length(&"a".repeat(MAX_QUERY_STRING_LENGTH + 1)).is_err());
    }

    #[test]
    fn test_validate_remote_ip() {
        assert!(validate_remote_ip("192.168.1.1", false).is_err());
        assert!(validate_remote_ip("127.0.0.1", false).is_err());
        assert!(validate_remote_ip("8.8.8.8", false).is_ok());
        assert!(validate_remote_ip("192.168.1.1", true).is_ok());
    }
}