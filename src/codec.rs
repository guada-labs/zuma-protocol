use bytes::{Buf, BytesMut};

use crate::{
    MAX_FRAME_SIZE, MessageType,
    error::DecodeError,
    frame::Frame,
    header::{CURRENT_VERSION, HEADER_SIZE, Header, MAGIC},
};

impl Header {
    pub fn decode(src: &[u8]) -> Result<Header, DecodeError> {
        if src.len() < HEADER_SIZE {
            return Err(DecodeError::Incomplete);
        }

        if src[0..4] != MAGIC {
            return Err(DecodeError::MagicMismatch);
        }

        if src[4] != CURRENT_VERSION {
            return Err(DecodeError::UnsupportedVersion {
                expected: CURRENT_VERSION,
                actual: src[4],
            });
        }

        let message_type = MessageType::try_from(src[5])?;
        let flags = u16::from_be_bytes([src[6], src[7]]);
        let payload_size = u32::from_be_bytes([src[8], src[9], src[10], src[11]]);

        if payload_size > MAX_FRAME_SIZE {
            return Err(DecodeError::FrameTooLarge {
                size: payload_size,
                max: MAX_FRAME_SIZE,
            });
        }

        Ok(Header {
            magic: MAGIC,
            version: CURRENT_VERSION,
            message_type,
            flags,
            payload_size,
        })
    }

    pub fn encode(&self, dst: &mut [u8; 12]) {
        dst[0..4].copy_from_slice(&self.magic);
        dst[4] = self.version;
        dst[5] = self.message_type as u8;
        dst[6..8].copy_from_slice(&self.flags.to_be_bytes());
        dst[8..12].copy_from_slice(&self.payload_size.to_be_bytes());
    }
}

pub fn decode_frame(src: &mut BytesMut) -> Result<Option<Frame>, DecodeError> {
    if src.len() < HEADER_SIZE {
        return Ok(None);
    }

    let header = Header::decode(&src[..HEADER_SIZE])?;
    let total_len = HEADER_SIZE + header.payload_size as usize;

    if src.len() < total_len {
        return Ok(None);
    }

    src.advance(HEADER_SIZE);
    let payload = src.split_to(header.payload_size as usize).freeze(); // Slicing zero-copy

    Ok(Some(Frame { header, payload }))
}
