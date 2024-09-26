use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryToken {
    pub(crate) token: Option<String>,
}