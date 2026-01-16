use std::{time::{Duration, Instant}, u16, u8};
use bitflags::bitflags;

use crate::bus::MemoryBus;

const FPS: f64 = 60.7275;
const FRAME_TIME: Duration = Duration::from_nanos((1_000_000_000f64 / FPS) as u64);
const CYCLES_PER_FRAME: u64 = (4_194_304 as f64 / FPS) as u64;

bitflags! {
    pub struct Flags: u8 {
        const C = 1 << 4;
        const H = 1 << 5;
        const N = 1 << 6;
        const Z = 1 << 7;
    }
}


pub struct Cpu {
    pub register_a: u8,
    pub register_f: Flags,
    pub register_b: u8,
    pub register_c: u8,
    pub register_d: u8,
    pub register_e: u8,
    pub register_h: u8,
    pub register_l: u8,

    pub stack_pointer: u16,
    pub program_counter: u16,

    pub halt: bool,
    pub interruption: bool,

    pub opcode: u8,
    pub cycles: u8,

    pub memory_bus: MemoryBus
}



impl Cpu {
    pub fn new(bus: MemoryBus) -> Self {
        Self {
            register_a: 0,
            register_f: Flags::empty(),
            register_b: 0,
            register_c: 0,
            register_d: 0,
            register_e: 0,
            register_h: 0,
            register_l: 0,

            stack_pointer: 0,
            program_counter: 0,

            halt: false,
            interruption: false,

            opcode: 0,
            cycles: 0,
            memory_bus: bus
        }
    }
 
    // ADD magic values to ignore the gameboy bootrom
    pub fn init_registers(&mut self){

        self.register_a = 0x01;
        self.register_b = 0x00;
        self.register_c = 0x13;
        self.register_d = 0x00;
        self.register_e = 0xD8;
        self.register_h = 0x01;
        self.register_l = 0x4D;

        self.program_counter = 0x0100;
        self.stack_pointer = 0xFFFE;

        if(self.memory_bus.cartridge.header_checksum == 0x00){
            self.register_f = Flags::Z;
        } else {
            self.register_f = Flags::Z | Flags::H | Flags::C;
        }
    }

    pub fn start(&mut self) {
        self.init_registers();
        self.clock();
    }

    fn increment_program_counter(&mut self){
        self.program_counter+=1;
    }

    fn update_cycles(&mut self, cycles: u8){
        self.cycles = cycles;
    }

    fn clock(&mut self){
        let mut next = Instant::now();

        loop {
            self.cpu_step(300);

            next += FRAME_TIME;
            let now = Instant::now();
            
            if next > now {
                let remain = next - now;
                println!("Sleep: {}", remain.as_secs());
                std::thread::sleep(remain);
            }

            println!("End program");
            break;
        }
    }

    fn cpu_step(&mut self, steps: u64){
        for _ in 0..steps{
            let instruction = self.memory_bus.read(self.program_counter);
            println!("Instruction: 0x{:02X} - Address: 0x{:04X}", instruction, self.program_counter);
            self.process(instruction);
        }
    }

    fn process(&mut self, inst: u8){
        match inst {
            0x00 => self.NOP(),
            0x01 => self.LD_BC_u16(),
            0xC3 => self.INC_A(),
            0x10 => self.STOP(),
            0xFF => self.RST(),
            _ => todo!("Instrução ainda não implementada: 0x{:02X}", inst),
        }
    }

    fn NOP(&mut self) {
        println!("INSTRUCTION NOP");
        self.increment_program_counter();
        self.update_cycles(4);
    }

    fn LD_BC_u16(&mut self) {
        self.increment_program_counter();
        self.register_c = self.ld(self.program_counter);

        self.increment_program_counter();
        self.register_b = self.ld(self.program_counter);

        self.update_cycles(3);
    } 

    fn STOP(&mut self) {
        println!("INSTRUCTION STOP");
        self.increment_program_counter();
        self.increment_program_counter();
        self.update_cycles(4);
    }

    fn INC_A(&mut self) {
        self.increment_program_counter();
        
        self.register_f.remove(Flags::H);

        let a: u8 = self.register_a;
        let result = a.wrapping_add(1);

        if(result == 0x00) {
            self.register_f.insert(Flags::Z);
        } else {
            self.register_f.remove(Flags::Z);
        }

        if((result & 0x0F) == 0x00){
            self.register_f.insert(Flags::H);
        } else {
            self.register_f.remove(Flags::H);
        }

        self.update_cycles(4);

        self.register_a = a;
    }

    fn RST(&mut self) {
       self.increment_program_counter(); 
    }

    fn ld(&mut self, addr: u16) -> u8 {
        return self.memory_bus.read(addr);
    }


    fn push(&mut self, pc: u16) {
        let upper: u8 = (pc >> 8 & 0xffff) as u8;
        let lower: u8 = (pc & 0xffff) as u8;

        self.stack_pointer -= 1;
        self.memory_bus.write(self.stack_pointer, upper);
        

        self.stack_pointer -= 1;
        self.memory_bus.write(self.stack_pointer, lower);
    }

}
