use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum FormatError {
    #[error("Found invalid Nushell syntax")]
    GarbageFound,
}
