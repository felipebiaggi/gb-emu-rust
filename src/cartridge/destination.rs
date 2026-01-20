use std::fmt;

pub enum Destination {
    Japan,
    Overseas,
    Unknown(u8),
}

impl From<u8> for Destination {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Destination::Japan,
            0x01 => Destination::Overseas,
            other => Destination::Unknown(other),
        }
    }
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Destination::Japan => write!(f, "Japan (0x00)"),
            Destination::Overseas => write!(f, "Overseas (0x01)"),
            Destination::Unknown(x) => write!(f, "Unknown (0x{:02X})", x),
        }
    }
}
