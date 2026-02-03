//! Shared data context for API request handlers.

use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use std::sync::Arc;

/// Shared application data available to all API request handlers.
///
/// This struct is injected into Actix-web's application data and provides
/// request handlers with access to the tracker instance and API configuration.
///
/// # Thread Safety
///
/// Both fields are wrapped in `Arc` for safe sharing across multiple
/// worker threads in the Actix-web runtime.
///
/// # Example
///
/// ```rust,ignore
/// async fn handler(data: Data<Arc<ApiServiceData>>) -> HttpResponse {
///     let tracker = &data.torrent_tracker;
///     let config = &data.api_trackers_config;
///     // Use tracker and config...
/// }
/// ```
#[derive(Debug)]
pub struct ApiServiceData {
    /// Reference to the main tracker instance.
    pub torrent_tracker: Arc<TorrentTracker>,

    /// Configuration for this API server instance.
    pub api_trackers_config: Arc<ApiTrackersConfig>,
}