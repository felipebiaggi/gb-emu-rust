use super::MbcOps;

pub struct NoMbc {
    rom: Vec<u8>,
}

impl NoMbc {
    pub fn new(rom: Vec<u8>) -> Self {
        Self { rom }
    }
}

impl MbcOps for NoMbc {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.rom[addr as usize],
            _ => 0xFF, // sem RAM externa
        }
    }

    fn write(&mut self, _addr: u16, _data: u8) {
        // ROM read-only: writes silenciosamente ignorados
    }
}
