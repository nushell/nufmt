use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum FormatError {
    #[error("found invalid Nushell syntax")]
    GarbageFound,
}
