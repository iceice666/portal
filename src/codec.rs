type AnyResult<T = ()> = anyhow::Result<T>;

use std::io;

use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use tokio_util::codec;

#[derive(Debug)]
pub enum ErrorKind {
    Io(io::Error),
    Disconnected,
}

impl From<io::Error> for ErrorKind {
    fn from(err: io::Error) -> Self {
        ErrorKind::Io(err)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum RequestKind {
    Ping,
    Data(Vec<u8>),
}
