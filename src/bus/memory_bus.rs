use crate::cartridge::Cartridge;

pub struct MemoryBus {
    pub cartridge: Cartridge,
    vram: [u8; 0x2000],
    eram: [u8; 0x2000],
    wram: [u8; 0x2000],
    oam: [u8; 0xA0],
    hram: [u8; 0x7F],
    io: [u8; 0x80],
    if_reg: u8,
    ie_reg: u8,
}

impl MemoryBus {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cartridge,
            vram: [0; 0x2000],
            eram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0xA0],
            hram: [0; 0x7F],
            io: [0; 0x80],
            if_reg: 0x00,
            ie_reg: 0x00,
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x7FFF => {
                println!("Write Cartridge addr: 0x{:04X}", addr);
                self.cartridge.write(addr, data);
            }

            0x8000..=0x9FFF => {
                println!("Write VRAM addr: 0x{:04X}", addr);
                self.vram[(addr - 0x8000) as usize] = data;
            }

            0xA000..=0xBFFF => {
                println!("Write ERAM addr: 0x{:04X}", addr);
                self.eram[(addr - 0xA000) as usize] = data;
            }

            0xC000..=0xDFFF => {
                println!("Write WRAM addr: 0x{:04X}", addr);
                self.wram[(addr - 0xC000) as usize] = data;
            }

            0xE000..=0xFDFF => {
                println!("Write ERAM addr: 0x{:04X}", addr);
                let echo = addr - 0xE000;
                self.wram[echo as usize] = data;
            }

            0xFE00..=0xFE9F => {
                println!("Write OAM addr: 0x{:04X}", addr);
                self.oam[(addr - 0xFE00) as usize] = data;
            }

            0xFEA0..=0xFEFF => {
            }

            0xFF00..=0xFF7F => {
                println!("Write I/O addr: 0x{:04X}", addr);
                if addr == 0xFF0F {
                    self.if_reg = data & 0x1F;
                } else {
                    self.io[(addr - 0xFF00) as usize] = data;
                }
            }

            0xFF80..=0xFFFE => {
                println!("Write HRAM addr: 0x{:04X}", addr);
                self.hram[(addr - 0xFF80) as usize] = data;
            }

            0xFFFF => {
                println!("Write IE addr: 0x{:04X}", addr);
                self.ie_reg = data & 0x1F;
            }
        }
    }

    pub fn request_interrupt(&mut self, flag_bit: u8) {
        self.if_reg |= flag_bit & 0x1F;
        println!(
            " request if |= 0x{:02X} -> if=0x{:02X}",
            flag_bit & 0x1F,
            self.if_reg
        );
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => {
                println!("Read Cartridge addr: 0x{:04X}", addr);
                self.cartridge.read(addr)
            }

            0x8000..=0x9FFF => {
                println!("Read VRAM addr: 0x{:04X}", addr);
                self.vram[(addr - 0x8000) as usize]
            }

            0xA000..=0xBFFF => {
                println!("Read ERAM addr: 0x{:04X}", addr);
                self.eram[(addr - 0xA000) as usize]
            }

            0xC000..=0xDFFF => {
                println!("Read WRAM addr: 0x{:04X}", addr);
                self.wram[(addr - 0xC000) as usize]
            }

            0xE000..=0xFDFF => {
                println!("Read ECHO RAM addr: 0x{:04X}", addr);
                self.wram[(addr - 0xE000) as usize]
            }

            0xFE00..=0xFE9F => {
                println!("Read OAM addr: 0x{:04X}", addr);
                self.oam[(addr - 0xFE00) as usize]
            }

            0xFEA0..=0xFEFF => {
                println!("Read not usable addr: 0x{:04X}", addr);
                0xFF
            }

            0xFF00..=0xFF7F => {
                println!("Read I/O registers addr: 0x{:04X}", addr);
                if addr == 0xFF0F {
                    self.if_reg
                } else {
                    self.io[(addr - 0xFF00) as usize]
                }
            }

            0xFF80..=0xFFFE => {
                println!("Read HRAM addr: 0x{:04X}", addr);
                self.hram[(addr - 0xFF80) as usize]
            }

            0xFFFF => {
                println!("Read IE addr: 0x{:04X}", addr);
                self.ie_reg
            }
        }
    }
}
