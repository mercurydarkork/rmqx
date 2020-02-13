use std::{io, str};
use thiserror::Error;
use tokio::time::Duration;

#[derive(Error, Debug)]
pub enum MqttError {
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
    #[error("authenticate fail")]
    AuthError,
    #[error("service unavailable")]
    ServiceUnavailable,
    #[error("IoError {0}")]
    IoError(#[from] io::Error),
    #[error("Utf8Error {0}")]
    Utf8Error(#[from] str::Utf8Error),
}

impl PartialEq for MqttError {
    fn eq(&self, other: &Self) -> bool {
        match self {
            MqttError::InvalidProtocol => match other {
                MqttError::InvalidProtocol => true,
                _ => false,
            },
            MqttError::InvalidLength => match other {
                MqttError::InvalidLength => true,
                _ => false,
            },
            MqttError::UnsupportedProtocolLevel => match other {
                MqttError::UnsupportedProtocolLevel => true,
                _ => false,
            },
            MqttError::ConnectReservedFlagSet => match other {
                MqttError::ConnectReservedFlagSet => true,
                _ => false,
            },
            MqttError::ConnAckReservedFlagSet => match other {
                MqttError::ConnAckReservedFlagSet => true,
                _ => false,
            },
            MqttError::InvalidClientId => match other {
                MqttError::InvalidClientId => true,
                _ => false,
            },
            MqttError::UnsupportedPacketType => match other {
                MqttError::UnsupportedPacketType => true,
                _ => false,
            },
            MqttError::PacketIdRequired => match other {
                MqttError::PacketIdRequired => true,
                _ => false,
            },
            MqttError::MaxSizeExceeded => match other {
                MqttError::MaxSizeExceeded => true,
                _ => false,
            },
            MqttError::AuthError => match other {
                MqttError::AuthError => true,
                _ => false,
            },
            MqttError::ServiceUnavailable => match other {
                MqttError::ServiceUnavailable => true,
                _ => false,
            },
            MqttError::Timeout(_) => false,
            MqttError::IoError(_) => false,
            MqttError::Utf8Error(_) => false,
        }
    }
}

#[derive(Error, Copy, Clone, Debug, PartialEq)]
pub enum TopicError {
    #[error("invalid topic")]
    InvalidTopic,
    #[error("invalid level")]
    InvalidLevel,
}
