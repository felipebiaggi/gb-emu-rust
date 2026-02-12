use std::env;
use std::fs;
use std::u8;

mod bus;
mod cartridge;
mod cpu;
mod machine;
mod ppu;

use crate::cartridge::Cartridge;
use crate::machine::Emulator;

fn main() {
    let args: Vec<String> = env::args().collect();

    let rom: Vec<u8> = match fs::read(&args[1]) {
        Ok(vec_u8) => vec_u8,
        Err(erro) => {
            eprintln!("Error ao ler o arquivo '{}': {}", &args[1], erro);
            return;
        }
    };

    let cartridge = Cartridge::load(rom);
    let mut emulator = Emulator::new(cartridge);

    emulator.start();
}
