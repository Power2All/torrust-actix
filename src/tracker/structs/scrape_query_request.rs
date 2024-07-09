use serde::Deserialize;
use crate::tracker::structs::info_hash::InfoHash;

#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct ScrapeQueryRequest {
    pub(crate) info_hash: Vec<InfoHash>,
}
