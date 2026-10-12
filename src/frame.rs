use bytes::Bytes;

use crate::{
    error::EncodeError,
    header::{CURRENT_VERSION, Header, MAGIC, MessageType},
    size::PayloadSize,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub(crate) header: Header,
    pub(crate) payload: Bytes,
}

impl Frame {
    /// Builds a frame with a consistent header: `payload_size` is derived from `payload`,
    /// so an inconsistent frame cannot be created through the public API.
    ///
    /// Fails early if the payload cannot travel in a v0 frame (larger than `MAX_FRAME_SIZE`
    /// or not representable as `u32`).
    pub fn new(message_type: MessageType, flags: u16, payload: Bytes) -> Result<Self, EncodeError> {
        Ok(Self {
            header: Header {
                magic: MAGIC,
                version: CURRENT_VERSION,
                message_type,
                flags,
                payload_size: PayloadSize::from_usize(payload.len())?,
            },
            payload,
        })
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    /// Verifies the frame invariant: the header's declared size matches the payload.
    ///
    /// Crate-internal code can still assemble frames field by field (e.g. decoding),
    /// so `encode_frame` calls this as the last door before the wire.
    pub fn check(&self) -> Result<(), EncodeError> {
        let actual = self.payload.len();

        if self.header.payload_size.as_usize() != actual {
            return Err(EncodeError::PayloadSizeMismatch {
                declared: self.header.payload_size.get(),
                actual,
            });
        }

        Ok(())
    }

    pub fn new_ping() -> Self {
        Self {
            header: Header {
                magic: MAGIC,
                version: CURRENT_VERSION,
                message_type: MessageType::Heartbeat,
                flags: 0x00,
                payload_size: PayloadSize::ZERO,
            },
            payload: Bytes::new(),
        }
    }

    pub fn is_compressed(&self) -> bool {
        self.header.is_compressed()
    }

    pub fn is_multipart(&self) -> bool {
        self.header.is_multipart()
    }
}
