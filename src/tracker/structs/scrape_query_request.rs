//! Scrape request query parameters.

use serde::Deserialize;
use crate::tracker::structs::info_hash::InfoHash;

/// Parsed scrape request parameters.
///
/// This struct represents the query parameters from a BitTorrent tracker
/// scrape request. Scrape requests allow clients to query torrent statistics
/// without performing a full announce (BEP 48).
///
/// # Parameters
///
/// - `info_hash`: One or more 20-byte torrent info hashes to query
///
/// # Response Format
///
/// The scrape response contains a dictionary mapping each info hash to:
/// - `complete`: Number of seeders
/// - `incomplete`: Number of leechers
/// - `downloaded`: Number of times the torrent was completed
///
/// # Example Request
///
/// ```text
/// GET /scrape?info_hash=%xx...&info_hash=%yy...
/// ```
///
/// Multiple info_hash parameters can be provided to query multiple torrents
/// in a single request.
#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct ScrapeQueryRequest {
    /// List of info hashes to query statistics for.
    ///
    /// Multiple hashes can be provided in a single request.
    pub(crate) info_hash: Vec<InfoHash>,
}