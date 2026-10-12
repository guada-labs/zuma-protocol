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

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;
    use crate::header::{FLAG_COMPRESSED, FLAG_MULTIPART, MAX_FRAME_SIZE};

    #[test]
    fn ping_frame_roundtrip() {
        let frame = Frame::new_ping();

        let mut buf = BytesMut::new();
        encode_frame(&frame, &mut buf).expect("ping must encode");

        assert_eq!(buf.len(), HEADER_SIZE);

        let decoded = decode_frame(&mut buf).expect("ping must decode");

        assert_eq!(decoded, Some(frame));
        assert!(buf.is_empty());
    }

    #[test]
    fn frame_with_payload_and_flags_roundtrip() {
        let frame = Frame::new(
            MessageType::SyncDoc,
            FLAG_COMPRESSED | FLAG_MULTIPART,
            Bytes::from_static(b"zuma"),
        )
        .expect("frame must build");

        let mut buf = BytesMut::new();
        encode_frame(&frame, &mut buf).expect("frame must encode");

        assert_eq!(buf.len(), HEADER_SIZE + 4);

        let decoded = decode_frame(&mut buf)
            .expect("frame must decode")
            .expect("complete frame");

        assert_eq!(decoded, frame);
        assert!(decoded.is_compressed());
        assert!(decoded.is_multipart());
        assert!(buf.is_empty());
    }

    #[test]
    fn header_bytes_use_big_endian_field_layout() {
        let frame = Frame::new(
            MessageType::SyncDoc,
            FLAG_COMPRESSED | FLAG_MULTIPART,
            Bytes::from_static(b"zuma"),
        )
        .expect("frame must build");

        let mut buf = BytesMut::new();
        encode_frame(&frame, &mut buf).expect("frame must encode");

        // magic "ZUMA" | version | type 0x10 | flags 0x0003 | payload_size 4,
        // all numeric fields in big-endian order.
        assert_eq!(
            &buf[..HEADER_SIZE],
            &[
                0x5A, 0x55, 0x4D, 0x41, 0x00, 0x10, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04
            ]
        );
        assert_eq!(&buf[HEADER_SIZE..], b"zuma");
    }

    #[test]
    fn partial_header_returns_none_and_keeps_buffer() {
        let frame = Frame::new_ping();
        let mut buf = BytesMut::new();
        encode_frame(&frame, &mut buf).expect("ping must encode");

        // A TCP chunk may carry only part of a frame: "not yet", not an error.
        buf.truncate(7);

        assert_eq!(
            decode_frame(&mut buf).expect("partial header must not be an error"),
            None
        );
        assert_eq!(buf.len(), 7);
    }

    #[test]
    fn partial_payload_returns_none_and_keeps_all_bytes() {
        let frame = Frame::new(
            MessageType::ContextSignal,
            0x0000,
            Bytes::from_static(b"hello"),
        )
        .expect("frame must build");

        let mut buf = BytesMut::new();
        encode_frame(&frame, &mut buf).expect("frame must encode");

        // Only 3 of the 5 payload bytes arrived.
        buf.truncate(HEADER_SIZE + 3);

        assert_eq!(
            decode_frame(&mut buf).expect("partial payload must not be an error"),
            None
        );

        // Nothing is consumed while the frame is incomplete, header included.
        assert_eq!(buf.len(), HEADER_SIZE + 3);

        // Once the missing bytes arrive, the same buffer decodes cleanly.
        buf.extend_from_slice(&frame.payload()[3..]);

        let decoded = decode_frame(&mut buf)
            .expect("frame must decode")
            .expect("complete frame");

        assert_eq!(decoded, frame);
        assert!(buf.is_empty());
    }

    #[test]
    fn bad_magic_returns_magic_mismatch() {
        let frame = Frame::new_ping();
        let mut buf = BytesMut::new();
        encode_frame(&frame, &mut buf).expect("ping must encode");

        // A complete header arrives, but the first magic byte is not 'Z'.
        buf[0] = b'X';

        assert!(matches!(
            decode_frame(&mut buf),
            Err(DecodeError::MagicMismatch)
        ));

        // Errors do not partially consume the buffer.
        assert_eq!(buf.len(), HEADER_SIZE);
    }

    #[test]
    fn unsupported_version_returns_expected_and_actual() {
        let frame = Frame::new_ping();
        let mut buf = BytesMut::new();
        encode_frame(&frame, &mut buf).expect("ping must encode");

        buf[4] = 0x01;

        assert!(matches!(
            decode_frame(&mut buf),
            Err(DecodeError::UnsupportedVersion {
                expected: 0x00,
                actual: 0x01
            })
        ));
    }

    #[test]
    fn invalid_message_type_is_rejected() {
        // 0x00 is reserved, 0x42 is not assigned in v0.
        for invalid_type in [0x00_u8, 0x42] {
            let frame = Frame::new_ping();
            let mut buf = BytesMut::new();
            encode_frame(&frame, &mut buf).expect("ping must encode");

            buf[5] = invalid_type;

            assert!(matches!(
                decode_frame(&mut buf),
                Err(DecodeError::InvalidMessageType(value)) if value == invalid_type
            ));
        }
    }

    #[test]
    fn oversized_declared_payload_is_rejected_immediately() {
        let oversized = MAX_FRAME_SIZE + 1;

        // Hand-built header: a peer can declare an impossible size, even though
        // encode_frame would never produce such a frame.
        let mut buf = BytesMut::from(&MAGIC[..]);
        buf.extend_from_slice(&[CURRENT_VERSION, MessageType::Heartbeat as u8]);
        buf.extend_from_slice(&0u16.to_be_bytes());
        buf.extend_from_slice(&oversized.to_be_bytes());
        assert_eq!(buf.len(), HEADER_SIZE);

        // Only the header arrived: the parser rejects the declared size without
        // waiting for (or allocating) any payload.
        assert!(matches!(
            decode_frame(&mut buf),
            Err(DecodeError::FrameTooLarge { size, max })
                if size == oversized && max == MAX_FRAME_SIZE
        ));
    }

    #[test]
    fn encode_frame_appends_to_existing_bytes() {
        let frame = Frame::new_ping();

        // Callers may batch frames or keep earlier bytes in the same buffer.
        let mut buf = BytesMut::from(&b"PREFIX"[..]);
        encode_frame(&frame, &mut buf).expect("ping must encode");

        assert_eq!(buf.len(), 6 + HEADER_SIZE);
        assert_eq!(&buf[..6], b"PREFIX");

        // The prefix is not a frame: split it off before decoding.
        let mut frames = buf.split_off(6);

        let decoded = decode_frame(&mut frames).expect("frame must decode");
        assert_eq!(decoded, Some(frame));
        assert!(frames.is_empty());
    }

    #[test]
    fn decode_frame_consumes_one_frame_at_a_time() {
        let first = Frame::new_ping();
        let second = Frame::new(MessageType::SyncDoc, 0x0000, Bytes::from_static(b"state"))
            .expect("frame must build");

        let mut buf = BytesMut::new();
        encode_frame(&first, &mut buf).expect("first must encode");
        encode_frame(&second, &mut buf).expect("second must encode");

        // First call: only the first frame is consumed.
        let decoded_first = decode_frame(&mut buf)
            .expect("first must decode")
            .expect("complete first frame");
        assert_eq!(decoded_first, first);

        // Second call: the remaining frame decodes and the buffer empties.
        let decoded_second = decode_frame(&mut buf)
            .expect("second must decode")
            .expect("complete second frame");
        assert_eq!(decoded_second, second);
        assert!(buf.is_empty());
    }

    #[test]
    fn incomplete_second_frame_leaves_first_decode_unaffected() {
        let first = Frame::new_ping();
        let second = Frame::new(MessageType::SyncDoc, 0x0000, Bytes::from_static(b"state"))
            .expect("frame must build");

        let mut buf = BytesMut::new();
        encode_frame(&first, &mut buf).expect("first must encode");
        encode_frame(&second, &mut buf).expect("second must encode");

        // Drop the last byte: the second frame is now incomplete.
        let full_len = buf.len();
        buf.truncate(full_len - 1);

        let decoded_first = decode_frame(&mut buf)
            .expect("first must decode")
            .expect("complete first frame");
        assert_eq!(decoded_first, first);

        // What is left waits for its missing byte.
        assert_eq!(
            decode_frame(&mut buf).expect("partial second must not be an error"),
            None
        );
        assert_eq!(buf.len(), HEADER_SIZE + 4);
    }

    #[test]
    fn inconsistent_frame_is_rejected_and_dst_stays_intact() {
        // Crate-internal code is the only place that can build an inconsistent
        // frame: public constructors always derive payload_size from the payload.
        let mut frame = Frame::new(MessageType::SyncDoc, 0x0000, Bytes::from_static(b"zuma"))
            .expect("frame must build");
        frame.header.payload_size = PayloadSize::ZERO;

        let mut buf = BytesMut::from(&b"KEEP"[..]);

        assert!(matches!(
            encode_frame(&frame, &mut buf),
            Err(EncodeError::PayloadSizeMismatch {
                declared: 0,
                actual: 4
            })
        ));

        // check() runs before any write, so dst is untouched.
        assert_eq!(&buf[..], b"KEEP");
    }
}
