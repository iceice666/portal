#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Timeout occurred.")]
    Timeout,
    #[error("An IO error occurred: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("An Bincode error occurred: {source}")]
    Bincode {
        #[from]
        source: bincode::Error,
    },

    #[error("The data is too large.")]
    DataTooLarge,
}
