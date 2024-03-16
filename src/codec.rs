type AnyResult<T = ()> = anyhow::Result<T>;

use anyhow::anyhow;
use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use std::io;
use tokio_util::codec::{self, Encoder};

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
    Data,
}

impl From<RequestKind> for u8 {
    fn from(kind: RequestKind) -> u8 {
        match kind {
            RequestKind::Ping => 0,
            RequestKind::Data => 1,
        }
    }
}

impl TryFrom<u8> for RequestKind {
    type Error = anyhow::Error;

    fn try_from(byte: u8) -> AnyResult<Self> {
        Ok(match byte {
            0 => RequestKind::Ping,
            1 => RequestKind::Data,
            _ => Err(anyhow!("Invalid request kind"))?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum ResponseKind {
    Pong,
    Data,
    Ok,
}

impl From<ResponseKind> for u8 {
    fn from(kind: ResponseKind) -> u8 {
        match kind {
            ResponseKind::Pong => 0,
            ResponseKind::Data => 1,
            ResponseKind::Ok => 2,
        }
    }
}

impl TryFrom<u8> for ResponseKind {
    type Error = anyhow::Error;

    fn try_from(byte: u8) -> AnyResult<Self> {
        Ok(match byte {
            0 => ResponseKind::Pong,
            1 => ResponseKind::Data,
            2 => ResponseKind::Ok,
            _ => Err(anyhow!("Invalid response kind"))?,
        })
    }
}
