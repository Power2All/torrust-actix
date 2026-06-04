use crate::config::config::{
    default_attach_stacktrace,
    default_max_breadcrumbs,
    default_sample_rate,
    default_traces_sample_rate
};
use crate::config::structs::sentry_config::SentryConfig;

impl Default for SentryConfig {
    fn default() -> Self {
        SentryConfig {
            enabled: false,
            dsn: String::new(),
            debug: false,
            sample_rate: default_sample_rate(),
            max_breadcrumbs: default_max_breadcrumbs(),
            attach_stacktrace: default_attach_stacktrace(),
            send_default_pii: false,
            traces_sample_rate: default_traces_sample_rate(),
        }
    }
}