pub(crate) type CrateResult<T = ()> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Timeout occurred.")]
    Timeout,
    #[error("An IO error occurred: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("A Bincode error occurred: {source}")]
    Bincode {
        #[from]
        source: bincode::Error,
    },

    #[error("The data is too large.")]
    DataTooLarge,

    #[error("Cannot calculate the SHA256 hash.")]
    Sha256Digest,

    #[error("Given path is not a file.")]
    NotAFile,
}
