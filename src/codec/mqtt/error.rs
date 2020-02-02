use std::{io, str};
use thiserror::Error;
use tokio::time::Duration;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("read/write timeout")]
    Timeout(Duration),
    #[error("invalid protocol")]
    InvalidProtocol,
    #[error("invalid length")]
    InvalidLength,
    #[error("unsupported protocol level")]
    UnsupportedProtocolLevel,
    #[error("connect reserved flag set")]
    ConnectReservedFlagSet,
    #[error("connack reserved flag set")]
    ConnAckReservedFlagSet,
    #[error("invalid client id")]
    InvalidClientId,
    #[error("unsupported packet type")]
    UnsupportedPacketType,
    #[error("packet id required")]
    PacketIdRequired,
    #[error("max size execeeded")]
    MaxSizeExceeded,
    #[error("IoError {0}")]
    IoError(#[from] io::Error),
    #[error("Utf8Error {0}")]
    Utf8Error(#[from] str::Utf8Error),
}

impl PartialEq for ParseError {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ParseError::InvalidProtocol => match other {
                ParseError::InvalidProtocol => true,
                _ => false,
            },
            ParseError::InvalidLength => match other {
                ParseError::InvalidLength => true,
                _ => false,
            },
            ParseError::UnsupportedProtocolLevel => match other {
                ParseError::UnsupportedProtocolLevel => true,
                _ => false,
            },
            ParseError::ConnectReservedFlagSet => match other {
                ParseError::ConnectReservedFlagSet => true,
                _ => false,
            },
            ParseError::ConnAckReservedFlagSet => match other {
                ParseError::ConnAckReservedFlagSet => true,
                _ => false,
            },
            ParseError::InvalidClientId => match other {
                ParseError::InvalidClientId => true,
                _ => false,
            },
            ParseError::UnsupportedPacketType => match other {
                ParseError::UnsupportedPacketType => true,
                _ => false,
            },
            ParseError::PacketIdRequired => match other {
                ParseError::PacketIdRequired => true,
                _ => false,
            },
            ParseError::MaxSizeExceeded => match other {
                ParseError::MaxSizeExceeded => true,
                _ => false,
            },
            ParseError::Timeout(_) => false,
            ParseError::IoError(_) => false,
            ParseError::Utf8Error(_) => false,
        }
    }
}

// impl From<io::Error> for ParseError {
//     fn from(err: io::Error) -> Self {
//         ParseError::IoError(err)
//     }
// }

// impl From<str::Utf8Error> for ParseError {
//     fn from(err: str::Utf8Error) -> Self {
//         ParseError::Utf8Error(err)
//     }
// }

#[derive(Error, Copy, Clone, Debug, PartialEq)]
pub enum TopicError {
    #[error("invalid topic")]
    InvalidTopic,
    #[error("invalid level")]
    InvalidLevel,
}
