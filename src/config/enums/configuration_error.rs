#[derive(Debug)]
pub enum ConfigurationError {
    IOError(std::io::Error),
    ParseError(toml::de::Error),
}