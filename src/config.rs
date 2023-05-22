/// options available to the formatter
pub struct Config {
    /// number of spaces of indent
    pub tab_spaces: usize,
    /// max amount of characters per line
    /// # Maximum width of each line.
    /// Default: 100
    pub max_width: usize,
    /// number of lines bafore and after a custom command
    pub margin: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            tab_spaces: 2,
            max_width: 100,
            margin: 1,
        }
    }
}
