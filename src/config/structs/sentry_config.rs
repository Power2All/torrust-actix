use serde::{
    Deserialize,
    Serialize
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SentryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub dsn: String,
    #[serde(default)]
    pub debug: bool,
    #[serde(default = "crate::config::impls::sentry_config::default_sample_rate")]
    pub sample_rate: f32,
    #[serde(default = "crate::config::impls::sentry_config::default_max_breadcrumbs")]
    pub max_breadcrumbs: usize,
    #[serde(default = "crate::config::impls::sentry_config::default_attach_stacktrace")]
    pub attach_stacktrace: bool,
    #[serde(default)]
    pub send_default_pii: bool,
    #[serde(default = "crate::config::impls::sentry_config::default_traces_sample_rate")]
    pub traces_sample_rate: f32
}