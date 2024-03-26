use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid input. Please try again.")]
    InvalidInput,
    #[error("See you next time!")]
    Exit,
    #[error("Not implemented yet.")]
    NotImplemented,
    #[error("An error occurred: {0}")]
    Custom(String),
    #[error("Timeout occurred.")]
    Timeout,
    #[error("An IO error occurred: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
}

impl Error {
    pub fn handle(&self) {
        println!("{}", self);
    }
}
