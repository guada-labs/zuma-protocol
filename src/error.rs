use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("magic bytes mismatch: expected 'ZUMA'")]
    MagicMismatch,
    #[error("unsupported protocol version: expected {expected:#04x}, got {actual:#04x}")]
    UnsupportedVersion { expected: u8, actual: u8 },
    #[error("invalid or reserved message type: {0:#04x}")]
    InvalidMessageType(u8),
    #[error("frame payload too large: {size} bytes exceeds maximum limit of {max} bytes")]
    FrameTooLarge { size: u32, max: u32 },
    #[error("incomplete buffer: requires more bytes to parse")]
    Incomplete,
    #[error("unknown decode error")]
    Unknown,
}

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error("frame payload too large: {size} bytes exceeds maximum limit of {max} bytes")]
    FrameTooLarge { size: usize, max: u32 },
    #[error("header declares {declared} payload bytes but the payload holds {actual}")]
    PayloadSizeMismatch { declared: u32, actual: usize },
    #[error("unknown encode error")]
    Unknown,
}
