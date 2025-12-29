//! Keeps all the options, tweaks and dials of the configuration.

use std::convert::TryFrom;

use crate::config_error::ConfigError;
use nu_protocol::Value;

/// Configuration options for the formatter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub indent: usize,
    pub line_length: usize,
    pub margin: usize,
    pub excludes: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            indent: 4,
            line_length: 80,
            margin: 1,
            excludes: Vec::new(),
        }
    }
}

impl Config {
    pub fn new(tab_spaces: usize, max_width: usize, margin: usize) -> Self {
        Self {
            indent: tab_spaces,
            line_length: max_width,
            margin,
            excludes: Vec::new(),
        }
    }
}

impl TryFrom<Value> for Config {
    type Error = ConfigError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let mut config = Config::default();

        let Value::Record { val: record, .. } = value else {
            // Nothing means use defaults
            if matches!(value, Value::Nothing { .. }) {
                return Ok(config);
            }
            return Err(ConfigError::InvalidFormat);
        };

        for (key, value) in record.iter() {
            match key.as_str() {
                "indent" => config.indent = parse_positive_int(key, value)?,
                "line_length" => config.line_length = parse_positive_int(key, value)?,
                "margin" => config.margin = parse_positive_int(key, value)?,
                "exclude" => config.excludes = parse_string_list(value)?,
                unknown => return Err(ConfigError::UnknownOption(unknown.to_string())),
            }
        }

        Ok(config)
    }
}

/// Parse a value as a positive integer (usize)
fn parse_positive_int(key: &str, value: &Value) -> Result<usize, ConfigError> {
    let Value::Int { val, .. } = value else {
        return Err(ConfigError::InvalidOptionType(
            key.to_string(),
            value.get_type().to_string(),
            "number",
        ));
    };

    if *val <= 0 {
        return Err(ConfigError::InvalidOptionValue(
            key.to_string(),
            val.to_string(),
            "a positive number",
        ));
    }

    Ok(*val as usize)
}

/// Parse a value as a list of strings
fn parse_string_list(value: &Value) -> Result<Vec<String>, ConfigError> {
    let Value::List { vals, .. } = value else {
        return Err(ConfigError::InvalidOptionType(
            "excludes".to_string(),
            value.get_type().to_string(),
            "list<string>",
        ));
    };

    vals.iter()
        .map(|val| {
            let Value::String { val, .. } = val else {
                return Err(ConfigError::InvalidOptionType(
                    "excludes".to_string(),
                    val.get_type().to_string(),
                    "list<string>",
                ));
            };
            Ok(val.clone())
        })
        .collect()
}
