use std::{u8, u16};
use bitflags::bitflags;

use crate::bus::MemoryBus;

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

// impl Default for Cpu {
//     fn default() -> Self {
//         Self {
//             register_a: 0,
//             register_f: Flags::empty(),
//             register_b: 0,
//             register_c: 0,
//             register_d: 0,
//             register_e: 0,
//             register_h: 0,
//             register_l: 0,
//
//             stack_pointer: 0,
//             program_counter: 0,
//
//             halt: false,
//             interruption: false,
//
//             opcode: 0,
//             cycles: 0
//         }
//     }
// }

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
}
