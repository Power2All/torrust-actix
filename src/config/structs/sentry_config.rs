use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SentryConfig {
    pub enabled: bool,
    pub dsn: String,
    pub debug: bool,
    pub sample_rate: f32,
    pub max_breadcrumbs: usize,
    pub attach_stacktrace: bool,
    pub send_default_pii: bool,
    pub traces_sample_rate: f32
}