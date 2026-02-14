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
        let large_message = "A".repeat(300000); // Exceeds MAX_PEER_MESSAGE_SIZE
        assert!(validate_peer_message(&large_message).is_err());
    }

    #[test]
    fn test_validate_peer_message_content() {
        assert!(validate_peer_message("normal message").is_ok());
        assert!(validate_peer_message("v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\n").is_ok());
    }

    #[test]
    fn test_validate_peer_message_suspicious() {
        assert!(validate_peer_message("<script>alert('xss')</script>").is_err());
        assert!(validate_peer_message("javascript:alert(1)").is_err());
    }

    #[test]
    fn test_validate_info_hash() {
        assert!(validate_info_hash_hex("3b245504cf5f11bb3ee84da598e4e5b78e5c2dde").is_ok());
        assert!(validate_info_hash_hex("invalid!hash").is_err());
    }

    #[test]
    fn test_validate_remote_ip() {
        assert!(validate_remote_ip("192.168.1.1", false).is_err());
        assert!(validate_remote_ip("127.0.0.1", false).is_err());
        assert!(validate_remote_ip("8.8.8.8", false).is_ok());
        assert!(validate_remote_ip("192.168.1.1", true).is_ok());
    }
}