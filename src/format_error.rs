use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum FormatError {
    #[error("Found invalid Nushell syntax")]
    GarbageFound,
}
