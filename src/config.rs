//! Keeps all the options, tweaks and dials of the configuration.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub tab_spaces: usize,
    pub max_width: usize,
    pub margin: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            tab_spaces: 4,
            max_width: 80,
            margin: 1,
        }
    }
}

impl Config {
    pub fn new(tab_spaces: usize, max_width: usize, margin: usize) -> Self {
        Config {
            tab_spaces,
            max_width,
            margin,
        }
    }
}
