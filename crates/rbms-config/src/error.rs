/// Everything that can go wrong while reading, migrating or writing the player configuration.
///
/// The `Display` text of the I/O and RON variants is exactly the string the string-typed helpers in
/// the app used to produce, so the messages the player prints stay unchanged across the move out of
/// the app crate.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file could not be read.
    #[error("{0}")]
    Read(#[source] std::io::Error),
    /// The file could not be written (including the atomic temp-and-rename dance).
    #[error("{0}")]
    Write(#[source] std::io::Error),
    /// The file was read but is not valid RON for the schema it declares.
    #[error("{0}")]
    Parse(#[from] ron::error::SpannedError),
    /// The value could not be serialized to RON.
    #[error("{0}")]
    Serialize(#[from] ron::Error),
    /// The file declares a schema this build cannot read. The caller must leave the file alone
    /// rather than overwrite a configuration written by a newer build.
    #[error("schema version {from} cannot be migrated: {reason}")]
    Migrate { from: u32, reason: String },
}
