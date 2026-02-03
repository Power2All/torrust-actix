//! User account identifier for private tracker functionality.

/// A 20-byte user identifier for private tracker authentication.
///
/// The user ID is used to identify user accounts in private tracker mode.
/// It is included in announce requests as a passkey and allows per-user
/// statistics tracking (upload, download, ratio, etc.).
///
/// # Structure
///
/// The user ID is exactly 20 bytes, typically generated from a passkey
/// string that users include in their tracker URLs.
///
/// # Example
///
/// ```rust
/// use torrust_actix::tracker::structs::user_id::UserId;
///
/// // Create from a 20-byte array
/// let user = UserId([0u8; 20]);
///
/// // Access the underlying bytes
/// let bytes: &[u8; 20] = &user.0;
/// ```
///
/// # Usage
///
/// In private tracker mode, the user ID is extracted from the announce URL:
/// `http://tracker.example.com/announce?passkey=<user_key>`
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct UserId(pub [u8; 20]);