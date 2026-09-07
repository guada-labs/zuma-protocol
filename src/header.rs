use crate::error::DecodeError;

pub const MAGIC: [u8; 4] = *b"ZUMA";
pub const HEADER_SIZE: usize = 12;
pub const CURRENT_VERSION: u8 = 0x00;
pub const MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;
pub const FLAG_COMPRESSED: u16 = 0x0001; // Bit 0: 0 = crudo, 1 = comprimido con Zstd
pub const FLAG_MULTIPART: u16 = 0x0002; // Bit 1: 0 = mensaje completo, 1 = fragmento (chunk)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    HandshakeInit = 0x01,
    HandshakeResp = 0x02,
    SyncDoc = 0x10,
    ContextSignal = 0x20,
    ActionCmd = 0x30,
    Heartbeat = 0xFF,
}

impl TryFrom<u8> for MessageType {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(MessageType::HandshakeInit),
            0x02 => Ok(MessageType::HandshakeResp),
            0x10 => Ok(MessageType::SyncDoc),
            0x20 => Ok(MessageType::ContextSignal),
            0x30 => Ok(MessageType::ActionCmd),
            0xFF => Ok(MessageType::Heartbeat),
            other => Err(DecodeError::InvalidMessageType(other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub magic: [u8; 4],
    pub version: u8,
    pub message_type: MessageType,
    pub flags: u16,
    pub payload_size: u32,
}

impl Header {
    pub fn is_compressed(&self) -> bool {
        (self.flags & FLAG_COMPRESSED) != 0
    }

    pub fn is_multipart(&self) -> bool {
        (self.flags & FLAG_MULTIPART) != 0
    }
}
