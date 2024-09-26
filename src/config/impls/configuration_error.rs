use crate::config::enums::configuration_error::ConfigurationError;

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConfigurationError::IOError(e) => e.fmt(f),
            ConfigurationError::ParseError(e) => e.fmt(f)
        }
    }
}

impl std::error::Error for ConfigurationError {}