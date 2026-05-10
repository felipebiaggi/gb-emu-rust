use super::MbcOps;

pub struct Mbc1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank: u8,            // bits 0-4 do registrador
    ram_bank_or_upper: u8,   // 2 bits: RAM bank ou upper bits do ROM (depende do mode)
    ram_enabled: bool,
    mode: u8,                // 0 = ROM banking, 1 = RAM banking
}

impl Mbc1 {
    pub fn new(rom: Vec<u8>, ram_size: usize) -> Self {
        Self {
            rom,
            ram: vec![0; ram_size],
            rom_bank: 1,
            ram_bank_or_upper: 0,
            ram_enabled: false,
            mode: 0,
        }
    }

    fn effective_rom_bank(&self) -> usize {
        let mut bank = self.rom_bank as usize;
        if self.mode == 0 {
            bank |= (self.ram_bank_or_upper as usize) << 5;
        }
        // Quirk: 0x00, 0x20, 0x40, 0x60 viram +1 (bank 0 nunca aparece em 0x4000-0x7FFF)
        if (bank & 0x1F) == 0 {
            bank |= 1;
        }
        bank
    }
}

impl MbcOps for Mbc1 {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                // Modo 0: bank 0 fixo
                // Modo 1 (ROMs > 512KB): upper bits aplicados
                let offset = if self.mode == 0 {
                    addr as usize
                } else {
                    ((self.ram_bank_or_upper as usize) << 5) * 0x4000 + addr as usize
                };
                self.rom[offset]
            }
            0x4000..=0x7FFF => {
                let bank = self.effective_rom_bank();
                let offset = bank * 0x4000 + (addr as usize - 0x4000);
                self.rom[offset]
            }
            0xA000..=0xBFFF => {
                if !self.ram_enabled || self.ram.is_empty() {
                    return 0xFF;
                }
                let bank = if self.mode == 1 {
                    self.ram_bank_or_upper as usize
                } else {
                    0
                };
                let offset = bank * 0x2000 + (addr as usize - 0xA000);
                self.ram[offset]
            }
            _ => 0xFF,
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => {
                // RAM enable: low nibble = 0xA habilita, qualquer outro desabilita
                self.ram_enabled = (data & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                // ROM bank low (5 bits). Bank 0 vira 1 imediatamente
                let bank = data & 0x1F;
                self.rom_bank = if bank == 0 { 1 } else { bank };
            }
            0x4000..=0x5FFF => {
                // RAM bank ou upper bits do ROM bank (2 bits)
                self.ram_bank_or_upper = data & 0x03;
            }
            0x6000..=0x7FFF => {
                // Banking mode select (1 bit)
                self.mode = data & 0x01;
            }
            0xA000..=0xBFFF => {
                if !self.ram_enabled || self.ram.is_empty() {
                    return;
                }
                let bank = if self.mode == 1 {
                    self.ram_bank_or_upper as usize
                } else {
                    0
                };
                let offset = bank * 0x2000 + (addr as usize - 0xA000);
                if let Some(slot) = self.ram.get_mut(offset) {
                    *slot = data;
                }
            }
            _ => {}
        }
    }
}
