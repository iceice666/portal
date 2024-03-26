use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Invalid input. Please try again.")]
    InvalidInput,
    #[error("See you next time!")]
    Exit,
    #[error("Not implemented yet.")]
    NotImplemented,
    #[error("An error occurred: {0}")]
    Custom(String),
}

impl Error {
    pub fn handle(&self) {
        println!("{}", self);
    }
}
