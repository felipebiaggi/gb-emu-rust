use bitflags::{bitflags, parser};
use raylib::core::texture::RaylibTexture2D;
use raylib::prelude::*;
use std::time::{Duration, Instant};

use crate::bus::MemoryBus;
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::ppu::Ppu;

pub struct Emulator {
    pub cpu: Cpu,
    pub bus: MemoryBus,
    pub ppu: Ppu,
}

const GB_W: i32 = 160;
const GB_H: i32 = 144;
const CYCLES_PER_FRAME: u64 = 70_224;

bitflags! {
    pub struct InterruptFlags: u8 {
        const VBLANK  = 1 << 0;
        const LCDSTAT = 1 << 1;
        const TIMER   = 1 << 2;
        const SERIAL  = 1 << 3;
        const JOYPAD  = 1 << 4;
    }
}

impl Emulator {
    pub fn new(cartridge: Cartridge) -> Self {
        let bus = MemoryBus::new(cartridge);

        Self {
            cpu: Cpu::new(),
            ppu: Ppu::new(),
            bus: bus,
        }
    }

    pub fn start(&mut self) {
        self.cpu.reset();
        self.bus.reset();
        self.run();
    }

    fn run(&mut self) {
        let window_title = self.bus.cartridge.game_title.clone();

        let (mut rl, thread) = raylib::init()
            .size(640, 480)
            .title(&window_title.split('\0').next().unwrap_or("GB"))
            .build();

        let mut rgba: Vec<u8> = vec![0; (GB_W as usize) * (GB_H as usize) * 4];

        let image = Image::gen_image_color(GB_W, GB_H, Color::BLACK);
        let mut texture: Texture2D = rl.load_texture_from_image(&thread, &image).unwrap();

        while !rl.window_should_close() {
            if let Some(frame) = self.run_frame() {
                for (index, &color) in frame.iter().enumerate() {
                    let pixel = index * 4;

                    let value = match (color & 0b11) {
                        0 => 255,
                        1 => 170,
                        2 => 85,
                        _ => 0,
                    };

                    rgba[pixel + 0] = value;
                    rgba[pixel + 1] = value;
                    rgba[pixel + 2] = value;
                    rgba[pixel + 3] = 255;
                }
                texture.update_texture(&rgba).unwrap();
            }

            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::BLACK);

            let scale = 3.0;
            let draw_w = GB_W as f32 * scale;
            let draw_h = GB_H as f32 * scale;
            let x = (640.0 - draw_w) * 0.5;
            let y = (480.0 - draw_h) * 0.5;

            d.draw_texture_ex(&texture, Vector2::new(x, y), 0.0, scale, Color::WHITE);
            d.draw_fps(10, 10);
        }
    }

    fn run_frame(&mut self) -> Option<&[u8]> {
        let mut cycles_this_frame: u64 = 0;

        while cycles_this_frame < CYCLES_PER_FRAME {
            let cycles = self.cpu.step(&mut self.bus) as u64;
            self.ppu.tick(cycles, &mut self.bus);

            cycles_this_frame += cycles as u64;
        }

        self.ppu.take_frame()
    }
}
