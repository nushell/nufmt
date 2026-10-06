//! Keeps all the options, tweaks and dials of the configuration.

use std::convert::TryFrom;

use crate::config_error::ConfigError;
use nu_protocol::Value;

/// Character used for indentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentChar {
    Space,
    Tab,
}

/// When an `if`/`else` or `try`/`catch` chain puts every branch on its own
/// lines (issue #217).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConsistentBranches {
    /// Lay out each branch on its own terms, so `{ foo }` can sit next to a
    /// multiline branch.
    Never,
    /// When a chain written on one line has a branch that must span several
    /// lines, expand every branch. Chains already written across lines keep
    /// their layout.
    #[default]
    SingleLine,
    /// Whenever any branch spans several lines, expand every branch.
    Always,
}

/// Configuration options for the formatter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Visual indentation width per level (default: 4).
    ///
    /// With `indent_char = "space"`, this is the number of spaces written.
    /// With `indent_char = "tab"`, this is the virtual tab width used in
    /// layout calculations while writing one tab per indentation level.
    pub indent: usize,
    /// Character used for each indentation unit (`space` or `tab`).
    pub indent_char: IndentChar,
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
    /// When the branches of an `if`/`else` or `try`/`catch` chain are all
    /// expanded together (default: `single_line`).
    pub consistent_branches: ConsistentBranches,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            indent: 4,
            indent_char: IndentChar::Space,
            line_length: 80,
            margin: 1,
            margin_is_explicit: false,
            excludes: Vec::new(),
            consistent_branches: ConsistentBranches::default(),
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
            indent_char: IndentChar::Space,
            line_length: max_width,
            margin,
            margin_is_explicit: true,
            excludes: Vec::new(),
            consistent_branches: ConsistentBranches::default(),
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
                "indent_char" => config.indent_char = parse_indent_char(value)?,
                "line_length" => config.line_length = parse_positive_int(key, value)?,
                "margin" => {
                    config.margin = parse_non_negative_int(key, value)?;
                    config.margin_is_explicit = true;
                }
                "exclude" => config.excludes = parse_string_list(value)?,
                "consistent_branches" => {
                    config.consistent_branches = parse_consistent_branches(value)?;
                }
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

/// Parse a value as the indentation character.
fn parse_indent_char(value: &Value) -> Result<IndentChar, ConfigError> {
    let Value::String { val, .. } = value else {
        return Err(ConfigError::InvalidOptionType(
            "indent_char".to_string(),
            value.get_type().to_string(),
            "string",
        ));
    };

    match val.as_str() {
        "space" => Ok(IndentChar::Space),
        "tab" => Ok(IndentChar::Tab),
        _ => Err(ConfigError::InvalidOptionValue(
            "indent_char".to_string(),
            val.clone(),
            "space or tab",
        )),
    }
}

/// Parse a value as the [`ConsistentBranches`] mode.
fn parse_consistent_branches(value: &Value) -> Result<ConsistentBranches, ConfigError> {
    let Value::String { val, .. } = value else {
        return Err(ConfigError::InvalidOptionType(
            "consistent_branches".to_string(),
            value.get_type().to_string(),
            "string",
        ));
    };

    match val.as_str() {
        "never" => Ok(ConsistentBranches::Never),
        "single_line" => Ok(ConsistentBranches::SingleLine),
        "always" => Ok(ConsistentBranches::Always),
        _ => Err(ConfigError::InvalidOptionValue(
            "consistent_branches".to_string(),
            val.clone(),
            "never, single_line or always",
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{Span, record};

    fn config_with(key: &str, value: Value) -> Result<Config, ConfigError> {
        Config::try_from(Value::test_record(record! { key => value }))
    }

    #[test]
    fn consistent_branches_defaults_to_single_line() {
        assert_eq!(
            Config::default().consistent_branches,
            ConsistentBranches::SingleLine
        );
    }

    #[test]
    fn consistent_branches_parses_every_mode() {
        for (name, mode) in [
            ("never", ConsistentBranches::Never),
            ("single_line", ConsistentBranches::SingleLine),
            ("always", ConsistentBranches::Always),
        ] {
            let config = config_with("consistent_branches", Value::test_string(name)).unwrap();
            assert_eq!(config.consistent_branches, mode);
        }
    }

    #[test]
    fn consistent_branches_rejects_unknown_values() {
        assert!(matches!(
            config_with("consistent_branches", Value::test_string("sometimes")),
            Err(ConfigError::InvalidOptionValue(..))
        ));
        assert!(matches!(
            config_with("consistent_branches", Value::int(1, Span::test_data())),
            Err(ConfigError::InvalidOptionType(..))
        ));
    }
}
