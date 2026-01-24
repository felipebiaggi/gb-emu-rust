use std::time::{Duration, Instant};

use bitflags::{Flags, bitflags};

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
            0x04 => self.inc_b(),
            0x05 => self.dec_b(),
            0x06 => self.ld_b_u8(),
            0x07 => self.rlca(),
            0x08 => self.ld_u16_sp(),
            0x09 => self.add_hl_bc(),
            0x0A => self.ld_a_bc(),
            0x0B => self.dec_bc(),
            0x0C => self.inc_c(),
            0x0D => self.dec_c(),
            0x0E => self.ld_c_u8(),
            0x0F => self.rrca(),

            0x10 => self.stop_inst(),
            0x11 => self.ld_de_u16(),
            0x12 => self.ld_de_a(),
            0x13 => self.inc_de(),
            0x14 => self.inc_d(),
            0x15 => self.dec_d(),
            0x16 => self.ld_d_u8(),
            0x17 => self.rla(),
            0x18 => self.jr_i8(),
            0x19 => self.add_hl_de(),
            0x1A => self.ld_a_de(),
            0x1B => self.dec_de(),
            0x1C => self.inc_e(),
            0x1D => self.dec_e(),
            0x1E => self.ld_e_u8(),
            0x1F => self.rra(),

            0x20 => self.jr_nz_i8(),
            0x21 => self.ld_hl_u16(),
            0x22 => self.ldi_hl_a(),
            0x23 => self.inc_hl(),
            0x24 => self.inc_h(),
            0x25 => self.dec_h(),
            0x26 => self.ld_h_u8(),
            0x27 => self.daa(),
            0x28 => self.jr_z_i8(),
            0x29 => self.add_hl_hl(),
            0x2A => self.ldi_a_hl(),
            0x2B => self.dec_hl(),
            0x2C => self.inc_l(),
            0x2D => self.dec_l(),
            0x2E => self.ld_l_u8(),
            0x2F => self.cpl(),

            0x30 => self.jr_nc_i8(),
            0x31 => self.ld_sp_u16(),
            0x32 => self.ldd_hl_a(),
            0x33 => self.inc_sp(),
            0x34 => self.inc_hl_ptr(),
            0x35 => self.dec_hl_ptr(),
            0x36 => self.ld_hl_ptr_u8(),
            0x37 => self.scf(),
            0x38 => self.jr_c_i8(),
            0x39 => self.add_hl_sp(),
            0x3A => self.ldd_a_hl(),
            0x3B => self.dec_sp(),
            0x3C => self.inc_a(),
            0x3D => self.dec_a(),
            0x3E => self.ld_a_u8(),
            0x3F => self.ccf(),

            0x40 => self.ld_b_b(),
            0x41 => self.ld_b_c(),
            0x42 => self.ld_b_d(),
            0x43 => self.ld_b_e(),
            0x44 => self.ld_b_h(),
            0x45 => self.ld_b_l(),
            0x46 => self.ld_b_hl_ptr(),
            0x47 => self.ld_b_a(),
            0x48 => self.ld_c_b(),
            0x49 => self.ld_c_c(),
            0x4A => self.ld_c_d(),
            0x4B => self.ld_c_e(),
            0x4C => self.ld_c_h(),
            0x4D => self.ld_c_l(),
            0x4E => self.ld_c_hl_ptr(),
            0x4F => self.ld_c_a(),

            0x50 => self.ld_d_b(),
            0x51 => self.ld_d_c(),
            0x52 => self.ld_d_d(),
            0x53 => self.ld_d_e(),
            0x54 => self.ld_d_h(),
            0x55 => self.ld_d_l(),
            0x56 => self.ld_d_hl_ptr(),
            0x57 => self.ld_d_a(),
            0x58 => self.ld_e_b(),
            0x59 => self.ld_e_c(),
            0x5A => self.ld_e_d(),
            0x5B => self.ld_e_e(),
            0x5C => self.ld_e_h(),
            0x5D => self.ld_e_l(),
            0x5E => self.ld_e_hl_ptr(),
            0x5F => self.ld_e_a(),

            0x60 => self.ld_h_b(),
            0x61 => self.ld_h_c(),
            0x62 => self.ld_h_d(),
            0x63 => self.ld_h_e(),
            0x64 => self.ld_h_h(),
            0x65 => self.ld_h_l(),
            0x66 => self.ld_h_hl_ptr(),
            0x67 => self.ld_h_a(),
            0x68 => self.ld_l_b(),
            0x69 => self.ld_l_c(),
            0x6A => self.ld_l_d(),
            0x6B => self.ld_l_e(),
            0x6C => self.ld_l_h(),
            0x6D => self.ld_l_l(),
            0x6E => self.ld_l_hl_ptr(),
            0x6F => self.ld_l_a(),

            0x70 => self.ld_hl_ptr_b(),
            0x71 => self.ld_hl_ptr_c(),
            0x72 => self.ld_hl_ptr_d(),
            0x73 => self.ld_hl_ptr_e(),
            0x74 => self.ld_hl_ptr_h(),
            0x75 => self.ld_hl_ptr_l(),
            0x76 => self.halt_inst(),
            0x77 => self.ld_hl_ptr_a(),
            0x78 => self.ld_a_b(),
            0x79 => self.ld_a_c(),
            0x7A => self.ld_a_d(),
            0x7B => self.ld_a_e(),
            0x7C => self.ld_a_h(),
            0x7D => self.ld_a_l(),
            0x7E => self.ld_a_hl_ptr(),
            0x7F => self.ld_a_a(),

            0x80 => self.add_a_b(),
            0x81 => self.add_a_c(),
            0x82 => self.add_a_d(),
            0x83 => self.add_a_e(),
            0x84 => self.add_a_h(),
            0x85 => self.add_a_l(),
            0x86 => self.add_a_hl_ptr(),
            0x87 => self.add_a_a(),
            0x88 => self.adc_a_b(),
            0x89 => self.adc_a_c(),
            0x8A => self.adc_a_d(),
            0x8B => self.adc_a_e(),
            0x8C => self.adc_a_h(),
            0x8D => self.adc_a_l(),
            0x8E => self.adc_a_hl_ptr(),
            0x8F => self.adc_a_a(),

            0x90 => self.sub_b(),
            0x91 => self.sub_c(),
            0x92 => self.sub_d(),
            0x93 => self.sub_e(),
            0x94 => self.sub_h(),
            0x95 => self.sub_l(),
            0x96 => self.sub_hl_ptr(),
            0x97 => self.sub_a(),
            0x98 => self.sbc_a_b(),
            0x99 => self.sbc_a_c(),
            0x9A => self.sbc_a_d(),
            0x9B => self.sbc_a_e(),
            0x9C => self.sbc_a_h(),
            0x9D => self.sbc_a_l(),
            0x9E => self.sbc_a_hl_ptr(),
            0x9F => self.sbc_a_a(),

            0xA0 => self.and_b(),
            0xA1 => self.and_c(),
            0xA2 => self.and_d(),
            0xA3 => self.and_e(),
            0xA4 => self.and_h(),
            0xA5 => self.and_l(),
            0xA6 => self.and_hl_ptr(),
            0xA7 => self.and_a(),
            0xA8 => self.xor_b(),
            0xA9 => self.xor_c(),
            0xAA => self.xor_d(),
            0xAB => self.xor_e(),
            0xAC => self.xor_h(),
            0xAD => self.xor_l(),
            0xAE => self.xor_hl_ptr(),
            0xAF => self.xor_a(),

            0xB0 => self.or_b(),
            0xB1 => self.or_c(),
            0xB2 => self.or_d(),
            0xB3 => self.or_e(),
            0xB4 => self.or_h(),
            0xB5 => self.or_l(),
            0xB6 => self.or_hl_ptr(),
            0xB7 => self.or_a(),
            0xB8 => self.cp_b(),
            0xB9 => self.cp_c(),
            0xBA => self.cp_d(),
            0xBB => self.cp_e(),
            0xBC => self.cp_h(),
            0xBD => self.cp_l(),
            0xBE => self.cp_hl_ptr(),
            0xBF => self.cp_a(),

            0xC0 => self.ret_nz(),
            0xC1 => self.pop_bc(),
            0xC2 => self.jp_nz_u16(),
            0xC3 => self.jp_u16(),
            0xC4 => self.call_nz_u16(),
            0xC5 => self.push_bc(),
            0xC6 => self.add_a_u8(),
            0xC7 => self.rst_00(),
            0xC8 => self.ret_z(),
            0xC9 => self.ret(),
            0xCA => self.jp_z_u16(),
            0xCB => self.cb_prefix(),
            0xCC => self.call_z_u16(),
            0xCD => self.call_u16(),
            0xCE => self.adc_a_u8(),
            0xCF => self.rst_08(),

            0xD0 => self.ret_nc(),
            0xD1 => self.pop_de(),
            0xD2 => self.jp_nc_u16(),
            0xD3 => self.op_d3_unused(),
            0xD4 => self.call_nc_u16(),
            0xD5 => self.push_de(),
            0xD6 => self.sub_u8(),
            0xD7 => self.rst_10(),
            0xD8 => self.ret_c(),
            0xD9 => self.reti(),
            0xDA => self.jp_c_u16(),
            0xDB => self.op_db_unused(),
            0xDC => self.call_c_u16(),
            0xDD => self.op_dd_unused(),
            0xDE => self.sbc_a_u8(),
            0xDF => self.rst_18(),

            0xE0 => self.ldh_u8_a(),
            0xE1 => self.pop_hl(),
            0xE2 => self.ldh_c_a(),
            0xE3 => self.op_e3_unused(),
            0xE4 => self.op_e4_unused(),
            0xE5 => self.push_hl(),
            0xE6 => self.and_u8(),
            0xE7 => self.rst_20(),
            0xE8 => self.add_sp_i8(),
            0xE9 => self.jp_hl(),
            0xEA => self.ld_u16_a(),
            0xEB => self.op_eb_unused(),
            0xEC => self.op_ec_unused(),
            0xED => self.op_ed_unused(),
            0xEE => self.xor_u8(),
            0xEF => self.rst_28(),

            0xF0 => self.ldh_a_u8(),
            0xF1 => self.pop_af(),
            0xF2 => self.ldh_a_c(),
            0xF3 => self.di(),
            0xF4 => self.op_f4_unused(),
            0xF5 => self.push_af(),
            0xF6 => self.or_u8(),
            0xF7 => self.rst_30(),
            0xF8 => self.ld_hl_sp_i8(),
            0xF9 => self.ld_sp_hl(),
            0xFA => self.ld_a_u16(),
            0xFB => self.ei(),
            0xFC => self.op_fc_unused(),
            0xFD => self.op_fd_unused(),
            0xFE => self.cp_u8(),
            0xFF => self.rst_38(),
        }
    }

    fn process_cb(&mut self, inst: u8) {
        match inst {
            0x00 => self.rlc_b(),
            0x01 => self.rlc_c(),
            0x02 => self.rlc_d(),
            0x03 => self.rlc_e(),
            0x04 => self.rlc_h(),
            0x05 => self.rlc_l(),
            0x06 => self.rlc_hl_ptr(),
            0x07 => self.rlc_a(),
            0x08 => self.rrc_b(),
            0x09 => self.rrc_c(),
            0x0A => self.rrc_d(),
            0x0B => self.rrc_e(),
            0x0C => self.rrc_h(),
            0x0D => self.rrc_l(),
            0x0E => self.rrc_hl_ptr(),
            0x0F => self.rrc_a(),

            0x10 => self.rl_b(),
            0x11 => self.rl_c(),
            0x12 => self.rl_d(),
            0x13 => self.rl_e(),
            0x14 => self.rl_h(),
            0x15 => self.rl_l(),
            0x16 => self.rl_hl_ptr(),
            0x17 => self.rl_a(),
            0x18 => self.rr_b(),
            0x19 => self.rr_c(),
            0x1A => self.rr_d(),
            0x1B => self.rr_e(),
            0x1C => self.rr_h(),
            0x1D => self.rr_l(),
            0x1E => self.rr_hl_ptr(),
            0x1F => self.rr_a(),

            0x20 => self.sla_b(),
            0x21 => self.sla_c(),
            0x22 => self.sla_d(),
            0x23 => self.sla_e(),
            0x24 => self.sla_h(),
            0x25 => self.sla_l(),
            0x26 => self.sla_hl_ptr(),
            0x27 => self.sla_a(),
            0x28 => self.sra_b(),
            0x29 => self.sra_c(),
            0x2A => self.sra_d(),
            0x2B => self.sra_e(),
            0x2C => self.sra_h(),
            0x2D => self.sra_l(),
            0x2E => self.sra_hl_ptr(),
            0x2F => self.sra_a(),

            0x30 => self.swap_b(),
            0x31 => self.swap_c(),
            0x32 => self.swap_d(),
            0x33 => self.swap_e(),
            0x34 => self.swap_h(),
            0x35 => self.swap_l(),
            0x36 => self.swap_hl_ptr(),
            0x37 => self.swap_a(),
            0x38 => self.srl_b(),
            0x39 => self.srl_c(),
            0x3A => self.srl_d(),
            0x3B => self.srl_e(),
            0x3C => self.srl_h(),
            0x3D => self.srl_l(),
            0x3E => self.srl_hl_ptr(),
            0x3F => self.srl_a(),

            0x40 => self.bit_0_b(),
            0x41 => self.bit_0_c(),
            0x42 => self.bit_0_d(),
            0x43 => self.bit_0_e(),
            0x44 => self.bit_0_h(),
            0x45 => self.bit_0_l(),
            0x46 => self.bit_0_hl_ptr(),
            0x47 => self.bit_0_a(),
            0x48 => self.bit_1_b(),
            0x49 => self.bit_1_c(),
            0x4A => self.bit_1_d(),
            0x4B => self.bit_1_e(),
            0x4C => self.bit_1_h(),
            0x4D => self.bit_1_l(),
            0x4E => self.bit_1_hl_ptr(),
            0x4F => self.bit_1_a(),

            0x50 => self.bit_2_b(),
            0x51 => self.bit_2_c(),
            0x52 => self.bit_2_d(),
            0x53 => self.bit_2_e(),
            0x54 => self.bit_2_h(),
            0x55 => self.bit_2_l(),
            0x56 => self.bit_2_hl_ptr(),
            0x57 => self.bit_2_a(),
            0x58 => self.bit_3_b(),
            0x59 => self.bit_3_c(),
            0x5A => self.bit_3_d(),
            0x5B => self.bit_3_e(),
            0x5C => self.bit_3_h(),
            0x5D => self.bit_3_l(),
            0x5E => self.bit_3_hl_ptr(),
            0x5F => self.bit_3_a(),

            0x60 => self.bit_4_b(),
            0x61 => self.bit_4_c(),
            0x62 => self.bit_4_d(),
            0x63 => self.bit_4_e(),
            0x64 => self.bit_4_h(),
            0x65 => self.bit_4_l(),
            0x66 => self.bit_4_hl_ptr(),
            0x67 => self.bit_4_a(),
            0x68 => self.bit_5_b(),
            0x69 => self.bit_5_c(),
            0x6A => self.bit_5_d(),
            0x6B => self.bit_5_e(),
            0x6C => self.bit_5_h(),
            0x6D => self.bit_5_l(),
            0x6E => self.bit_5_hl_ptr(),
            0x6F => self.bit_5_a(),

            0x70 => self.bit_6_b(),
            0x71 => self.bit_6_c(),
            0x72 => self.bit_6_d(),
            0x73 => self.bit_6_e(),
            0x74 => self.bit_6_h(),
            0x75 => self.bit_6_l(),
            0x76 => self.bit_6_hl_ptr(),
            0x77 => self.bit_6_a(),
            0x78 => self.bit_7_b(),
            0x79 => self.bit_7_c(),
            0x7A => self.bit_7_d(),
            0x7B => self.bit_7_e(),
            0x7C => self.bit_7_h(),
            0x7D => self.bit_7_l(),
            0x7E => self.bit_7_hl_ptr(),
            0x7F => self.bit_7_a(),

            0x80 => self.res_0_b(),
            0x81 => self.res_0_c(),
            0x82 => self.res_0_d(),
            0x83 => self.res_0_e(),
            0x84 => self.res_0_h(),
            0x85 => self.res_0_l(),
            0x86 => self.res_0_hl_ptr(),
            0x87 => self.res_0_a(),
            0x88 => self.res_1_b(),
            0x89 => self.res_1_c(),
            0x8A => self.res_1_d(),
            0x8B => self.res_1_e(),
            0x8C => self.res_1_h(),
            0x8D => self.res_1_l(),
            0x8E => self.res_1_hl_ptr(),
            0x8F => self.res_1_a(),

            0x90 => self.res_2_b(),
            0x91 => self.res_2_c(),
            0x92 => self.res_2_d(),
            0x93 => self.res_2_e(),
            0x94 => self.res_2_h(),
            0x95 => self.res_2_l(),
            0x96 => self.res_2_hl_ptr(),
            0x97 => self.res_2_a(),
            0x98 => self.res_3_b(),
            0x99 => self.res_3_c(),
            0x9A => self.res_3_d(),
            0x9B => self.res_3_e(),
            0x9C => self.res_3_h(),
            0x9D => self.res_3_l(),
            0x9E => self.res_3_hl_ptr(),
            0x9F => self.res_3_a(),

            0xA0 => self.res_4_b(),
            0xA1 => self.res_4_c(),
            0xA2 => self.res_4_d(),
            0xA3 => self.res_4_e(),
            0xA4 => self.res_4_h(),
            0xA5 => self.res_4_l(),
            0xA6 => self.res_4_hl_ptr(),
            0xA7 => self.res_4_a(),
            0xA8 => self.res_5_b(),
            0xA9 => self.res_5_c(),
            0xAA => self.res_5_d(),
            0xAB => self.res_5_e(),
            0xAC => self.res_5_h(),
            0xAD => self.res_5_l(),
            0xAE => self.res_5_hl_ptr(),
            0xAF => self.res_5_a(),

            0xB0 => self.res_6_b(),
            0xB1 => self.res_6_c(),
            0xB2 => self.res_6_d(),
            0xB3 => self.res_6_e(),
            0xB4 => self.res_6_h(),
            0xB5 => self.res_6_l(),
            0xB6 => self.res_6_hl_ptr(),
            0xB7 => self.res_6_a(),
            0xB8 => self.res_7_b(),
            0xB9 => self.res_7_c(),
            0xBA => self.res_7_d(),
            0xBB => self.res_7_e(),
            0xBC => self.res_7_h(),
            0xBD => self.res_7_l(),
            0xBE => self.res_7_hl_ptr(),
            0xBF => self.res_7_a(),

            0xC0 => self.set_0_b(),
            0xC1 => self.set_0_c(),
            0xC2 => self.set_0_d(),
            0xC3 => self.set_0_e(),
            0xC4 => self.set_0_h(),
            0xC5 => self.set_0_l(),
            0xC6 => self.set_0_hl_ptr(),
            0xC7 => self.set_0_a(),
            0xC8 => self.set_1_b(),
            0xC9 => self.set_1_c(),
            0xCA => self.set_1_d(),
            0xCB => self.set_1_e(),
            0xCC => self.set_1_h(),
            0xCD => self.set_1_l(),
            0xCE => self.set_1_hl_ptr(),
            0xCF => self.set_1_a(),

            0xD0 => self.set_2_b(),
            0xD1 => self.set_2_c(),
            0xD2 => self.set_2_d(),
            0xD3 => self.set_2_e(),
            0xD4 => self.set_2_h(),
            0xD5 => self.set_2_l(),
            0xD6 => self.set_2_hl_ptr(),
            0xD7 => self.set_2_a(),
            0xD8 => self.set_3_b(),
            0xD9 => self.set_3_c(),
            0xDA => self.set_3_d(),
            0xDB => self.set_3_e(),
            0xDC => self.set_3_h(),
            0xDD => self.set_3_l(),
            0xDE => self.set_3_hl_ptr(),
            0xDF => self.set_3_a(),

            0xE0 => self.set_4_b(),
            0xE1 => self.set_4_c(),
            0xE2 => self.set_4_d(),
            0xE3 => self.set_4_e(),
            0xE4 => self.set_4_h(),
            0xE5 => self.set_4_l(),
            0xE6 => self.set_4_hl_ptr(),
            0xE7 => self.set_4_a(),
            0xE8 => self.set_5_b(),
            0xE9 => self.set_5_c(),
            0xEA => self.set_5_d(),
            0xEB => self.set_5_e(),
            0xEC => self.set_5_h(),
            0xED => self.set_5_l(),
            0xEE => self.set_5_hl_ptr(),
            0xEF => self.set_5_a(),

            0xF0 => self.set_6_b(),
            0xF1 => self.set_6_c(),
            0xF2 => self.set_6_d(),
            0xF3 => self.set_6_e(),
            0xF4 => self.set_6_h(),
            0xF5 => self.set_6_l(),
            0xF6 => self.set_6_hl_ptr(),
            0xF7 => self.set_6_a(),
            0xF8 => self.set_7_b(),
            0xF9 => self.set_7_c(),
            0xFA => self.set_7_d(),
            0xFB => self.set_7_e(),
            0xFC => self.set_7_h(),
            0xFD => self.set_7_l(),
            0xFE => self.set_7_hl_ptr(),
            0xFF => self.set_7_a(),
        }
    }

    fn update_cycles(&mut self, cycles: u8) {
        self.cycles = cycles;
    }

    fn advance_program_counter(&mut self, n: u16) {
        self.program_counter = self.program_counter.wrapping_add(n);
    }

    fn read_u8(&mut self, addr: u16) -> u8 {
        self.memory_bus.read(addr)
    }

    fn register_concat(&self, high: u8, low: u8) -> u16 {
        ((high as u16) << 8) | (low as u16)
    }

    fn inc(&mut self, register: u8) -> u8 {
        self.register_f.remove(FFlags::N);

        let old = register;
        let result = old.wrapping_add(1);

        self.register_f.set(FFlags::Z, result == 0x00);
        self.register_f.set(FFlags::H, (old & 0x0F) == 0x0F);

        result
    }

    fn dec(&mut self, register: u8) -> u8 {
        self.register_f.insert(FFlags::N);

        let old = register;
        let result = old.wrapping_sub(1);

        self.register_f.set(FFlags::Z, result == 0x00);
        self.register_f.set(FFlags::H, (old & 0x0F) == 0x00);

        result
    }

    fn rlc(&mut self, register: u8) -> u8 {
        let bit7 = (register & 0b1000_0000) != 0;
        let result = register.rotate_left(1);

        self.register_f.set(FFlags::Z, result == 0x00);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.set(FFlags::C, bit7);

        result
    }

    fn push(&mut self, value: u16) {
        let upper = (value >> 8) as u8;
        let lower = value as u8;

        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory_bus.write(self.stack_pointer, upper);

        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory_bus.write(self.stack_pointer, lower);
    }

    fn nop(&mut self) {
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    // d16 imediato (little-endian): low = PC+1, high = PC+2
    fn ld_bc_u16(&mut self) {
        let low = self.read_u8(self.program_counter.wrapping_add(1));
        let high = self.read_u8(self.program_counter.wrapping_add(2));

        self.register_c = low;
        self.register_b = high;

        self.advance_program_counter(3);
        self.update_cycles(12);
    }

    fn ld_bc_a(&mut self) {
        let addr = self.register_concat(self.register_b, self.register_c);
        self.memory_bus.write(addr, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_bc(&mut self) {
        let bc = self.register_concat(self.register_b, self.register_c);
        let bc = bc.wrapping_add(1);

        self.register_b = (bc >> 8) as u8;
        self.register_c = bc as u8;

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
        self.register_b = self.read_u8(self.program_counter.wrapping_add(1));

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rlca(&mut self) {
        let bit7 = (self.register_a & 0b1000_0000) != 0;
        let result = self.register_a.rotate_left(1);

        // RLCA: Z sempre 0, N=0, H=0, C=bit7
        self.register_f.remove(FFlags::Z);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.set(FFlags::C, bit7);

        self.register_a = result;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_u16_sp(&mut self) {
        let low = self.read_u8(self.program_counter + 1);
        let high = self.read_u8(self.program_counter + 2);
        let addr = (low as u16) | ((high as u16) << 8);

        let sp_low = self.stack_pointer as u8;
        let sp_high = (self.stack_pointer >> 8) as u8;

        self.memory_bus.write(addr, sp_low);
        self.memory_bus.write(addr.wrapping_add(1), sp_high);

        self.advance_program_counter(3);
        self.update_cycles(20);
    }

    fn add_hl_bc(&mut self) {
        let hl = self.register_concat(self.register_h, self.register_l);
        let bc = self.register_concat(self.register_b, self.register_c);

        let result = hl.wrapping_add(bc);

        self.register_f.remove(FFlags::N);

        let half_carry = ((hl & 0x0FFF) + (bc & 0x0FFF)) > 0x0FFF;
        self.register_f.set(FFlags::H, half_carry);

        let carry = (hl as u32 + bc as u32) > 0xFFFF;
        self.register_f.set(FFlags::C, carry);

        self.register_h = (result >> 8) as u8;
        self.register_l = result as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_a_bc(&mut self) {
        let addr = self.register_concat(self.register_b, self.register_c);
        self.register_a = self.read_u8(addr);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn dec_bc(&mut self) {
        let bc = self.regqister_concat(self.register_b, self.register_c);
        let bc = bc.wrapping_sub(1);

        self.register_b = (bc >> 8) as u8;
        self.register_c = bc as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_c(&mut self) {
        self.register_c = self.inc(self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn dec_c(&mut self) {
        self.register_c = self.dec(self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_c_u8(&mut self) {
        self.register_c = self.read_u8(self.program_counter.wrapping_add(1));

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rrca(&mut self) {
        let bit0 = (self.register_a & 0b0000_0001) != 0;
        let result = self.register_a.rotate_right(1);

        // RLCA: Z sempre 0, N=0, H=0, C=bit0
        self.register_f.remove(FFlags::Z);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.set(FFlags::C, bit0);

        self.register_a = result;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn stop_inst(&mut self) {
        let next = self.read_u8(self.program_counter + 1);
        if next != 0x00 {
            panic!(
                "stop (0x10) inválido: esperado 0x00 após o opcode, mas veio 0x{:02x} em pc=0x{:04x}",
                next, self.program_counter
            );
        }

        self.stop = true;

        self.advance_program_counter(2);
        self.update_cycles(4);
    }

    fn ld_de_u16(&mut self) {
        let low = self.read_u8(self.program_counter.wrapping_add(1));
        let high = self.read_u8(self.program_counter.wrapping_add(2));

        self.register_d = low;
        self.register_e = high;

        self.advance_program_counter(3);
        self.update_cycles(12);
    }

    fn ld_de_a(&mut self) {
        let de = self.register_concat(self.register_d, self.register_e);
        self.register_a = self.read_u8(de);

        self.advance_program_counter(1);
        self.update_cycles(12);
    }

    fn inc_de(&mut self) {
        let de = self.regqister_concat(self.register_d, self.register_e);
        let de = de.wrapping_add(1);

        self.register_d = (de >> 8) as u8;
        self.register_e = de as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_d(&mut self) {}
    fn dec_d(&mut self) {}
    fn ld_d_u8(&mut self) {}
    fn rla(&mut self) {}
    fn jr_i8(&mut self) {}
    fn add_hl_de(&mut self) {}
    fn ld_a_de(&mut self) {}
    fn dec_de(&mut self) {}
    fn inc_e(&mut self) {}
    fn dec_e(&mut self) {}
    fn ld_e_u8(&mut self) {}
    fn rra(&mut self) {}

    fn jr_nz_i8(&mut self) {}
    fn ld_hl_u16(&mut self) {}
    fn ldi_hl_a(&mut self) {}

    fn inc_hl(&mut self) {
        let hl = self.regqister_concat(self.register_h, self.register_l);
        let hl = hl.wrapping_add(1);

        self.register_b = (hl >> 8) as u8;
        self.register_c = hl as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_h(&mut self) {}
    fn dec_h(&mut self) {}
    fn ld_h_u8(&mut self) {}
    fn daa(&mut self) {}
    fn jr_z_i8(&mut self) {}
    fn add_hl_hl(&mut self) {}
    fn ldi_a_hl(&mut self) {}
    fn dec_hl(&mut self) {}
    fn inc_l(&mut self) {}
    fn dec_l(&mut self) {}
    fn ld_l_u8(&mut self) {}
    fn cpl(&mut self) {}

    fn jr_nc_i8(&mut self) {}
    fn ld_sp_u16(&mut self) {}
    fn ldd_hl_a(&mut self) {}
    
    fn inc_sp(&mut self) {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }
    
    fn inc_hl_ptr(&mut self) {}
    fn dec_hl_ptr(&mut self) {}
    fn ld_hl_ptr_u8(&mut self) {}
    fn scf(&mut self) {}
    fn jr_c_i8(&mut self) {}
    fn add_hl_sp(&mut self) {}
    fn ldd_a_hl(&mut self) {}
    fn dec_sp(&mut self) {}

    fn inc_a(&mut self) {
        self.register_a = self.inc(self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn dec_a(&mut self) {
        self.register_a = self.dec(self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_a_u8(&mut self) {}
    fn ccf(&mut self) {}

    fn ld_b_b(&mut self) {}
    fn ld_b_c(&mut self) {}
    fn ld_b_d(&mut self) {}
    fn ld_b_e(&mut self) {}
    fn ld_b_h(&mut self) {}
    fn ld_b_l(&mut self) {}
    fn ld_b_hl_ptr(&mut self) {}
    fn ld_b_a(&mut self) {}
    fn ld_c_b(&mut self) {}
    fn ld_c_c(&mut self) {}
    fn ld_c_d(&mut self) {}
    fn ld_c_e(&mut self) {}
    fn ld_c_h(&mut self) {}
    fn ld_c_l(&mut self) {}
    fn ld_c_hl_ptr(&mut self) {}
    fn ld_c_a(&mut self) {}

    fn ld_d_b(&mut self) {}
    fn ld_d_c(&mut self) {}
    fn ld_d_d(&mut self) {}
    fn ld_d_e(&mut self) {}
    fn ld_d_h(&mut self) {}
    fn ld_d_l(&mut self) {}
    fn ld_d_hl_ptr(&mut self) {}
    fn ld_d_a(&mut self) {}
    fn ld_e_b(&mut self) {}
    fn ld_e_c(&mut self) {}
    fn ld_e_d(&mut self) {}
    fn ld_e_e(&mut self) {}
    fn ld_e_h(&mut self) {}
    fn ld_e_l(&mut self) {}
    fn ld_e_hl_ptr(&mut self) {}
    fn ld_e_a(&mut self) {}

    fn ld_h_b(&mut self) {}
    fn ld_h_c(&mut self) {}
    fn ld_h_d(&mut self) {}
    fn ld_h_e(&mut self) {}
    fn ld_h_h(&mut self) {}
    fn ld_h_l(&mut self) {}
    fn ld_h_hl_ptr(&mut self) {}
    fn ld_h_a(&mut self) {}
    fn ld_l_b(&mut self) {}
    fn ld_l_c(&mut self) {}
    fn ld_l_d(&mut self) {}
    fn ld_l_e(&mut self) {}
    fn ld_l_h(&mut self) {}
    fn ld_l_l(&mut self) {}
    fn ld_l_hl_ptr(&mut self) {}
    fn ld_l_a(&mut self) {}

    fn ld_hl_ptr_b(&mut self) {}
    fn ld_hl_ptr_c(&mut self) {}
    fn ld_hl_ptr_d(&mut self) {}
    fn ld_hl_ptr_e(&mut self) {}
    fn ld_hl_ptr_h(&mut self) {}
    fn ld_hl_ptr_l(&mut self) {}
    fn halt_inst(&mut self) {}
    fn ld_hl_ptr_a(&mut self) {}
    fn ld_a_b(&mut self) {}
    fn ld_a_c(&mut self) {}
    fn ld_a_d(&mut self) {}
    fn ld_a_e(&mut self) {}
    fn ld_a_h(&mut self) {}
    fn ld_a_l(&mut self) {}
    fn ld_a_hl_ptr(&mut self) {}
    fn ld_a_a(&mut self) {}

    fn add_a_b(&mut self) {}
    fn add_a_c(&mut self) {}
    fn add_a_d(&mut self) {}
    fn add_a_e(&mut self) {}
    fn add_a_h(&mut self) {}
    fn add_a_l(&mut self) {}
    fn add_a_hl_ptr(&mut self) {}
    fn add_a_a(&mut self) {}

    fn adc_a_b(&mut self) {}
    fn adc_a_c(&mut self) {}
    fn adc_a_d(&mut self) {}
    fn adc_a_e(&mut self) {}
    fn adc_a_h(&mut self) {}
    fn adc_a_l(&mut self) {}
    fn adc_a_hl_ptr(&mut self) {}
    fn adc_a_a(&mut self) {}

    fn sub_b(&mut self) {}
    fn sub_c(&mut self) {}
    fn sub_d(&mut self) {}
    fn sub_e(&mut self) {}
    fn sub_h(&mut self) {}
    fn sub_l(&mut self) {}
    fn sub_hl_ptr(&mut self) {}
    fn sub_a(&mut self) {}

    fn sbc_a_b(&mut self) {}
    fn sbc_a_c(&mut self) {}
    fn sbc_a_d(&mut self) {}
    fn sbc_a_e(&mut self) {}
    fn sbc_a_h(&mut self) {}
    fn sbc_a_l(&mut self) {}
    fn sbc_a_hl_ptr(&mut self) {}
    fn sbc_a_a(&mut self) {}

    fn and_b(&mut self) {}
    fn and_c(&mut self) {}
    fn and_d(&mut self) {}
    fn and_e(&mut self) {}
    fn and_h(&mut self) {}
    fn and_l(&mut self) {}
    fn and_hl_ptr(&mut self) {}
    fn and_a(&mut self) {}

    fn xor_b(&mut self) {}
    fn xor_c(&mut self) {}
    fn xor_d(&mut self) {}
    fn xor_e(&mut self) {}
    fn xor_h(&mut self) {}
    fn xor_l(&mut self) {}
    fn xor_hl_ptr(&mut self) {}
    fn xor_a(&mut self) {}

    fn or_b(&mut self) {}
    fn or_c(&mut self) {}
    fn or_d(&mut self) {}
    fn or_e(&mut self) {}
    fn or_h(&mut self) {}
    fn or_l(&mut self) {}
    fn or_hl_ptr(&mut self) {}
    fn or_a(&mut self) {}

    fn cp_b(&mut self) {}
    fn cp_c(&mut self) {}
    fn cp_d(&mut self) {}
    fn cp_e(&mut self) {}
    fn cp_h(&mut self) {}
    fn cp_l(&mut self) {}
    fn cp_hl_ptr(&mut self) {}
    fn cp_a(&mut self) {}

    fn ret_nz(&mut self) {}
    fn pop_bc(&mut self) {}
    fn jp_nz_u16(&mut self) {}
    fn jp_u16(&mut self) {}
    fn call_nz_u16(&mut self) {}
    fn push_bc(&mut self) {}
    fn add_a_u8(&mut self) {}
    fn rst_00(&mut self) {}
    fn ret_z(&mut self) {}
    fn ret(&mut self) {}
    fn jp_z_u16(&mut self) {}

    fn cb_prefix(&mut self) {
        self.advance_program_counter(1);

        let inst = self.memory_bus.read(self.program_counter);
        self.opcode = inst;

        self.process_cb(inst);
    }

    fn call_z_u16(&mut self) {}
    fn call_u16(&mut self) {}
    fn adc_a_u8(&mut self) {}
    fn rst_08(&mut self) {}

    fn ret_nc(&mut self) {}
    fn pop_de(&mut self) {}
    fn jp_nc_u16(&mut self) {}
    fn op_d3_unused(&mut self) {}
    fn call_nc_u16(&mut self) {}
    fn push_de(&mut self) {}
    fn sub_u8(&mut self) {}
    fn rst_10(&mut self) {}
    fn ret_c(&mut self) {}
    fn reti(&mut self) {}
    fn jp_c_u16(&mut self) {}
    fn op_db_unused(&mut self) {}
    fn call_c_u16(&mut self) {}
    fn op_dd_unused(&mut self) {}
    fn sbc_a_u8(&mut self) {}
    fn rst_18(&mut self) {}

    fn ldh_u8_a(&mut self) {}
    fn pop_hl(&mut self) {}
    fn ldh_c_a(&mut self) {}
    fn op_e3_unused(&mut self) {}
    fn op_e4_unused(&mut self) {}
    fn push_hl(&mut self) {}
    fn and_u8(&mut self) {}
    fn rst_20(&mut self) {}
    fn add_sp_i8(&mut self) {}
    fn jp_hl(&mut self) {}
    fn ld_u16_a(&mut self) {}
    fn op_eb_unused(&mut self) {}
    fn op_ec_unused(&mut self) {}
    fn op_ed_unused(&mut self) {}
    fn xor_u8(&mut self) {}
    fn rst_28(&mut self) {}

    fn ldh_a_u8(&mut self) {}
    fn pop_af(&mut self) {}
    fn ldh_a_c(&mut self) {}
    fn di(&mut self) {}
    fn op_f4_unused(&mut self) {}
    fn push_af(&mut self) {}
    fn or_u8(&mut self) {}
    fn rst_30(&mut self) {}
    fn ld_hl_sp_i8(&mut self) {}
    fn ld_sp_hl(&mut self) {}
    fn ld_a_u16(&mut self) {}
    fn ei(&mut self) {}
    fn op_fc_unused(&mut self) {}
    fn op_fd_unused(&mut self) {}
    fn cp_u8(&mut self) {}

    fn rst_38(&mut self) {
        let ret = self.program_counter.wrapping_add(1);
        self.push(ret);

        self.program_counter = 0x0038;
        self.update_cycles(16);
    }

    fn rlc_b(&mut self) {}
    fn rlc_c(&mut self) {}
    fn rlc_d(&mut self) {}
    fn rlc_e(&mut self) {}
    fn rlc_h(&mut self) {}
    fn rlc_l(&mut self) {}
    fn rlc_hl_ptr(&mut self) {}
    fn rlc_a(&mut self) {}

    fn rrc_b(&mut self) {}
    fn rrc_c(&mut self) {}
    fn rrc_d(&mut self) {}
    fn rrc_e(&mut self) {}
    fn rrc_h(&mut self) {}
    fn rrc_l(&mut self) {}
    fn rrc_hl_ptr(&mut self) {}
    fn rrc_a(&mut self) {}

    fn rl_b(&mut self) {}
    fn rl_c(&mut self) {}
    fn rl_d(&mut self) {}
    fn rl_e(&mut self) {}
    fn rl_h(&mut self) {}
    fn rl_l(&mut self) {}
    fn rl_hl_ptr(&mut self) {}
    fn rl_a(&mut self) {}

    fn rr_b(&mut self) {}
    fn rr_c(&mut self) {}
    fn rr_d(&mut self) {}
    fn rr_e(&mut self) {}
    fn rr_h(&mut self) {}
    fn rr_l(&mut self) {}
    fn rr_hl_ptr(&mut self) {}
    fn rr_a(&mut self) {}

    fn sla_b(&mut self) {}
    fn sla_c(&mut self) {}
    fn sla_d(&mut self) {}
    fn sla_e(&mut self) {}
    fn sla_h(&mut self) {}
    fn sla_l(&mut self) {}
    fn sla_hl_ptr(&mut self) {}
    fn sla_a(&mut self) {}

    fn sra_b(&mut self) {}
    fn sra_c(&mut self) {}
    fn sra_d(&mut self) {}
    fn sra_e(&mut self) {}
    fn sra_h(&mut self) {}
    fn sra_l(&mut self) {}
    fn sra_hl_ptr(&mut self) {}
    fn sra_a(&mut self) {}

    fn swap_b(&mut self) {}
    fn swap_c(&mut self) {}
    fn swap_d(&mut self) {}
    fn swap_e(&mut self) {}
    fn swap_h(&mut self) {}
    fn swap_l(&mut self) {}
    fn swap_hl_ptr(&mut self) {}
    fn swap_a(&mut self) {}

    fn srl_b(&mut self) {}
    fn srl_c(&mut self) {}
    fn srl_d(&mut self) {}
    fn srl_e(&mut self) {}
    fn srl_h(&mut self) {}
    fn srl_l(&mut self) {}
    fn srl_hl_ptr(&mut self) {}
    fn srl_a(&mut self) {}

    fn bit_0_b(&mut self) {}
    fn bit_0_c(&mut self) {}
    fn bit_0_d(&mut self) {}
    fn bit_0_e(&mut self) {}
    fn bit_0_h(&mut self) {}
    fn bit_0_l(&mut self) {}
    fn bit_0_hl_ptr(&mut self) {}
    fn bit_0_a(&mut self) {}

    fn bit_1_b(&mut self) {}
    fn bit_1_c(&mut self) {}
    fn bit_1_d(&mut self) {}
    fn bit_1_e(&mut self) {}
    fn bit_1_h(&mut self) {}
    fn bit_1_l(&mut self) {}
    fn bit_1_hl_ptr(&mut self) {}
    fn bit_1_a(&mut self) {}

    fn bit_2_b(&mut self) {}
    fn bit_2_c(&mut self) {}
    fn bit_2_d(&mut self) {}
    fn bit_2_e(&mut self) {}
    fn bit_2_h(&mut self) {}
    fn bit_2_l(&mut self) {}
    fn bit_2_hl_ptr(&mut self) {}
    fn bit_2_a(&mut self) {}

    fn bit_3_b(&mut self) {}
    fn bit_3_c(&mut self) {}
    fn bit_3_d(&mut self) {}
    fn bit_3_e(&mut self) {}
    fn bit_3_h(&mut self) {}
    fn bit_3_l(&mut self) {}
    fn bit_3_hl_ptr(&mut self) {}
    fn bit_3_a(&mut self) {}

    fn bit_4_b(&mut self) {}
    fn bit_4_c(&mut self) {}
    fn bit_4_d(&mut self) {}
    fn bit_4_e(&mut self) {}
    fn bit_4_h(&mut self) {}
    fn bit_4_l(&mut self) {}
    fn bit_4_hl_ptr(&mut self) {}
    fn bit_4_a(&mut self) {}

    fn bit_5_b(&mut self) {}
    fn bit_5_c(&mut self) {}
    fn bit_5_d(&mut self) {}
    fn bit_5_e(&mut self) {}
    fn bit_5_h(&mut self) {}
    fn bit_5_l(&mut self) {}
    fn bit_5_hl_ptr(&mut self) {}
    fn bit_5_a(&mut self) {}

    fn bit_6_b(&mut self) {}
    fn bit_6_c(&mut self) {}
    fn bit_6_d(&mut self) {}
    fn bit_6_e(&mut self) {}
    fn bit_6_h(&mut self) {}
    fn bit_6_l(&mut self) {}
    fn bit_6_hl_ptr(&mut self) {}
    fn bit_6_a(&mut self) {}

    fn bit_7_b(&mut self) {}
    fn bit_7_c(&mut self) {}
    fn bit_7_d(&mut self) {}
    fn bit_7_e(&mut self) {}
    fn bit_7_h(&mut self) {}
    fn bit_7_l(&mut self) {}
    fn bit_7_hl_ptr(&mut self) {}
    fn bit_7_a(&mut self) {}

    fn res_0_b(&mut self) {}
    fn res_0_c(&mut self) {}
    fn res_0_d(&mut self) {}
    fn res_0_e(&mut self) {}
    fn res_0_h(&mut self) {}
    fn res_0_l(&mut self) {}
    fn res_0_hl_ptr(&mut self) {}
    fn res_0_a(&mut self) {}

    fn res_1_b(&mut self) {}
    fn res_1_c(&mut self) {}
    fn res_1_d(&mut self) {}
    fn res_1_e(&mut self) {}
    fn res_1_h(&mut self) {}
    fn res_1_l(&mut self) {}
    fn res_1_hl_ptr(&mut self) {}
    fn res_1_a(&mut self) {}

    fn res_2_b(&mut self) {}
    fn res_2_c(&mut self) {}
    fn res_2_d(&mut self) {}
    fn res_2_e(&mut self) {}
    fn res_2_h(&mut self) {}
    fn res_2_l(&mut self) {}
    fn res_2_hl_ptr(&mut self) {}
    fn res_2_a(&mut self) {}

    fn res_3_b(&mut self) {}
    fn res_3_c(&mut self) {}
    fn res_3_d(&mut self) {}
    fn res_3_e(&mut self) {}
    fn res_3_h(&mut self) {}
    fn res_3_l(&mut self) {}
    fn res_3_hl_ptr(&mut self) {}
    fn res_3_a(&mut self) {}

    fn res_4_b(&mut self) {}
    fn res_4_c(&mut self) {}
    fn res_4_d(&mut self) {}
    fn res_4_e(&mut self) {}
    fn res_4_h(&mut self) {}
    fn res_4_l(&mut self) {}
    fn res_4_hl_ptr(&mut self) {}
    fn res_4_a(&mut self) {}

    fn res_5_b(&mut self) {}
    fn res_5_c(&mut self) {}
    fn res_5_d(&mut self) {}
    fn res_5_e(&mut self) {}
    fn res_5_h(&mut self) {}
    fn res_5_l(&mut self) {}
    fn res_5_hl_ptr(&mut self) {}
    fn res_5_a(&mut self) {}

    fn res_6_b(&mut self) {}
    fn res_6_c(&mut self) {}
    fn res_6_d(&mut self) {}
    fn res_6_e(&mut self) {}
    fn res_6_h(&mut self) {}
    fn res_6_l(&mut self) {}
    fn res_6_hl_ptr(&mut self) {}
    fn res_6_a(&mut self) {}

    fn res_7_b(&mut self) {}
    fn res_7_c(&mut self) {}
    fn res_7_d(&mut self) {}
    fn res_7_e(&mut self) {}
    fn res_7_h(&mut self) {}
    fn res_7_l(&mut self) {}
    fn res_7_hl_ptr(&mut self) {}
    fn res_7_a(&mut self) {}

    fn set_0_b(&mut self) {}
    fn set_0_c(&mut self) {}
    fn set_0_d(&mut self) {}
    fn set_0_e(&mut self) {}
    fn set_0_h(&mut self) {}
    fn set_0_l(&mut self) {}
    fn set_0_hl_ptr(&mut self) {}
    fn set_0_a(&mut self) {}

    fn set_1_b(&mut self) {}
    fn set_1_c(&mut self) {}
    fn set_1_d(&mut self) {}
    fn set_1_e(&mut self) {}
    fn set_1_h(&mut self) {}
    fn set_1_l(&mut self) {}
    fn set_1_hl_ptr(&mut self) {}
    fn set_1_a(&mut self) {}

    fn set_2_b(&mut self) {}
    fn set_2_c(&mut self) {}
    fn set_2_d(&mut self) {}
    fn set_2_e(&mut self) {}
    fn set_2_h(&mut self) {}
    fn set_2_l(&mut self) {}
    fn set_2_hl_ptr(&mut self) {}
    fn set_2_a(&mut self) {}

    fn set_3_b(&mut self) {}
    fn set_3_c(&mut self) {}
    fn set_3_d(&mut self) {}
    fn set_3_e(&mut self) {}
    fn set_3_h(&mut self) {}
    fn set_3_l(&mut self) {}
    fn set_3_hl_ptr(&mut self) {}
    fn set_3_a(&mut self) {}

    fn set_4_b(&mut self) {}
    fn set_4_c(&mut self) {}
    fn set_4_d(&mut self) {}
    fn set_4_e(&mut self) {}
    fn set_4_h(&mut self) {}
    fn set_4_l(&mut self) {}
    fn set_4_hl_ptr(&mut self) {}
    fn set_4_a(&mut self) {}

    fn set_5_b(&mut self) {}
    fn set_5_c(&mut self) {}
    fn set_5_d(&mut self) {}
    fn set_5_e(&mut self) {}
    fn set_5_h(&mut self) {}
    fn set_5_l(&mut self) {}
    fn set_5_hl_ptr(&mut self) {}
    fn set_5_a(&mut self) {}

    fn set_6_b(&mut self) {}
    fn set_6_c(&mut self) {}
    fn set_6_d(&mut self) {}
    fn set_6_e(&mut self) {}
    fn set_6_h(&mut self) {}
    fn set_6_l(&mut self) {}
    fn set_6_hl_ptr(&mut self) {}
    fn set_6_a(&mut self) {}

    fn set_7_b(&mut self) {}
    fn set_7_c(&mut self) {}
    fn set_7_d(&mut self) {}
    fn set_7_e(&mut self) {}
    fn set_7_h(&mut self) {}
    fn set_7_l(&mut self) {}
    fn set_7_hl_ptr(&mut self) {}
    fn set_7_a(&mut self) {}
}
