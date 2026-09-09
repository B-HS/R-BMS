/// Everything that can go wrong while reading or writing a persisted store file.
///
/// The `Display` text is exactly the string the string-typed helpers used to produce, so the
/// messages the player prints stay unchanged across the move out of the app crate.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The file could not be read.
    #[error("{0}")]
    Read(#[source] std::io::Error),
    /// The file could not be written (including the atomic temp-and-rename dance).
    #[error("{0}")]
    Write(#[source] std::io::Error),
    /// The file was read but is not valid RON for the target type.
    #[error("{0}")]
    Parse(#[from] ron::error::SpannedError),
    /// The value could not be serialized to RON.
    #[error("{0}")]
    Serialize(#[from] ron::Error),
}
