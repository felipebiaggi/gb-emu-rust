use std::time::{Duration, Instant};

use bitflags::bitflags;

use crate::bus::MemoryBus;

const CPU_HZ: u64 = 4_194_304;
const CYCLES_PER_FRAME: u64 = 70_224;
const FPS: f64 = CPU_HZ as f64 / CYCLES_PER_FRAME as f64;
const FRAME_TIME: Duration = Duration::from_nanos((1_000_000_000f64 / FPS) as u64);

bitflags! {
    pub struct FFlags: u8 {
        const C = 1 << 4;
        const H = 1 << 5;
        const N = 1 << 6;
        const Z = 1 << 7;
    }
}

pub struct Cpu {
    // 8-bit regs
    pub register_a: u8,
    pub register_f: FFlags,
    pub register_b: u8,
    pub register_c: u8,
    pub register_d: u8,
    pub register_e: u8,
    pub register_h: u8,
    pub register_l: u8,

    // 16-bit regs
    pub stack_pointer: u16,
    pub program_counter: u16,

    // state
    pub halt: bool,
    pub stop: bool,
    pub interruption: bool,

    pub opcode: u8,
    pub cycles: u8,

    // memory
    pub memory_bus: MemoryBus,
}

impl Cpu {
    pub fn new(bus: MemoryBus) -> Self {
        Self {
            register_a: 0,
            register_f: FFlags::empty(),
            register_b: 0,
            register_c: 0,
            register_d: 0,
            register_e: 0,
            register_h: 0,
            register_l: 0,

            stack_pointer: 0,
            program_counter: 0,

            halt: false,
            stop: false,
            interruption: false,

            opcode: 0,
            cycles: 0,
            memory_bus: bus,
        }
    }

    // Valores mágicos pós-bootrom (pra começar direto em 0x0100).
    pub fn init_registers(&mut self) {
        self.register_a = 0x01;
        self.register_b = 0x00;
        self.register_c = 0x13;
        self.register_d = 0x00;
        self.register_e = 0xD8;
        self.register_h = 0x01;
        self.register_l = 0x4D;

        self.program_counter = 0x0100;
        self.stack_pointer = 0xFFFE;

        if self.memory_bus.cartridge.header_checksum == 0x00 {
            self.register_f = FFlags::Z;
        } else {
            self.register_f = FFlags::Z | FFlags::H | FFlags::C;
        }
    }

    pub fn start(&mut self) {
        self.init_registers();
        self.clock();
    }

    fn clock(&mut self) {
        let mut next_frame_deadline = Instant::now();

        loop {
            self.run_frame();

            next_frame_deadline += FRAME_TIME;
            let now = Instant::now();

            if next_frame_deadline > now {
                let remain = next_frame_deadline - now;
                std::thread::sleep(remain);
            } else {
                next_frame_deadline = now;
            }

            //remover futuramente
            break;
        }
    }

    fn run_frame(&mut self) {
        let mut cycles_this_frame: u64 = 0;

        while cycles_this_frame < CYCLES_PER_FRAME {
            let cycles = self.step();
            cycles_this_frame += cycles as u64;

        }
    }

     fn step(&mut self) -> u8 {
    
        //stop e halt temporarios
        if self.stop {
            return 0;
        }

        if self.halt {
            return 4;
        }

        let inst = self.memory_bus.read(self.program_counter);
        self.opcode = inst;

        self.process(inst);

        self.cycles
    }

    fn process(&mut self, inst: u8) {
        match inst {
            0x00 => self.nop(),
            0x01 => self.ld_bc_u16(),
            0x02 => self.ld_bc_a(),
            0x03 => self.inc_bc(),
            0x05 => self.dec_b(),
            0x06 => self.ld_b_u8(),
            0x07 => self.rlca(),
            0x08 => self.ld_u16_sp(),
            0x10 => self.stop_inst(),
            0x3C => self.inc_a(),
            0xFF => self.rst_38(),
            _ => todo!("instrução ainda não implementada: 0x{:02X}", inst),
        }
    }

