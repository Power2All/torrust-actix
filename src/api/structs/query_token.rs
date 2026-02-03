//! API authentication token query parameter.

use serde::{Deserialize, Serialize};

/// Query parameter struct for API authentication.
///
/// Used to extract the authentication token from the URL query string.
/// All API endpoints require a valid token for authentication.
///
/// # Example
///
/// Request: `GET /api/torrents?token=my_secret_key`
///
/// ```rust,ignore
/// async fn handler(query: Query<QueryToken>) -> HttpResponse {
///     if let Some(token) = &query.token {
///         // Validate token...
///     }
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryToken {
    /// The API authentication token (optional in struct, required for access).
    pub(crate) token: Option<String>,
}