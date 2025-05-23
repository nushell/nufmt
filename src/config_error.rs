use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("Failed read the configuration file: {0}")]
    IOError(String),
    #[error("The configuration is not a valid nuon record")]
    InvalidFormat,
    #[error("Found unknown configuration option: {0}")]
    UnknownOption(String),
    #[error("Found invalid type for option '{0}': got {1}, expected {2}")]
    InvalidOptionType(String, String, &'static str),
    #[error("Found invalid value for option '{0}': got {1}, expected {2}")]
    InvalidOptionValue(String, String, &'static str),
    #[error("Found an invalid exclude pattern")]
    InvalidExcludePattern,
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value.to_string())
    }
}

impl From<nu_protocol::ShellError> for ConfigError {
    fn from(_value: nu_protocol::ShellError) -> Self {
        Self::InvalidFormat
    }
}

impl From<ignore::Error> for ConfigError {
    fn from(_value: ignore::Error) -> Self {
        Self::InvalidExcludePattern
    }
}
