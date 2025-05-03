//! Keeps all the options, tweaks and dials of the configuration.

use std::convert::TryFrom;

use crate::config_error::ConfigError;
use nu_protocol::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub indent: usize,
    pub line_length: usize,
    pub margin: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            indent: 4,
            line_length: 80,
            margin: 1,
        }
    }
}

impl Config {
    pub fn new(tab_spaces: usize, max_width: usize, margin: usize) -> Self {
        Config {
            indent: tab_spaces,
            line_length: max_width,
            margin,
        }
    }
}

impl TryFrom<Value> for Config {
    type Error = ConfigError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let mut config = Config::default();
        match value {
            Value::String { val, .. } => {
                if !val.is_empty() {
                    return Err(ConfigError::InvalidFormat);
                };
            }
            Value::Record { val: record, .. } => {
                for (key, value) in record.iter() {
                    match key.as_str() {
                        "indent" => {
                            let indent = parse_value_to_usize(key, value)?;
                            config.indent = indent;
                        }
                        "line_length" => {
                            let line_length = parse_value_to_usize(key, value)?;
                            config.line_length = line_length;
                        }
                        "margin" => {
                            let margin = parse_value_to_usize(key, value)?;
                            config.margin = margin;
                        }
                        unknown => return Err(ConfigError::UnknownOption(unknown.to_string())),
                    }
                }
            }
            _ => {
                return Err(ConfigError::InvalidFormat);
            }
        }
        Ok(config)
    }
}

fn parse_value_to_usize(key: &str, value: &Value) -> Result<usize, ConfigError> {
    match value {
        Value::Int { val, .. } => {
            if *val <= 0 {
                return Err(ConfigError::InvalidOptionValue(
                    key.to_string(),
                    format!("{}", val),
                    "a positive number",
                ));
            }
            Ok(*val as usize)
        }
        other => Err(ConfigError::InvalidOptionType(
            key.to_string(),
            other.get_type().to_string(),
            "number",
        )),
    }
}
