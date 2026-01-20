use std::u16;

use crate::cartridge::Cartridge;

pub struct MemoryBus {
    pub cartridge: Cartridge,
}

impl MemoryBus {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cartridge: cartridge,
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {}

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            // Cartridge 32KiB
            0x000..=0x7FFF => {
                println!("Read Cartridge addr: 0x{:02X}", addr);
                return self.cartridge.read(addr);
            }

            // Video RAM (VRAM) 8KiB
            0x8000..=0x9FFF => {
                println!("Read VRAM addr: 0x{:02X}", addr);
                return 0x00;
            }

            // External RAM 8KiB
            0xA000..=0xBFFF => {
                println!("Read External RAM addr: 0x{:02X}", addr);
                return 0x00;
            }

            // Work RAM (WRAM)
            0xC000..=0xCFFF => {
                println!("Read Work RAM addr: 0x{:02X}", addr);
                return 0x00;
            }

            // Work RAM (WRAM) GBC mode
            0xD000..=0xDFFF => {
                println!("Read Work RAM (GBC mode) addr: 0x{:02X}", addr);
                return 0x00;
            }

            // Echo RAM (mirror of C000-DDFF)
            0xE000..=0xFDFF => {
                println!("Read Echo RAM addr: 0x{:02X}", addr);
                return 0x00;
            }

            // Object Attribute memory
            0xFE00..=0xFE9F => {
                println!("Read OAM addr: 0x{:02X}", addr);
                return 0x00;
            }

            // Not usable
            0xFEA0..=0xFEFF => {
                println!("Read Not usable addr: 0x{:02X}", addr);
                return 0x00;
            }

            // I/O Registers
            0xFF00..=0xFF7F => {
                println!("Read I/O registers addr: 0x{:02X}", addr);
                return 0x00;
            }

            // High RAM (HRAM)
            0xFF80..=0xFFFE => {
                println!("Read HRAM addr: 0x{:02X}", addr);
                return 0x00;
            }

            // Interrupt Enable Register
            0xFFFF => {
                println!("Read Interrupt addr: 0x{:02X}", addr);
                return 0x00;
            }
        }
    }
}
