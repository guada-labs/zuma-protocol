use bytes::{Buf, BytesMut};

use crate::{
    MessageType,
    error::{DecodeError, EncodeError},
    frame::Frame,
    header::{CURRENT_VERSION, HEADER_SIZE, Header, MAGIC},
    size::PayloadSize,
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
        let payload_size =
            PayloadSize::from_wire(u32::from_be_bytes([src[8], src[9], src[10], src[11]]))?;

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
        dst[8..12].copy_from_slice(&self.payload_size.get().to_be_bytes());
    }
}

pub fn decode_frame(src: &mut BytesMut) -> Result<Option<Frame>, DecodeError> {
    if src.len() < HEADER_SIZE {
        return Ok(None);
    }

    let header = Header::decode(&src[..HEADER_SIZE])?;
    let total_len = HEADER_SIZE + header.payload_size.as_usize();

    if src.len() < total_len {
        return Ok(None);
    }

    src.advance(HEADER_SIZE);
    let payload = src.split_to(header.payload_size.as_usize()).freeze(); // Slicing zero-copy

    Ok(Some(Frame { header, payload }))
}

/// Appends `frame` (12-byte header + payload) to `dst`.
///
/// Fails if `frame` is internally inconsistent (declared size differs from the
/// payload length). A payload larger than `MAX_FRAME_SIZE` can never be declared,
/// so it always trips that same check.
pub fn encode_frame(frame: &Frame, dst: &mut BytesMut) -> Result<(), EncodeError> {
    frame.check()?;

    let mut header_bytes = [0u8; HEADER_SIZE];
    frame.header.encode(&mut header_bytes);
    dst.extend_from_slice(&header_bytes);
    dst.extend_from_slice(&frame.payload);

    Ok(())
}

