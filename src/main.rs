use std::env;
use std::fs;
use std::u8;

mod cartridge;
mod cpu;
mod bus;

use crate::bus::memory_bus;
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::bus::MemoryBus;


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
    
    let bus = MemoryBus::new(cartridge);

    let mut cpu = Cpu::new(bus);

    // println!("{}", cpu.memory_bus.cartridge);

    cpu.start();
}
