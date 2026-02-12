use crate::{bus::MemoryBus, ppu::framebuffer::FrameBuffer};

// Registros (endereços clássicos do GB)
const LCDC: u16 = 0xFF40;
const STAT: u16 = 0xFF41;
const SCY: u16 = 0xFF42;
const SCX: u16 = 0xFF43;
const LY: u16 = 0xFF44;
const LYC: u16 = 0xFF45;
const BGP: u16 = 0xFF47;

// Bits do LCDC
const LCDC_ENABLE: u8 = 1 << 7;
const LCDC_BG_ENABLE: u8 = 1 << 0;

// Modos da PPU (STAT bits 0-1)
const MODE_HBLANK: u8 = 0;
const MODE_VBLANK: u8 = 1;
const MODE_OAM: u8 = 2;
const MODE_XFER: u8 = 3;

// Timings por linha (em "dots"/t-cycles da PPU; no GB 1 M-cycle CPU = 4 dots)
const DOTS_PER_LINE: u16 = 456;
const OAM_DOTS: u16 = 80;
const XFER_DOTS: u16 = 172; // aproximado (varia no real), mas serve p/ base
const HBLANK_DOTS: u16 = DOTS_PER_LINE - OAM_DOTS - XFER_DOTS; // 204

pub struct Ppu {
    framebuffer: Box<FrameBuffer>,
    frame_ready: bool,
    mode: u8,
    dot: u16,
    rendered_this_line: bool,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            framebuffer: Box::new(FrameBuffer::new()),
            frame_ready: false,
            mode: MODE_OAM,
            dot: 0,
            rendered_this_line: false,
        }
    }

    pub fn tick(&mut self, t_cycles: u64, bus: &mut MemoryBus) {
        let lcdc = bus.read(LCDC);
        if (lcdc & LCDC_ENABLE) == 0 {
            self.mode = MODE_HBLANK;
            self.dot = 0;
            self.rendered_this_line = false;
            bus.write(LY, 0);
            self.set_stat_mode(bus, MODE_HBLANK);
            return;
        }

        let mut dots_to_advance = t_cycles as u16;

        while dots_to_advance > 0 {
            dots_to_advance -= 1;
            self.dot += 1;

            let ly = bus.read(LY);

            // VBlank lines
            if ly >= 144 {
                if self.mode != MODE_VBLANK {
                    self.mode = MODE_VBLANK;
                    self.set_stat_mode(bus, MODE_VBLANK);
                    self.frame_ready = true; // 1x por frame
                }
            } else {
                // Visible lines
                let new_mode = if self.dot < OAM_DOTS {
                    MODE_OAM
                } else if self.dot < (OAM_DOTS + XFER_DOTS) {
                    MODE_XFER
                } else {
                    MODE_HBLANK
                };

                if new_mode != self.mode {
                    self.mode = new_mode;
                    self.set_stat_mode(bus, new_mode);

                    if new_mode == MODE_XFER {
                        // garante render 1x por linha
                        self.rendered_this_line = false;
                    }
                }

                // Render 1 vez durante XFER
                if self.mode == MODE_XFER && !self.rendered_this_line {
                    self.render_scanline(bus, ly);
                    self.rendered_this_line = true;
                }
            }

            // End of line
            if self.dot >= DOTS_PER_LINE {
                self.dot = 0;
                self.rendered_this_line = false;

                let mut new_ly = ly.wrapping_add(1);
                if new_ly > 153 {
                    new_ly = 0;
                }
                bus.write(LY, new_ly);
                self.update_lyc(bus, new_ly);
            }
        }
    }

    fn update_lyc(&self, bus: &mut MemoryBus, ly: u8) {
        let lyc = bus.read(LYC);
        let mut stat = bus.read(STAT);

        if ly == lyc {
            stat |= 1 << 2; // coincidence flag
        } else {
            stat &= !(1 << 2);
        }
        bus.write(STAT, stat);
    }

    fn render_scanline(&mut self, bus: &mut MemoryBus, ly: u8) {
        // Render mínimo: só BG, sem janela/sprites, sem “timing real” de FIFO
        let lcdc = bus.read(LCDC);
        if (lcdc & LCDC_BG_ENABLE) == 0 {
            return;
        }

        let scx = bus.read(SCX);
        let scy = bus.read(SCY);
        let bgp = bus.read(BGP);

        // Escolhe base do BG map (LCDC bit 3)
        let bg_map_base: u16 = if (lcdc & (1 << 3)) != 0 {
            0x9C00
        } else {
            0x9800
        };

        // Tile data base (LCDC bit 4)
        // bit4=1 => 0x8000 unsigned index
        // bit4=0 => 0x8800 signed index
        let tile_data_unsigned = (lcdc & (1 << 4)) != 0;

        let y = ly as u16;
        let world_y = y.wrapping_add(scy as u16);
        let tile_row = (world_y / 8) & 31;
        let row_in_tile = (world_y % 8) as u16;

        for x in 0..160u16 {
            let world_x = x.wrapping_add(scx as u16);
            let tile_col = (world_x / 8) & 31;
            let col_in_tile = (world_x % 8) as u16;

            let tile_index_addr = bg_map_base + tile_row * 32 + tile_col;
            let tile_index = bus.read(tile_index_addr);

            let tile_addr: u16 = if tile_data_unsigned {
                0x8000 + (tile_index as u16) * 16
            } else {
                let signed = tile_index as i8 as i32;
                (0x9000i32 + signed * 16) as u16
            };

            // Cada linha do tile usa 2 bytes
            let lo = bus.read(tile_addr + row_in_tile * 2);
            let hi = bus.read(tile_addr + row_in_tile * 2 + 1);

            // bit do pixel (7..0)
            let bit = 7 - col_in_tile as u8;
            let b0 = (lo >> bit) & 1;
            let b1 = (hi >> bit) & 1;
            let color_id = (b1 << 1) | b0; // 0..3

            // Paleta BGP mapeia 0..3 -> shade 0..3
            let shade = (bgp >> (color_id * 2)) & 0b11;

            // Escreve no framebuffer
            let idx = ((y as usize) * 160 + (x as usize));
            self.framebuffer.pixels[idx] = shade as u8;
        }
    }

    pub fn take_frame(&mut self) -> Option<&[u8]> {
        if self.frame_ready {
            self.frame_ready = false;
            Some(&self.framebuffer.pixels)
        } else {
            None
        }
    }

    fn set_stat_mode(&self, bus: &mut MemoryBus, mode: u8) {
        let mut stat = bus.read(STAT);
        stat = (stat & !0b11) | (mode & 0b11);
        bus.write(STAT, stat);
    }
}