    fn nop(&mut self) {
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_bc_u16(&mut self) {
        let low = self.read_u8_pc(1);
        let high = self.read_u8_pc(2);

        self.register_c = low;
        self.register_b = high;

        self.advance_program_counter(3);
        self.update_cycles(12);
    }

    fn ld_bc_a(&mut self) {
        let addr: u16 = ((self.register_b as u16) << 8) | (self.register_c as u16);
        self.memory_bus.write(addr, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_bc(&mut self) {
        let bc: u16 = ((self.register_b as u16) << 8) | (self.register_c as u16);
        let inc_bc = bc.wrapping_add(1);

        self.register_b = (inc_bc >> 8) as u8;
        self.register_c = (inc_bc & 0x00FF) as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_b(&mut self) {
        self.register_b = self.inc(self.register_b);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn dec_b(&mut self) {
        self.register_b = self.dec(self.register_b);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_b_u8(&mut self) {
        let imm = self.read_u8_pc(1);
        self.register_b = imm;

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rlca(&mut self) {
        let bit7 = (self.register_a & 0b1000_0000) != 0;
        let new_register = self.register_a.rotate_left(1);

        self.register_f.remove(FFlags::Z);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);

        if bit7 {
            self.register_f.insert(FFlags::C);
        } else {
            self.register_f.remove(FFlags::C);
        }

        self.register_a = new_register;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_u16_sp(&mut self) {
        let low = self.read_u8_pc(1);
        let high = self.read_u8_pc(2);
        let addr = (low as u16) | ((high as u16) << 8);

        let sp_low = (self.stack_pointer & 0x00FF) as u8;
        let sp_high = (self.stack_pointer >> 8) as u8;

        self.memory_bus.write(addr, sp_low);
        self.memory_bus.write(addr.wrapping_add(1), sp_high);

        self.advance_program_counter(3);
        self.update_cycles(20);
    }

    fn stop_inst(&mut self) {
        let next = self.read_u8_pc(1);
        if next != 0x00 {
            panic!(
                "stop (0x10) inválido: esperado 0x00 após o opcode, mas veio 0x{:02X} em pc=0x{:04X}",
                next, self.program_counter
            );
        }

        self.stop = true;

        self.advance_program_counter(2);
        self.update_cycles(4);
    }

    fn inc_a(&mut self) {
        self.register_a = self.inc(self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn rst_38(&mut self) {
        let ret = self.program_counter.wrapping_add(1);
        self.push(ret);

        self.program_counter = 0x0038;
        self.update_cycles(16);
    }

    fn update_cycles(&mut self, cycles: u8) {
        self.cycles = cycles;
    }

    fn advance_program_counter(&mut self, n: u16) {
        self.program_counter = self.program_counter.wrapping_add(n);
    }

    fn read_u8_pc(&mut self, offset: u16) -> u8 {
        self.memory_bus.read(self.program_counter.wrapping_add(offset))
    }

    fn inc(&mut self, register: u8) -> u8 {
        self.register_f.remove(FFlags::N);

        let old_register: u8 = register;
        let result = old_register.wrapping_add(1);

        self.register_f.set(FFlags::Z, result == 0x00);
        self.register_f.set(FFlags::H, (old_register & 0x0F) == 0x0F);

        result
    }

    fn dec(&mut self, register: u8) -> u8 {
        self.register_f.insert(FFlags::N);

        let old_register: u8 = register;
        let result = old_register.wrapping_sub(1);

        self.register_f.set(FFlags::Z, result == 0x00);
        self.register_f.set(FFlags::H, (old_register & 0x0F) == 0x00);

        result
    }

    fn rlc(&mut self, register: u8) -> u8 {
        let bit7 = (register & 0b1000_0000) != 0;
        let new_register = register.rotate_left(1);

        self.register_f.set(FFlags::Z, new_register == 0x00);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.set(FFlags::C, bit7);

        new_register
    }

    fn push(&mut self, pc: u16) {
        let upper: u8 = ((pc >> 8) & 0xFF) as u8;
        let lower: u8 = (pc & 0xFF) as u8;

        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory_bus.write(self.stack_pointer, upper);

        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory_bus.write(self.stack_pointer, lower);
    }
}
