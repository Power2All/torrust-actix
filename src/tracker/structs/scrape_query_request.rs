use serde::Deserialize;
use crate::tracker::structs::info_hash::InfoHash;

#[derive(Deserialize, Clone, Debug)]
pub struct ScrapeQueryRequest {
    pub(crate) info_hash: Vec<InfoHash>,
}