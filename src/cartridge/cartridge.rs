use std::{fmt, u8, u16};

use super::cartridge_type::CartridgeType;
use super::destination::Destination;

pub struct Cartridge {
    pub game_data: Vec<u8>,
    pub game_title: String,
    pub manufacturer_code: String,
    pub cgb_flag: u8,
    pub licensee_code: String,
    pub sgb_flag: u8,
    pub cartridge_type: CartridgeType,
    pub rom_size: u8,
    pub ram_size: u8,
    pub destination_code: Destination,
    pub old_licensee_code: u8,
    pub mask_rom_version_number: u8,
    pub header_checksum: u8,
    pub global_checksum: u16,
}

impl Cartridge {
    pub fn read(&self, addr: u16) -> u8 {
        return self.game_data[addr as usize];
    }

    pub fn load(value: Vec<u8>) -> Self {
        let game_title = String::from_utf8_lossy(&value[308..324]).to_string();

        let manufacturer_code = String::from_utf8_lossy(&value[319..323]).to_string();

        let cgb_flag = value[323];

        let licensee_code = format!("{}{}", value[324] as char, value[325] as char);

        let sgb_flag = value[326];

        let cartridge_type = CartridgeType::from(value[327]);

        let rom_size = value[328];

        let ram_size = value[329];

        let destination_code = Destination::from(value[330]);

        let old_licensee_code = value[331];

        let mask_rom_version_number = value[332];

        let header_checksum = value[333];

        let global_checksum = u16::from_be_bytes([value[334], value[335]]);

        Self {
            game_data: value,
            game_title,
            manufacturer_code,
            cgb_flag,
            licensee_code,
            sgb_flag,
            cartridge_type,
            rom_size,
            ram_size,
            destination_code,
            old_licensee_code,
            mask_rom_version_number,
            header_checksum,
            global_checksum,
        }
    }
}

impl fmt::Display for Cartridge {
    fn fmt(&self, format: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(format, "=== Game Boy Cartridge Info ===")?;
        writeln!(format, "Title:               {}", self.game_title)?;
        writeln!(format, "Manufacturer Code:   {}", self.manufacturer_code)?;
        writeln!(format, "Licensee Code:       {}", self.licensee_code)?;
        writeln!(format, "CGB Flag:            {:#04X}", self.cgb_flag)?;
        writeln!(format, "SGB Flag:            {:#04X}", self.sgb_flag)?;
        writeln!(format, "Cartridge Type:      {}", self.cartridge_type)?;
        writeln!(format, "ROM Size:            {:#04X}", self.rom_size)?;
        writeln!(format, "RAM Size:            {:#04X}", self.ram_size)?;
        writeln!(format, "Destination Code:    {}", self.destination_code)?;
        writeln!(
            format,
            "Old Licensee Code:   {:#04X}",
            self.old_licensee_code
        )?;
        writeln!(
            format,
            "Mask ROM Version:    {:#04X}",
            self.mask_rom_version_number
        )?;
        writeln!(format, "Header Checksum:     {:#04X}", self.header_checksum)?;
        writeln!(format, "Global Checksum:     {:#06X}", self.global_checksum)?;
        writeln!(
            format,
            "Game Data Size:      {} bytes",
            self.game_data.len()
        )?;
        Ok(())
    }
}
