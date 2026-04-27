//! Keeps all the options, tweaks and dials of the configuration.

use std::convert::TryFrom;

use crate::config_error::ConfigError;
use nu_protocol::Value;

/// Configuration options for the formatter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Number of spaces per indentation level (default: 4).
    pub indent: usize,
    /// Maximum line length before wrapping (default: 80).
    pub line_length: usize,
    /// Number of blank lines to insert between top-level definitions (default: 1).
    pub margin: usize,
    /// Whether `margin` was set explicitly in the config file.
    ///
    /// When `false`, the formatter uses heuristics (e.g. preserving the
    /// blank-line structure already present in the source) instead of
    /// enforcing a fixed count.
    pub margin_is_explicit: bool,
    /// Glob patterns for files to exclude from formatting.
    pub excludes: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            indent: 4,
            line_length: 80,
            margin: 1,
            margin_is_explicit: false,
            excludes: Vec::new(),
        }
    }
}

impl Config {
    /// Create a `Config` with explicitly specified values.
    ///
    /// All three arguments are mandatory; `excludes` defaults to empty and
    /// `margin_is_explicit` is set to `true`.
    pub fn new(tab_spaces: usize, max_width: usize, margin: usize) -> Self {
        Self {
            indent: tab_spaces,
            line_length: max_width,
            margin,
            margin_is_explicit: true,
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
                "margin" => {
                    config.margin = parse_non_negative_int(key, value)?;
                    config.margin_is_explicit = true;
                }
                "exclude" => config.excludes = parse_string_list(value)?,
                unknown => return Err(ConfigError::UnknownOption(unknown.to_string())),
            }
        }

        Ok(config)
    }
}

/// Parse a value as an integer that must be `>= min_value`.
///
/// `expected_desc` is the human-readable constraint shown in error messages
/// (e.g. `"a positive number"`, `"a non-negative number"`).
fn parse_int_at_least(
    key: &str,
    value: &Value,
    min_value: i64,
    expected_desc: &'static str,
) -> Result<usize, ConfigError> {
    let Value::Int { val, .. } = value else {
        return Err(ConfigError::InvalidOptionType(
            key.to_string(),
            value.get_type().to_string(),
            "number",
        ));
    };

    if *val < min_value {
        return Err(ConfigError::InvalidOptionValue(
            key.to_string(),
            val.to_string(),
            expected_desc,
        ));
    }

    Ok(*val as usize)
}

/// Parse a value as a positive integer (must be `>= 1`).
fn parse_positive_int(key: &str, value: &Value) -> Result<usize, ConfigError> {
    parse_int_at_least(key, value, 1, "a positive number")
}

/// Parse a value as a non-negative integer (must be `>= 0`).
fn parse_non_negative_int(key: &str, value: &Value) -> Result<usize, ConfigError> {
    parse_int_at_least(key, value, 0, "a non-negative number")
}

/// Parse a `Value` as a `list<string>` and return the strings.
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
