use std::u8;
use std::fmt;

pub enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
    Mbc1RamBattery,
    Mbc2,
    Mbc2Battery,
    RomRam,
    RomRamBattery,
    Mmm01,
    Mmm01Ram,
    Mmm01RamBattery,
    Mbc3TimerBattery,
    Mbc3TimerRamBattery,
    Mbc3,
    Mbc3Ram,
    Mbc3RamBattery,
    Mbc5,
    Mbc5Ram,
    Mbc5RamBattery,
    Mbc5Rumble,
    Mbc5RumbleRam,
    Mbc5RumbleRamBattery,
    Mbc6,
    Mbc7SensorRumbleRamBattery,
    PocketCamera,
    BandaiTama5,
    Huc3,
    Huc1RamBattery,
}

impl From<u8> for CartridgeType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => CartridgeType::RomOnly,
            0x01 => CartridgeType::Mbc1,
            0x02 => CartridgeType::Mbc1Ram,
            0x03 => CartridgeType::Mbc1RamBattery,
            0x05 => CartridgeType::Mbc2,
            0x06 => CartridgeType::Mbc2Battery,
            0x08 => CartridgeType::RomRam,
            0x09 => CartridgeType::RomRamBattery,
            0x0B => CartridgeType::Mmm01,
            0x0C => CartridgeType::Mmm01Ram,
            0x0D => CartridgeType::Mmm01RamBattery,
            0x0F => CartridgeType::Mbc3TimerBattery,
            0x10 => CartridgeType::Mbc3TimerRamBattery,
            0x11 => CartridgeType::Mbc3,
            0x12 => CartridgeType::Mbc3Ram,
            0x13 => CartridgeType::Mbc3RamBattery,
            0x19 => CartridgeType::Mbc5,
            0x1A => CartridgeType::Mbc5Ram,
            0x1B => CartridgeType::Mbc5RamBattery,
            0x1C => CartridgeType::Mbc5Rumble,
            0x1D => CartridgeType::Mbc5RumbleRam,
            0x1E => CartridgeType::Mbc5RumbleRamBattery,
            0x20 => CartridgeType::Mbc6,
            0x22 => CartridgeType::Mbc7SensorRumbleRamBattery,
            0xFC => CartridgeType::PocketCamera,
            0xFD => CartridgeType::BandaiTama5,
            0xFE => CartridgeType::Huc3,
            0xFF => CartridgeType::Huc1RamBattery,
            other => panic!("Cartridge Type invalid: 0x{:02X}", other),
        }
    }
}

impl fmt::Display for CartridgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CartridgeType::RomOnly => write!(f, "ROM Only"),
            CartridgeType::Mbc1 => write!(f, "MBC1"),
            CartridgeType::Mbc1Ram => write!(f, "MBC1 + RAM"),
            CartridgeType::Mbc1RamBattery => write!(f, "MBC1 + RAM + Battery"),
            CartridgeType::Mbc2 => write!(f, "MBC2"),
            CartridgeType::Mbc2Battery => write!(f, "MBC2 + Battery"),
            CartridgeType::RomRam => write!(f, "ROM + RAM"),
            CartridgeType::RomRamBattery => write!(f, "ROM + RAM + Battery"),
            CartridgeType::Mmm01 => write!(f, "MMM01"),
            CartridgeType::Mmm01Ram => write!(f, "MMM01 + RAM"),
            CartridgeType::Mmm01RamBattery => write!(f, "MMM01 + RAM + Battery"),
            CartridgeType::Mbc3TimerBattery => write!(f, "MBC3 + Timer + Battery"),
            CartridgeType::Mbc3TimerRamBattery => write!(f, "MBC3 + Timer + RAM + Battery"),
            CartridgeType::Mbc3 => write!(f, "MBC3"),
            CartridgeType::Mbc3Ram => write!(f, "MBC3 + RAM"),
            CartridgeType::Mbc3RamBattery => write!(f, "MBC3 + RAM + Battery"),
            CartridgeType::Mbc5 => write!(f, "MBC5"),
            CartridgeType::Mbc5Ram => write!(f, "MBC5 + RAM"),
            CartridgeType::Mbc5RamBattery => write!(f, "MBC5 + RAM + Battery"),
            CartridgeType::Mbc5Rumble => write!(f, "MBC5 + Rumble"),
            CartridgeType::Mbc5RumbleRam => write!(f, "MBC5 + Rumble + RAM"),
            CartridgeType::Mbc5RumbleRamBattery => write!(f, "MBC5 + Rumble + RAM + Battery"),
            CartridgeType::Mbc6 => write!(f, "MBC6"),
            CartridgeType::Mbc7SensorRumbleRamBattery => write!(f, "MBC7 + Sensor + Rumble + RAM + Battery"),
            CartridgeType::PocketCamera => write!(f, "Pocket Camera"),
            CartridgeType::BandaiTama5 => write!(f, "Bandai TAMA5"),
            CartridgeType::Huc3 => write!(f, "HuC3"),
            CartridgeType::Huc1RamBattery => write!(f, "HuC1 + RAM + Battery"),
        }
    }
}


