use bytes::Bytes;

use crate::header::{CURRENT_VERSION, Header, MAGIC, MessageType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub header: Header,
    pub payload: Bytes,
}

impl Frame {
    pub fn new_ping() -> Self {
        Self {
            header: Header {
                magic: MAGIC,
                version: CURRENT_VERSION,
                message_type: MessageType::Heartbeat,
                flags: 0x00,
                payload_size: 0x00,
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
