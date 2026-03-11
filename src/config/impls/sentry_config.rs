use crate::config::structs::sentry_config::SentryConfig;

pub(crate) fn default_sample_rate() -> f32 { 1.0 }
pub(crate) fn default_traces_sample_rate() -> f32 { 1.0 }
pub(crate) fn default_max_breadcrumbs() -> usize { 100 }
pub(crate) fn default_attach_stacktrace() -> bool { true }

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