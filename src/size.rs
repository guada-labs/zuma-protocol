use crate::{
    error::{DecodeError, EncodeError},
    header::MAX_FRAME_SIZE,
};

/// A payload length that is guaranteed to fit in `u32` and to stay within
/// `MAX_FRAME_SIZE`. Once parsed, no caller needs to re-check the range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadSize(u32);

impl PayloadSize {
    /// Zero-length payload (e.g. heartbeat frames).
    pub const ZERO: Self = Self(0);

    /// The largest payload a v0 frame may carry.
    pub const MAX: u32 = MAX_FRAME_SIZE;

    /// Parses a length coming from memory (a `len()` call).
    ///
    /// Fails with the encode-side error because the caller is building a frame.
    pub fn from_usize(len: usize) -> Result<Self, EncodeError> {
        let size = u32::try_from(len).map_err(|_| EncodeError::FrameTooLarge {
            size: len,
            max: MAX_FRAME_SIZE,
        })?;

        if size > MAX_FRAME_SIZE {
            return Err(EncodeError::FrameTooLarge {
                size: len,
                max: MAX_FRAME_SIZE,
            });
        }

        Ok(Self(size))
    }

    /// Parses a length read off the wire (already a `u32`).
    ///
    /// Fails with the decode-side error because the value came from a peer.
    pub fn from_wire(size: u32) -> Result<Self, DecodeError> {
        if size > MAX_FRAME_SIZE {
            return Err(DecodeError::FrameTooLarge {
                size,
                max: MAX_FRAME_SIZE,
            });
        }

        Ok(Self(size))
    }

    pub fn get(self) -> u32 {
        self.0
    }

    /// Widening conversion: safe on 32-bit and 64-bit targets.
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

