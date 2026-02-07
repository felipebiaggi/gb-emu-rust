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

bitflags! {
    pub struct InterruptFlags: u8 {
        const VBLANK  = 1 << 0;
        const LCDSTAT = 1 << 1;
        const TIMER   = 1 << 2;
        const SERIAL  = 1 << 3;
        const JOYPAD  = 1 << 4;
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
    pub ime_pending: bool,

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
            ime_pending: false,

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

        self.memory_bus.write(0xFF0F, 0xE1); // IF
        self.memory_bus.write(0xFFFF, 0x00); // IE
        self.interruption = false;
        self.ime_pending = false;
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

        self.cycles = 0;

        let inst = self.memory_bus.read(self.program_counter);
        self.opcode = inst;

        self.process(inst);

        if self.ime_pending {
            self.interruption = true;
            self.ime_pending = false;
        }

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

            0x90 => self.sub_a_b(),
            0x91 => self.sub_a_c(),
            0x92 => self.sub_a_d(),
            0x93 => self.sub_a_e(),
            0x94 => self.sub_a_h(),
            0x95 => self.sub_a_l(),
            0x96 => self.sub_a_hl_ptr(),
            0x97 => self.sub_a_a(),
            0x98 => self.sbc_a_b(),
            0x99 => self.sbc_a_c(),
            0x9A => self.sbc_a_d(),
            0x9B => self.sbc_a_e(),
            0x9C => self.sbc_a_h(),
            0x9D => self.sbc_a_l(),
            0x9E => self.sbc_a_hl_ptr(),
            0x9F => self.sbc_a_a(),

            0xA0 => self.and_a_b(),
            0xA1 => self.and_a_c(),
            0xA2 => self.and_a_d(),
            0xA3 => self.and_a_e(),
            0xA4 => self.and_a_h(),
            0xA5 => self.and_a_l(),
            0xA6 => self.and_a_hl_ptr(),
            0xA7 => self.and_a_a(),

            0xA8 => self.xor_a_b(),
            0xA9 => self.xor_a_c(),
            0xAA => self.xor_a_d(),
            0xAB => self.xor_a_e(),
            0xAC => self.xor_a_h(),
            0xAD => self.xor_a_l(),
            0xAE => self.xor_a_hl_ptr(),
            0xAF => self.xor_a_a(),

            0xB0 => self.or_a_b(),
            0xB1 => self.or_a_c(),
            0xB2 => self.or_a_d(),
            0xB3 => self.or_a_e(),
            0xB4 => self.or_a_h(),
            0xB5 => self.or_a_l(),
            0xB6 => self.or_a_hl_ptr(),
            0xB7 => self.or_a_a(),

            0xB8 => self.cp_a_b(),
            0xB9 => self.cp_a_c(),
            0xBA => self.cp_a_d(),
            0xBB => self.cp_a_e(),
            0xBC => self.cp_a_h(),
            0xBD => self.cp_a_l(),
            0xBE => self.cp_a_hl_ptr(),
            0xBF => self.cp_a_a(),

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

    fn write_u8(&mut self, addr: u16, data: u8) {
        self.memory_bus.write(addr, data);
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

    fn rrc(&mut self, register: u8) -> u8 {
        let bit0 = (register & 0b0000_0001) != 0;
        let result = register.rotate_right(1);

        self.register_f.set(FFlags::Z, result == 0x00);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.set(FFlags::C, bit0);

        result
    }

    fn rl(&mut self, value: u8) -> u8 {
        let old_carry = self.register_f.contains(FFlags::C);

        let new_carry = (value & 0b1000_0000) != 0;

        let result = (value << 1) | (old_carry as u8);

        self.register_f
            .remove(FFlags::Z | FFlags::N | FFlags::H | FFlags::C);
        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.set(FFlags::C, new_carry);

        result
    }

    fn rr(&mut self, value: u8) -> u8 {
        let old_carry = self.register_f.contains(FFlags::C);

        let new_carry = (value & 0b0000_0001) != 0;

        let result = (value >> 1) | ((old_carry as u8) << 7);

        self.register_f
            .remove(FFlags::Z | FFlags::N | FFlags::H | FFlags::C);
        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.set(FFlags::C, new_carry);

        result
    }

    fn sla(&mut self, value: u8) -> u8 {
        let new_carry = (value & 0b1000_0000) != 0;
        let result = value << 1;

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.set(FFlags::C, new_carry);

        result
    }

    fn sra(&mut self, value: u8) -> u8 {
        let old_bit7 = value & 0b1000_0000;
        let new_carry = (value & 0b0000_0001) != 0;

        let result = (value >> 1) | old_bit7;

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.set(FFlags::C, new_carry);

        result
    }

    fn swap(&mut self, value: u8) -> u8 {
        let result = (value >> 4) | (value << 4);

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.remove(FFlags::C);

        result
    }

    fn srl(&mut self, value: u8) -> u8 {
        let new_carry = (value & 0b0000_0001) != 0;
        let result = value >> 1;

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.set(FFlags::C, new_carry);

        result
    }

    fn bit(&mut self, value: u8, bit: u8) {
        let mask = 1u8 << bit;
        let result = value & mask;

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.remove(FFlags::N);
        self.register_f.insert(FFlags::H);
    }

    fn res(&mut self, value: u8, bit: u8) -> u8 {
        value & !(1u8 << bit)
    }

    fn set(&mut self, value: u8, bit: u8) -> u8 {
        value | (1u8 << bit)
    }

    fn push_u16(&mut self, value: u16) {
        let upper = (value >> 8) as u8;
        let lower = value as u8;

        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory_bus.write(self.stack_pointer, upper);

        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        self.memory_bus.write(self.stack_pointer, lower);
    }

    fn pop_u8(&mut self) -> u8 {
        let value = self.read_u8(self.stack_pointer);
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        value
    }

    fn jr_cond_i8(&mut self, condition: bool) {
        /*
            Byte lido da memória (u8):
              0xFE (254)

            Interpretado como signed (i8):
              0xFE -> -2

            Reinterpretado como u16 (two’s complement):
              -2 -> 0xFFFE

            Soma com PC usando wrapping_add (módulo 2^16):
              PC=0x0102
              0x0102 + 0xFFFE = 0x0100
        */
        let offset = self.read_u8(self.program_counter.wrapping_add(1)) as i8 as u16;

        self.advance_program_counter(2);

        if condition {
            self.program_counter = self.program_counter.wrapping_add(offset);
            self.update_cycles(12);
        } else {
            self.update_cycles(8);
        }
    }

    fn add(&mut self, register_x: u8, register_y: u8) -> u8 {
        let (result, carry) = register_x.overflowing_add(register_y);

        self.register_f.set(FFlags::Z, result == 0x00);

        self.register_f.remove(FFlags::N);

        let half_carry = ((register_x & 0x0F) + (register_y & 0x0F)) > 0x0F;
        self.register_f.set(FFlags::H, half_carry);

        self.register_f.set(FFlags::C, carry);

        result
    }

    fn adc(&mut self, register_x: u8, register_y: u8) -> u8 {
        let carry_in: u8 = if self.register_f.contains(FFlags::C) {
            1
        } else {
            0
        };

        let (tmp, carry1) = register_x.overflowing_add(register_y);
        let (result, carry2) = tmp.overflowing_add(carry_in);

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.remove(FFlags::N);

        let half = (register_x & 0x0F) + (register_y & 0x0F) + carry_in;
        self.register_f.set(FFlags::H, half > 0x0F);

        self.register_f.set(FFlags::C, carry1 || carry2);

        result
    }

    fn sub(&mut self, register_x: u8, register_y: u8) -> u8 {
        let (result, borrow) = register_x.overflowing_sub(register_y);

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.insert(FFlags::N);

        // half-borrow do bit 4 (nibble baixo)
        self.register_f
            .set(FFlags::H, (register_x & 0x0F) < (register_y & 0x0F));

        // borrow total
        self.register_f.set(FFlags::C, borrow);

        result
    }

    fn sbc(&mut self, register_x: u8, register_y: u8) -> u8 {
        let carry_in: u8 = if self.register_f.contains(FFlags::C) {
            1
        } else {
            0
        };

        let (tmp, borrow1) = register_x.overflowing_sub(register_y);
        let (result, borrow2) = tmp.overflowing_sub(carry_in);

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.insert(FFlags::N);

        let y_plus_c = (register_y & 0x0F) + carry_in;
        self.register_f
            .set(FFlags::H, (register_x & 0x0F) < y_plus_c);

        self.register_f.set(FFlags::C, borrow1 || borrow2);

        result
    }

    fn and_(&mut self, register_x: u8, register_y: u8) -> u8 {
        let result = register_x & register_y;

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.remove(FFlags::N);
        self.register_f.insert(FFlags::H);
        self.register_f.remove(FFlags::C);

        result
    }

    fn xor(&mut self, register_x: u8, register_y: u8) -> u8 {
        let result = register_x ^ register_y;

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.remove(FFlags::C);

        result
    }

    fn or_(&mut self, register_x: u8, register_y: u8) -> u8 {
        let result = register_x | register_y;

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);
        self.register_f.remove(FFlags::C);

        result
    }

    fn cp(&mut self, register_x: u8, register_y: u8) {
        let (result, borrow) = register_x.overflowing_sub(register_y);

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.insert(FFlags::N);
        self.register_f
            .set(FFlags::H, (register_x & 0x0F) < (register_y & 0x0F));
        self.register_f.set(FFlags::C, borrow);
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
        let low = self.read_u8(self.program_counter.wrapping_add(1));
        let high = self.read_u8(self.program_counter.wrapping_add(2));
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
        let bc = self.register_concat(self.register_b, self.register_c);
        let bc = bc.wrapping_sub(1);

        self.register_b = (bc >> 8) as u8;
        self.register_c = bc as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_c(&mut self) {
        self.register_c = self.inc(self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn dec_c(&mut self) {
        self.register_c = self.dec(self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(4);
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
        let next = self.read_u8(self.program_counter.wrapping_add(1));
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

        self.register_e = low;
        self.register_d = high;

        self.advance_program_counter(3);
        self.update_cycles(12);
    }

    fn ld_de_a(&mut self) {
        let de = self.register_concat(self.register_d, self.register_e);
        self.write_u8(de, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_de(&mut self) {
        let de = self.register_concat(self.register_d, self.register_e);
        let de = de.wrapping_add(1);

        self.register_d = (de >> 8) as u8;
        self.register_e = de as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_d(&mut self) {
        self.register_d = self.inc(self.register_d);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn dec_d(&mut self) {
        self.register_d = self.dec(self.register_d);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_d_u8(&mut self) {
        self.register_d = self.read_u8(self.program_counter.wrapping_add(1));

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rla(&mut self) {
        let old_carry = self.register_f.contains(FFlags::C);

        let new_carry = (self.register_a & 0x80) != 0;

        let result = (self.register_a << 1) | (old_carry as u8);

        self.register_a = result;

        self.register_f
            .remove(FFlags::Z | FFlags::N | FFlags::H | FFlags::C);
        self.register_f.set(FFlags::C, new_carry);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn jr_i8(&mut self) {
        let offset_u8 = self.read_u8(self.program_counter.wrapping_add(1));
        let offset = offset_u8 as i8 as i16;

        self.advance_program_counter(2);
        self.program_counter = self.program_counter.wrapping_add(offset as u16);

        self.update_cycles(12);
    }

    fn add_hl_de(&mut self) {
        let hl = self.register_concat(self.register_h, self.register_l);
        let de = self.register_concat(self.register_d, self.register_e);

        let result = hl.wrapping_add(de);

        self.register_f.remove(FFlags::N);

        let half_carry = ((hl & 0x0FFF) + (de & 0x0FFF)) > 0x0FFF;
        self.register_f.set(FFlags::H, half_carry);

        let carry = (hl as u32 + de as u32) > 0xFFFF;
        self.register_f.set(FFlags::C, carry);

        self.register_h = (result >> 8) as u8;
        self.register_l = result as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_a_de(&mut self) {
        let addr = self.register_concat(self.register_d, self.register_e);
        self.register_a = self.read_u8(addr);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn dec_de(&mut self) {
        let de = self.register_concat(self.register_d, self.register_e);
        let de = de.wrapping_sub(1);

        self.register_d = (de >> 8) as u8;
        self.register_e = de as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_e(&mut self) {
        self.register_e = self.inc(self.register_e);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn dec_e(&mut self) {
        self.register_e = self.dec(self.register_e);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_e_u8(&mut self) {
        self.register_e = self.read_u8(self.program_counter.wrapping_add(1));

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rra(&mut self) {
        let old_carry = self.register_f.contains(FFlags::C);

        let new_carry = (self.register_a & 0x01) != 0;

        let result = (self.register_a >> 1) | ((old_carry as u8) << 7);

        self.register_a = result;

        self.register_f
            .remove(FFlags::Z | FFlags::N | FFlags::H | FFlags::C);
        self.register_f.set(FFlags::C, new_carry);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn jr_nz_i8(&mut self) {
        let z_set = self.register_f.contains(FFlags::Z);

        self.jr_cond_i8(!z_set);
    }

    fn ld_hl_u16(&mut self) {
        let low = self.read_u8(self.program_counter.wrapping_add(1));
        let high = self.read_u8(self.program_counter.wrapping_add(2));

        self.register_h = high;
        self.register_l = low;

        self.advance_program_counter(3);
        self.update_cycles(12);
    }

    fn ldi_hl_a(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.memory_bus.write(addr, self.register_a);

        let addr_plus = addr.wrapping_add(1);
        self.register_h = ((addr_plus >> 8) & 0x00FF) as u8;
        self.register_l = (addr_plus & 0x00FF) as u8;
        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_hl(&mut self) {
        let hl = self.register_concat(self.register_h, self.register_l);
        let hl_plus = hl.wrapping_add(1);

        self.register_h = (hl_plus >> 8) as u8;
        self.register_l = hl_plus as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_h(&mut self) {
        self.register_h = self.inc(self.register_h);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn dec_h(&mut self) {
        self.register_h = self.dec(self.register_h);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_h_u8(&mut self) {
        self.register_h = self.read_u8(self.program_counter.wrapping_add(1));

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn daa(&mut self) {
        let mut a = self.register_a;
        let n = self.register_f.contains(FFlags::N);
        let h = self.register_f.contains(FFlags::H);
        let c = self.register_f.contains(FFlags::C);

        let mut carry_out = c;

        if !n {
            if h || (a & 0x0F) > 0x09 {
                a = a.wrapping_add(0x06);
            }
            if c || a > 0x99 {
                a = a.wrapping_add(0x60);
                carry_out = true;
            }
        } else {
            if h {
                a = a.wrapping_sub(0x06);
            }
            if c {
                a = a.wrapping_sub(0x60);
            }
        }

        self.register_a = a;

        self.register_f.set(FFlags::Z, self.register_a == 0x00);
        self.register_f.remove(FFlags::H);
        self.register_f.set(FFlags::C, carry_out);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn jr_z_i8(&mut self) {
        let z_set = self.register_f.contains(FFlags::Z);

        self.jr_cond_i8(z_set);
    }

    fn add_hl_hl(&mut self) {
        let hl = self.register_concat(self.register_h, self.register_l);

        self.register_f.remove(FFlags::N);

        let hafl_carry = (hl & 0x0FFF) + (hl & 0x0FFF) > 0x0FFF;
        self.register_f.set(FFlags::H, hafl_carry);

        let (result, carry) = hl.overflowing_add(hl);
        self.register_f.set(FFlags::C, carry);

        self.register_h = ((result >> 8) & 0x00FF) as u8;
        self.register_l = (result & 0x00FF) as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ldi_a_hl(&mut self) {
        let hl = self.register_concat(self.register_h, self.register_l);
        self.register_a = self.read_u8(hl);

        let hl_plus = hl.wrapping_add(1);

        self.register_h = (hl_plus >> 8) as u8;
        self.register_l = hl_plus as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn dec_hl(&mut self) {
        let hl = self.register_concat(self.register_h, self.register_l);
        let hl_minus = hl.wrapping_sub(1);

        self.register_h = (hl_minus >> 8) as u8;
        self.register_l = hl_minus as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_l(&mut self) {
        self.register_l = self.inc(self.register_l);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn dec_l(&mut self) {
        self.register_l = self.dec(self.register_l);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_l_u8(&mut self) {
        self.register_l = self.read_u8(self.program_counter.wrapping_add(1));

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn cpl(&mut self) {
        self.register_a = !self.register_a;

        self.register_f.insert(FFlags::N);
        self.register_f.insert(FFlags::H);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn jr_nc_i8(&mut self) {
        let c_flag = self.register_f.contains(FFlags::C);

        self.jr_cond_i8(!c_flag);
    }

    fn ld_sp_u16(&mut self) {
        let low = self.read_u8(self.program_counter.wrapping_add(1));
        let high = self.read_u8(self.program_counter.wrapping_add(2));

        self.stack_pointer = ((high as u16) << 8) | (low as u16);

        self.advance_program_counter(3);
        self.update_cycles(12);
    }

    fn ldd_hl_a(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.memory_bus.write(addr, self.register_a);

        let addr_sub = addr.wrapping_sub(1);
        self.register_h = ((addr_sub >> 8) & 0x00FF) as u8;
        self.register_l = (addr_sub & 0x00FF) as u8;
        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_sp(&mut self) {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn inc_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);

        let old_value = self.read_u8(addr);
        let result = old_value.wrapping_add(1);

        self.write_u8(addr, result);

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.remove(FFlags::N);
        self.register_f.set(FFlags::H, (old_value & 0x0F) == 0x0F);

        self.advance_program_counter(1);
        self.update_cycles(12);
    }

    fn dec_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);

        let old_value = self.read_u8(addr);
        let result = old_value.wrapping_sub(1);

        self.write_u8(addr, result);

        self.register_f.set(FFlags::Z, result == 0);
        self.register_f.insert(FFlags::N);
        self.register_f.set(FFlags::H, (old_value & 0x0F) == 0x00);

        self.advance_program_counter(1);
        self.update_cycles(12);
    }

    fn ld_hl_ptr_u8(&mut self) {
        let value = self.read_u8(self.program_counter.wrapping_add(1));
        let addr = self.register_concat(self.register_h, self.register_l);

        self.write_u8(addr, value);

        self.advance_program_counter(2);
        self.update_cycles(12);
    }

    fn scf(&mut self) {
        self.register_f.insert(FFlags::C);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn jr_c_i8(&mut self) {
        let c_flag = self.register_f.contains(FFlags::C);

        self.jr_cond_i8(c_flag);
    }

    fn add_hl_sp(&mut self) {
        let hl = self.register_concat(self.register_h, self.register_l);
        let result = self.stack_pointer.wrapping_add(hl);

        self.register_f.remove(FFlags::N);

        let half_carry = ((hl & 0x0FFF) + (self.stack_pointer & 0x0FFF)) > 0x0FFF;
        self.register_f.set(FFlags::H, half_carry);

        let carry = (hl as u32 + self.stack_pointer as u32) > 0xFFFF;
        self.register_f.set(FFlags::C, carry);

        self.register_h = (result >> 8) as u8;
        self.register_l = result as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ldd_a_hl(&mut self) {
        let hl = self.register_concat(self.register_h, self.register_l);
        self.register_a = self.read_u8(hl);

        let hl_sub = hl.wrapping_sub(1);

        self.register_h = (hl_sub >> 8) as u8;
        self.register_l = hl_sub as u8;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn dec_sp(&mut self) {
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

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

    fn ld_a_u8(&mut self) {
        self.register_a = self.read_u8(self.program_counter.wrapping_add(1));

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn ccf(&mut self) {
        let carry = self.register_f.contains(FFlags::C);
        self.register_f.set(FFlags::C, !carry);
        self.register_f.remove(FFlags::N);
        self.register_f.remove(FFlags::H);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_b_b(&mut self) {
        self.register_b = self.register_b;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_b_c(&mut self) {
        self.register_b = self.register_c;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_b_d(&mut self) {
        self.register_b = self.register_d;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_b_e(&mut self) {
        self.register_b = self.register_e;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_b_h(&mut self) {
        self.register_b = self.register_h;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_b_l(&mut self) {
        self.register_b = self.register_l;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_b_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.register_b = self.read_u8(addr);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_b_a(&mut self) {
        self.register_b = self.register_a;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_c_b(&mut self) {
        self.register_c = self.register_b;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_c_c(&mut self) {
        self.register_c = self.register_c;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_c_d(&mut self) {
        self.register_c = self.register_d;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_c_e(&mut self) {
        self.register_c = self.register_e;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_c_h(&mut self) {
        self.register_c = self.register_h;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_c_l(&mut self) {
        self.register_c = self.register_l;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_c_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.register_c = self.read_u8(addr);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_c_a(&mut self) {
        self.register_c = self.register_a;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_d_b(&mut self) {
        self.register_d = self.register_b;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_d_c(&mut self) {
        self.register_d = self.register_c;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_d_d(&mut self) {
        self.register_d = self.register_d;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_d_e(&mut self) {
        self.register_d = self.register_e;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_d_h(&mut self) {
        self.register_d = self.register_h;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_d_l(&mut self) {
        self.register_d = self.register_l;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_d_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.register_d = self.read_u8(addr);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_d_a(&mut self) {
        self.register_d = self.register_a;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_e_b(&mut self) {
        self.register_e = self.register_b;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_e_c(&mut self) {
        self.register_e = self.register_c;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_e_d(&mut self) {
        self.register_e = self.register_d;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_e_e(&mut self) {
        self.register_e = self.register_e;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_e_h(&mut self) {
        self.register_e = self.register_h;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_e_l(&mut self) {
        self.register_e = self.register_l;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_e_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.register_e = self.read_u8(addr);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_e_a(&mut self) {
        self.register_e = self.register_a;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_h_b(&mut self) {
        self.register_h = self.register_b;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_h_c(&mut self) {
        self.register_h = self.register_c;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_h_d(&mut self) {
        self.register_h = self.register_d;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_h_e(&mut self) {
        self.register_h = self.register_e;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_h_h(&mut self) {
        self.register_h = self.register_h;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_h_l(&mut self) {
        self.register_h = self.register_l;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_h_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.register_h = self.read_u8(addr);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_h_a(&mut self) {
        self.register_h = self.register_a;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_l_b(&mut self) {
        self.register_l = self.register_b;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_l_c(&mut self) {
        self.register_l = self.register_c;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_l_d(&mut self) {
        self.register_l = self.register_d;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_l_e(&mut self) {
        self.register_l = self.register_e;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_l_h(&mut self) {
        self.register_l = self.register_h;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_l_l(&mut self) {
        self.register_l = self.register_l;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_l_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.register_l = self.read_u8(addr);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_l_a(&mut self) {
        self.register_l = self.register_a;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_hl_ptr_b(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.write_u8(addr, self.register_b);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_hl_ptr_c(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.write_u8(addr, self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_hl_ptr_d(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.write_u8(addr, self.register_d);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_hl_ptr_e(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.write_u8(addr, self.register_e);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_hl_ptr_h(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.write_u8(addr, self.register_h);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_hl_ptr_l(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.write_u8(addr, self.register_l);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn halt_inst(&mut self) {
        self.halt = true;
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_hl_ptr_a(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.write_u8(addr, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_a_b(&mut self) {
        self.register_a = self.register_b;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_a_c(&mut self) {
        self.register_a = self.register_c;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_a_d(&mut self) {
        self.register_a = self.register_d;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_a_e(&mut self) {
        self.register_a = self.register_e;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_a_h(&mut self) {
        self.register_a = self.register_h;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_a_l(&mut self) {
        self.register_a = self.register_l;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ld_a_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.register_a = self.read_u8(addr);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_a_a(&mut self) {
        self.register_a = self.register_a;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn add_a_b(&mut self) {
        self.register_a = self.add(self.register_a, self.register_b);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn add_a_c(&mut self) {
        self.register_a = self.add(self.register_a, self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn add_a_d(&mut self) {
        self.register_a = self.add(self.register_a, self.register_d);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn add_a_e(&mut self) {
        self.register_a = self.add(self.register_a, self.register_e);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn add_a_h(&mut self) {
        self.register_a = self.add(self.register_a, self.register_h);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn add_a_l(&mut self) {
        self.register_a = self.add(self.register_a, self.register_l);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn add_a_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let data = self.read_u8(addr);

        self.register_a = self.add(self.register_a, data);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn add_a_a(&mut self) {
        self.register_a = self.add(self.register_a, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn adc_a_b(&mut self) {
        self.register_a = self.adc(self.register_a, self.register_b);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn adc_a_c(&mut self) {
        self.register_a = self.adc(self.register_a, self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn adc_a_d(&mut self) {
        self.register_a = self.adc(self.register_a, self.register_d);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn adc_a_e(&mut self) {
        self.register_a = self.adc(self.register_a, self.register_e);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn adc_a_h(&mut self) {
        self.register_a = self.adc(self.register_a, self.register_h);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn adc_a_l(&mut self) {
        self.register_a = self.adc(self.register_a, self.register_l);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn adc_a_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let valor = self.read_u8(addr);

        self.register_a = self.adc(self.register_a, valor);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn adc_a_a(&mut self) {
        self.register_a = self.adc(self.register_a, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sub_a_b(&mut self) {
        self.register_a = self.sub(self.register_a, self.register_b);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sub_a_c(&mut self) {
        self.register_a = self.sub(self.register_a, self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sub_a_d(&mut self) {
        self.register_a = self.sub(self.register_a, self.register_d);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sub_a_e(&mut self) {
        self.register_a = self.sub(self.register_a, self.register_e);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sub_a_h(&mut self) {
        self.register_a = self.sub(self.register_a, self.register_h);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }
    fn sub_a_l(&mut self) {
        self.register_a = self.sub(self.register_a, self.register_l);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sub_a_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let valor = self.read_u8(addr);

        self.register_a = self.sub(self.register_a, valor);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn sub_a_a(&mut self) {
        self.register_a = self.sub(self.register_a, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sbc_a_b(&mut self) {
        self.register_a = self.sbc(self.register_a, self.register_b);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sbc_a_c(&mut self) {
        self.register_a = self.sbc(self.register_a, self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sbc_a_d(&mut self) {
        self.register_a = self.sbc(self.register_a, self.register_d);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sbc_a_e(&mut self) {
        self.register_a = self.sbc(self.register_a, self.register_e);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sbc_a_h(&mut self) {
        self.register_a = self.sbc(self.register_a, self.register_h);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sbc_a_l(&mut self) {
        self.register_a = self.sbc(self.register_a, self.register_l);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn sbc_a_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let valor = self.read_u8(addr);

        self.register_a = self.sbc(self.register_a, valor);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn sbc_a_a(&mut self) {
        self.register_a = self.sbc(self.register_a, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn and_a_b(&mut self) {
        self.register_a = self.and_(self.register_a, self.register_b);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn and_a_c(&mut self) {
        self.register_a = self.and_(self.register_a, self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn and_a_d(&mut self) {
        self.register_a = self.and_(self.register_a, self.register_d);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn and_a_e(&mut self) {
        self.register_a = self.and_(self.register_a, self.register_e);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn and_a_h(&mut self) {
        self.register_a = self.and_(self.register_a, self.register_h);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn and_a_l(&mut self) {
        self.register_a = self.and_(self.register_a, self.register_l);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn and_a_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let valor = self.read_u8(addr);

        self.register_a = self.and_(self.register_a, valor);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn and_a_a(&mut self) {
        self.register_a = self.and_(self.register_a, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn xor_a_b(&mut self) {
        self.register_a = self.xor(self.register_a, self.register_b);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn xor_a_c(&mut self) {
        self.register_a = self.xor(self.register_a, self.register_c);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn xor_a_d(&mut self) {
        self.register_a = self.xor(self.register_a, self.register_d);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn xor_a_e(&mut self) {
        self.register_a = self.xor(self.register_a, self.register_e);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn xor_a_h(&mut self) {
        self.register_a = self.xor(self.register_a, self.register_h);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn xor_a_l(&mut self) {
        self.register_a = self.xor(self.register_a, self.register_l);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn xor_a_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let valor = self.read_u8(addr);
        self.register_a = self.xor(self.register_a, valor);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn xor_a_a(&mut self) {
        self.register_a = self.xor(self.register_a, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn or_a_b(&mut self) {
        self.register_a = self.or_(self.register_a, self.register_b);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn or_a_c(&mut self) {
        self.register_a = self.or_(self.register_a, self.register_c);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn or_a_d(&mut self) {
        self.register_a = self.or_(self.register_a, self.register_d);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn or_a_e(&mut self) {
        self.register_a = self.or_(self.register_a, self.register_e);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn or_a_h(&mut self) {
        self.register_a = self.or_(self.register_a, self.register_h);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn or_a_l(&mut self) {
        self.register_a = self.or_(self.register_a, self.register_l);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn or_a_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let valor = self.read_u8(addr);

        self.register_a = self.or_(self.register_a, valor);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn or_a_a(&mut self) {
        self.register_a = self.or_(self.register_a, self.register_a);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn cp_a_b(&mut self) {
        self.cp(self.register_a, self.register_b);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn cp_a_c(&mut self) {
        self.cp(self.register_a, self.register_c);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn cp_a_d(&mut self) {
        self.cp(self.register_a, self.register_d);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn cp_a_e(&mut self) {
        self.cp(self.register_a, self.register_e);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn cp_a_h(&mut self) {
        self.cp(self.register_a, self.register_h);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn cp_a_l(&mut self) {
        self.cp(self.register_a, self.register_l);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn cp_a_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let valor = self.read_u8(addr);

        self.cp(self.register_a, valor);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn cp_a_a(&mut self) {
        self.cp(self.register_a, self.register_a);
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn ret_nz(&mut self) {
        let z_set = self.register_f.contains(FFlags::Z);

        self.advance_program_counter(1);

        if !z_set {
            let low = self.pop_u8() as u16;
            let high = self.pop_u8() as u16;

            self.program_counter = (high << 8) | low;
            self.update_cycles(20);
        } else {
            self.update_cycles(8);
        }
    }

    fn pop_bc(&mut self) {
        self.register_c = self.pop_u8();
        self.register_b = self.pop_u8();

        self.advance_program_counter(1);
        self.update_cycles(12);
    }

    fn jp_nz_u16(&mut self) {
        let z_set = self.register_f.contains(FFlags::Z);

        let lower = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;

        if !z_set {
            self.program_counter = (high << 8) | lower;
            self.update_cycles(16);
        } else {
            self.update_cycles(12);
            self.advance_program_counter(3);
        }
    }

    fn jp_u16(&mut self) {
        let lower = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;

        self.program_counter = (high << 8) | lower;

        self.update_cycles(16);
    }

    fn call_nz_u16(&mut self) {
        let z_set = self.register_f.contains(FFlags::Z);

        let lower = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;
        let target = (high << 8) | lower;

        if !z_set {
            let ret = self.program_counter.wrapping_add(3);
            self.push_u16(ret);

            self.program_counter = target;
            self.update_cycles(24);
        } else {
            self.advance_program_counter(3);
            self.update_cycles(12);
        }
    }

    fn push_bc(&mut self) {
        let bc = ((self.register_b as u16) << 8) | (self.register_c as u16);
        self.push_u16(bc);

        self.advance_program_counter(1);
        self.update_cycles(16);
    }

    fn add_a_u8(&mut self) {
        self.register_f.remove(FFlags::N);
        let value = self.read_u8(self.program_counter.wrapping_add(1));

        self.register_a = self.add(self.register_a, value);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rst_00(&mut self) {
        let ret = self.program_counter.wrapping_add(1);
        self.push_u16(ret);

        self.program_counter = 0x0000;
        self.update_cycles(16);
    }

    fn ret_z(&mut self) {
        let z_set = self.register_f.contains(FFlags::Z);

        if z_set {
            let low = self.pop_u8() as u16;
            let high = self.pop_u8() as u16;
            self.program_counter = (high << 8) | low;
            self.update_cycles(20);
        } else {
            self.advance_program_counter(1);
            self.update_cycles(8);
        }
    }

    fn ret(&mut self) {
        let low = self.pop_u8() as u16;
        let high = self.pop_u8() as u16;
        self.program_counter = (high << 8) | low;
        self.update_cycles(16);
    }

    fn jp_z_u16(&mut self) {
        let z_set = self.register_f.contains(FFlags::Z);

        let lower = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;

        if z_set {
            self.program_counter = (high << 8) | lower;
            self.update_cycles(16);
        } else {
            self.advance_program_counter(3);
            self.update_cycles(12);
        }
    }

    fn cb_prefix(&mut self) {
        println!("Enter CB Prefix");

        self.advance_program_counter(1);

        let inst = self.memory_bus.read(self.program_counter);
        self.opcode = inst;

        self.process_cb(inst);
    }

    fn call_z_u16(&mut self) {
        let z_set = self.register_f.contains(FFlags::Z);

        let lower = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;
        let target = (high << 8) | lower;

        if z_set {
            let ret = self.program_counter.wrapping_add(3);
            self.push_u16(ret);

            self.program_counter = target;
            self.update_cycles(24);
        } else {
            self.advance_program_counter(3);
            self.update_cycles(12);
        }
    }

    fn call_u16(&mut self) {
        let lower = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;

        let target = (high << 8) | lower;
        let ret = self.program_counter.wrapping_add(3);

        self.push_u16(ret);
        self.program_counter = target;

        self.update_cycles(24);
    }

    fn adc_a_u8(&mut self) {
        let value = self.read_u8(self.program_counter.wrapping_add(1));

        self.register_a = self.adc(self.register_a, value);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rst_08(&mut self) {
        let ret = self.program_counter.wrapping_add(1);
        self.push_u16(ret);

        self.program_counter = 0x0008;
        self.update_cycles(16);
    }

    fn ret_nc(&mut self) {
        let c_set = self.register_f.contains(FFlags::C);

        if !c_set {
            let low = self.pop_u8() as u16;
            let high = self.pop_u8() as u16;
            self.program_counter = (high << 8) | low;
            self.update_cycles(20);
        } else {
            self.advance_program_counter(1);
            self.update_cycles(8);
        }
    }

    fn pop_de(&mut self) {
        self.register_e = self.pop_u8();
        self.register_d = self.pop_u8();

        self.advance_program_counter(1);
        self.update_cycles(12);
    }

    fn jp_nc_u16(&mut self) {
        let c_set = self.register_f.contains(FFlags::C);

        let lower = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;

        if !c_set {
            self.program_counter = (high << 8) | lower;
            self.update_cycles(16);
        } else {
            self.advance_program_counter(3);
            self.update_cycles(12);
        }
    }

    fn op_d3_unused(&mut self) {
        panic!("Unused OPCODE 0xD3");
    }

    fn call_nc_u16(&mut self) {
        let c_set = self.register_f.contains(FFlags::C);

        let low = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;
        let target = (high << 8) | low;

        if !c_set {
            let ret = self.program_counter.wrapping_add(3);
            self.push_u16(ret);

            self.program_counter = target;
            self.update_cycles(24);
        } else {
            self.advance_program_counter(3);
            self.update_cycles(12);
        }
    }

    fn push_de(&mut self) {
        let de = ((self.register_d as u16) << 8) | (self.register_e as u16);
        self.push_u16(de);

        self.advance_program_counter(1);
        self.update_cycles(16);
    }

    fn sub_u8(&mut self) {
        let valor = self.read_u8(self.program_counter.wrapping_add(1));
        self.register_a = self.sub(self.register_a, valor);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rst_10(&mut self) {
        let ret = self.program_counter.wrapping_add(1);
        self.push_u16(ret);

        self.program_counter = 0x0010;
        self.update_cycles(16);
    }

    fn ret_c(&mut self) {
        let c_set = self.register_f.contains(FFlags::C);

        if c_set {
            let low = self.pop_u8() as u16;
            let high = self.pop_u8() as u16;
            self.program_counter = (high << 8) | low;
            self.update_cycles(20);
        } else {
            self.advance_program_counter(1);
            self.update_cycles(8);
        }
    }

    fn reti(&mut self) {
        let low = self.pop_u8() as u16;
        let high = self.pop_u8() as u16;
        self.program_counter = (high << 8) | low;
        self.interruption = true;
        self.ime_pending = false;

        self.update_cycles(16);
    }

    fn jp_c_u16(&mut self) {
        let c_set = self.register_f.contains(FFlags::C);

        let lower = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;

        if c_set {
            self.program_counter = (high << 8) | lower;
            self.update_cycles(16);
        } else {
            self.advance_program_counter(3);
            self.update_cycles(12);
        }
    }

    fn op_db_unused(&mut self) {
        panic!("unused opcode 0xDB");
    }

    fn call_c_u16(&mut self) {
        let c_set = self.register_f.contains(FFlags::C);

        let low = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;
        let target = (high << 8) | low;

        if c_set {
            let ret = self.program_counter.wrapping_add(3);
            self.push_u16(ret);

            self.program_counter = target;
            self.update_cycles(24);
        } else {
            self.advance_program_counter(3);
            self.update_cycles(12);
        }
    }

    fn op_dd_unused(&mut self) {
        panic!("unused opcode 0xDD");
    }

    fn sbc_a_u8(&mut self) {
        let valor = self.read_u8(self.program_counter.wrapping_add(1));
        self.register_a = self.sbc(self.register_a, valor);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rst_18(&mut self) {
        let ret = self.program_counter.wrapping_add(1);
        self.push_u16(ret);

        self.program_counter = 0x0018;
        self.update_cycles(16);
    }

    fn ldh_u8_a(&mut self) {
        let offset = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let addr = ((0xFF << 8) as u16) | offset;

        self.write_u8(addr, self.register_a);

        self.advance_program_counter(2);
        self.update_cycles(12);
    }

    fn pop_hl(&mut self) {
        self.register_l = self.pop_u8();
        self.register_h = self.pop_u8();

        self.advance_program_counter(1);
        self.update_cycles(12);
    }

    fn ldh_c_a(&mut self) {
        let addr = ((0xFF << 8) as u16) | (self.register_c as u16);

        self.write_u8(addr, self.register_a);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn op_e3_unused(&mut self) {
        panic!("unused opcode 0xE3");
    }

    fn op_e4_unused(&mut self) {
        panic!("unused opcode 0xE4");
    }

    fn push_hl(&mut self) {
        let hl = ((self.register_h as u16) << 8) | (self.register_l as u16);
        self.push_u16(hl);

        self.advance_program_counter(1);
        self.update_cycles(16);
    }

    fn and_u8(&mut self) {
        let value = self.read_u8(self.program_counter.wrapping_add(1));
        self.register_a = self.and_(self.register_a, value);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rst_20(&mut self) {
        let ret = self.program_counter.wrapping_add(1);
        self.push_u16(ret);
        self.program_counter = 0x0020;
        self.update_cycles(16);
    }

    fn add_sp_i8(&mut self) {
        let value = self.read_u8(self.program_counter.wrapping_add(1)) as i8 as i16;

        let sp = self.stack_pointer;
        let result = sp.wrapping_add(value as u16);

        self.register_f.remove(FFlags::Z);
        self.register_f.remove(FFlags::N);

        let low_sp = sp & 0x00FF;
        let low_val = (value as u16) & 0x00FF;

        self.register_f
            .set(FFlags::H, ((low_sp & 0x000F) + (low_val & 0x000F)) > 0x000F);

        self.register_f.set(FFlags::C, (low_sp + low_val) > 0x00FF);

        self.stack_pointer = result;

        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn jp_hl(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        self.program_counter = addr;
        self.update_cycles(4);
    }

    fn ld_u16_a(&mut self) {
        let low = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;
        let addr = (high << 8) | low;

        self.write_u8(addr, self.register_a);

        self.advance_program_counter(3);
        self.update_cycles(16);
    }

    fn op_eb_unused(&mut self) {
        panic!("unused opcode 0xEB");
    }

    fn op_ec_unused(&mut self) {
        panic!("unused opcode 0xEC");
    }

    fn op_ed_unused(&mut self) {
        panic!("unused opcode 0xED");
    }

    fn xor_u8(&mut self) {
        let value = self.read_u8(self.program_counter.wrapping_add(1));
        self.register_a = self.xor(self.register_a, value);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rst_28(&mut self) {
        let ret = self.program_counter.wrapping_add(1);
        self.push_u16(ret);
        self.program_counter = 0x0028;
        self.update_cycles(16);
    }

    fn ldh_a_u8(&mut self) {
        let value = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let addr = ((0xFF << 8) as u16) | value;

        self.register_a = self.read_u8(addr);
        self.advance_program_counter(2);
        self.update_cycles(12);
    }

    fn pop_af(&mut self) {
        let low = self.pop_u8();
        let high = self.pop_u8();

        self.register_a = high;
        self.register_f = FFlags::from_bits_truncate(low & 0xF0);

        self.advance_program_counter(1);
        self.update_cycles(12);
    }

    fn ldh_a_c(&mut self) {
        let addr = self.register_concat(0xFF, self.register_c);
        self.register_a = self.read_u8(addr);

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn di(&mut self) {
        self.interruption = false;
        self.ime_pending = false;
        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn op_f4_unused(&mut self) {
        panic!("unused opcode 0xF4");
    }

    fn push_af(&mut self) {
        let f = self.register_f.bits() & 0xF0;
        let af = ((self.register_a as u16) << 8) | (f as u16);

        self.push_u16(af);

        self.advance_program_counter(1);
        self.update_cycles(16);
    }

    fn or_u8(&mut self) {
        let value = self.read_u8(self.program_counter.wrapping_add(1));
        self.register_a = self.or_(self.register_a, value);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rst_30(&mut self) {
        let ret = self.program_counter.wrapping_add(1);
        self.push_u16(ret);
        self.program_counter = 0x0030;
        self.update_cycles(16);
    }

    fn ld_hl_sp_i8(&mut self) {
        let value = self.read_u8(self.program_counter.wrapping_add(1)) as i8 as i16;

        let sp = self.stack_pointer;
        let result = sp.wrapping_add(value as u16);

        self.register_f.remove(FFlags::Z);
        self.register_f.remove(FFlags::N);

        let low_sp = sp & 0x00FF;
        let low_val = (value as u16) & 0x00FF;

        self.register_f
            .set(FFlags::H, ((low_sp & 0x000F) + (low_val & 0x000F)) > 0x000F);
        self.register_f.set(FFlags::C, (low_sp + low_val) > 0x00FF);

        self.register_h = (result >> 8) as u8;
        self.register_l = result as u8;

        self.advance_program_counter(2);
        self.update_cycles(12);
    }

    fn ld_sp_hl(&mut self) {
        let hl = self.register_concat(self.register_h, self.register_l);
        self.stack_pointer = hl;

        self.advance_program_counter(1);
        self.update_cycles(8);
    }

    fn ld_a_u16(&mut self) {
        let low = self.read_u8(self.program_counter.wrapping_add(1)) as u16;
        let high = self.read_u8(self.program_counter.wrapping_add(2)) as u16;
        let addr = (high << 8) | low;

        self.register_a = self.read_u8(addr);

        self.advance_program_counter(3);
        self.update_cycles(16);
    }

    fn ei(&mut self) {
        self.ime_pending = true;

        self.advance_program_counter(1);
        self.update_cycles(4);
    }

    fn op_fc_unused(&mut self) {
        panic!("unused opcode 0xFC");
    }

    fn op_fd_unused(&mut self) {
        panic!("unused opcode 0xFD");
    }

    fn cp_u8(&mut self) {
        let value = self.read_u8(self.program_counter.wrapping_add(1));

        self.cp(self.register_a, value);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rst_38(&mut self) {
        let ret = self.program_counter.wrapping_add(1);
        self.push_u16(ret);

        self.program_counter = 0x0038;
        self.update_cycles(16);
    }

    fn rlc_b(&mut self) {
        self.register_b = self.rlc(self.register_b);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rlc_c(&mut self) {
        self.register_c = self.rlc(self.register_c);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rlc_d(&mut self) {
        self.register_d = self.rlc(self.register_d);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rlc_e(&mut self) {
        self.register_e = self.rlc(self.register_e);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rlc_h(&mut self) {
        self.register_h = self.rlc(self.register_h);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rlc_l(&mut self) {
        self.register_l = self.rlc(self.register_l);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rlc_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let result = self.rlc(value);

        self.write_u8(addr, result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn rlc_a(&mut self) {
        self.register_a = self.rlc(self.register_a);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rrc_b(&mut self) {
        self.register_b = self.rrc(self.register_b);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rrc_c(&mut self) {
        self.register_c = self.rrc(self.register_c);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rrc_d(&mut self) {
        self.register_d = self.rrc(self.register_d);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rrc_e(&mut self) {
        self.register_e = self.rrc(self.register_e);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rrc_h(&mut self) {
        self.register_h = self.rrc(self.register_h);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rrc_l(&mut self) {
        self.register_l = self.rrc(self.register_l);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rrc_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let result = self.rrc(value);

        self.write_u8(addr, result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn rrc_a(&mut self) {
        self.register_a = self.rrc(self.register_a);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rl_b(&mut self) {
        self.register_b = self.rl(self.register_b);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rl_c(&mut self) {
        self.register_c = self.rl(self.register_c);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rl_d(&mut self) {
        self.register_d = self.rl(self.register_d);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rl_e(&mut self) {
        self.register_e = self.rl(self.register_e);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rl_h(&mut self) {
        self.register_h = self.rl(self.register_h);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rl_l(&mut self) {
        self.register_l = self.rl(self.register_l);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rl_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let result = self.rl(value);

        self.write_u8(addr, result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn rl_a(&mut self) {
        self.register_a = self.rl(self.register_a);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rr_b(&mut self) {
        self.register_b = self.rr(self.register_b);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rr_c(&mut self) {
        self.register_c = self.rr(self.register_c);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rr_d(&mut self) {
        self.register_d = self.rr(self.register_d);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rr_e(&mut self) {
        self.register_e = self.rr(self.register_e);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rr_h(&mut self) {
        self.register_h = self.rr(self.register_h);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rr_l(&mut self) {
        self.register_l = self.rr(self.register_l);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn rr_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let result = self.rr(value);

        self.write_u8(addr, result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn rr_a(&mut self) {
        self.register_a = self.rr(self.register_a);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sla_b(&mut self) {
        self.register_b = self.sla(self.register_b);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sla_c(&mut self) {
        self.register_c = self.sla(self.register_c);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sla_d(&mut self) {
        self.register_d = self.sla(self.register_d);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sla_e(&mut self) {
        self.register_e = self.sla(self.register_e);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sla_h(&mut self) {
        self.register_h = self.sla(self.register_h);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sla_l(&mut self) {
        self.register_l = self.sla(self.register_l);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sla_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let result = self.sla(value);

        self.write_u8(addr, result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn sla_a(&mut self) {
        self.register_a = self.sla(self.register_a);

        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sra_b(&mut self) {
        self.register_b = self.sra(self.register_b);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sra_c(&mut self) {
        self.register_c = self.sra(self.register_c);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sra_d(&mut self) {
        self.register_d = self.sra(self.register_d);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sra_e(&mut self) {
        self.register_e = self.sra(self.register_e);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sra_h(&mut self) {
        self.register_h = self.sra(self.register_h);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sra_l(&mut self) {
        self.register_l = self.sra(self.register_l);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn sra_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let result = self.sra(value);

        self.write_u8(addr, result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn sra_a(&mut self) {
        self.register_a = self.sra(self.register_a);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn swap_b(&mut self) {
        self.register_b = self.swap(self.register_b);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn swap_c(&mut self) {
        self.register_c = self.swap(self.register_c);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn swap_d(&mut self) {
        self.register_d = self.swap(self.register_d);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn swap_e(&mut self) {
        self.register_e = self.swap(self.register_e);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn swap_h(&mut self) {
        self.register_h = self.swap(self.register_h);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn swap_l(&mut self) {
        self.register_l = self.swap(self.register_l);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn swap_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let result = self.swap(value);

        self.write_u8(addr, result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn swap_a(&mut self) {
        self.register_a = self.swap(self.register_a);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn srl_b(&mut self) {
        self.register_b = self.srl(self.register_b);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn srl_c(&mut self) {
        self.register_c = self.srl(self.register_c);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn srl_d(&mut self) {
        self.register_d = self.srl(self.register_d);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn srl_e(&mut self) {
        self.register_e = self.srl(self.register_e);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn srl_h(&mut self) {
        self.register_h = self.srl(self.register_h);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn srl_l(&mut self) {
        self.register_l = self.srl(self.register_l);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn srl_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let result = self.srl(value);

        self.write_u8(addr, result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn srl_a(&mut self) {
        self.register_a = self.srl(self.register_a);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_0_b(&mut self) {
        self.bit(self.register_b, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_0_c(&mut self) {
        self.bit(self.register_c, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_0_d(&mut self) {
        self.bit(self.register_d, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_0_e(&mut self) {
        self.bit(self.register_e, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_0_h(&mut self) {
        self.bit(self.register_h, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_0_l(&mut self) {
        self.bit(self.register_l, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_0_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        self.bit(value, 0);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn bit_0_a(&mut self) {
        self.bit(self.register_a, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_1_b(&mut self) {
        self.bit(self.register_b, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_1_c(&mut self) {
        self.bit(self.register_c, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_1_d(&mut self) {
        self.bit(self.register_d, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_1_e(&mut self) {
        self.bit(self.register_e, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_1_h(&mut self) {
        self.bit(self.register_h, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_1_l(&mut self) {
        self.bit(self.register_l, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_1_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        self.bit(value, 1);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn bit_1_a(&mut self) {
        self.bit(self.register_a, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_2_b(&mut self) {
        self.bit(self.register_b, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_2_c(&mut self) {
        self.bit(self.register_c, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_2_d(&mut self) {
        self.bit(self.register_d, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_2_e(&mut self) {
        self.bit(self.register_e, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_2_h(&mut self) {
        self.bit(self.register_h, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_2_l(&mut self) {
        self.bit(self.register_l, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_2_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        self.bit(value, 2);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn bit_2_a(&mut self) {
        self.bit(self.register_a, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_3_b(&mut self) {
        self.bit(self.register_b, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_3_c(&mut self) {
        self.bit(self.register_c, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_3_d(&mut self) {
        self.bit(self.register_d, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_3_e(&mut self) {
        self.bit(self.register_e, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_3_h(&mut self) {
        self.bit(self.register_h, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_3_l(&mut self) {
        self.bit(self.register_l, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_3_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        self.bit(value, 3);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn bit_3_a(&mut self) {
        self.bit(self.register_a, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_4_b(&mut self) {
        self.bit(self.register_b, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_4_c(&mut self) {
        self.bit(self.register_c, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_4_d(&mut self) {
        self.bit(self.register_d, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_4_e(&mut self) {
        self.bit(self.register_e, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_4_h(&mut self) {
        self.bit(self.register_h, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_4_l(&mut self) {
        self.bit(self.register_l, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_4_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        self.bit(value, 4);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn bit_4_a(&mut self) {
        self.bit(self.register_a, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_5_b(&mut self) {
        self.bit(self.register_b, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_5_c(&mut self) {
        self.bit(self.register_c, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_5_d(&mut self) {
        self.bit(self.register_d, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_5_e(&mut self) {
        self.bit(self.register_e, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_5_h(&mut self) {
        self.bit(self.register_h, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_5_l(&mut self) {
        self.bit(self.register_l, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_5_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        self.bit(value, 5);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn bit_5_a(&mut self) {
        self.bit(self.register_a, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_6_b(&mut self) {
        self.bit(self.register_b, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_6_c(&mut self) {
        self.bit(self.register_c, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_6_d(&mut self) {
        self.bit(self.register_d, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_6_e(&mut self) {
        self.bit(self.register_e, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_6_h(&mut self) {
        self.bit(self.register_h, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_6_l(&mut self) {
        self.bit(self.register_l, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_6_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        self.bit(value, 6);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn bit_6_a(&mut self) {
        self.bit(self.register_a, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_7_b(&mut self) {
        self.bit(self.register_b, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_7_c(&mut self) {
        self.bit(self.register_c, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_7_d(&mut self) {
        self.bit(self.register_d, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_7_e(&mut self) {
        self.bit(self.register_e, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_7_h(&mut self) {
        self.bit(self.register_h, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_7_l(&mut self) {
        self.bit(self.register_l, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn bit_7_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        self.bit(value, 7);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn bit_7_a(&mut self) {
        self.bit(self.register_a, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_0_b(&mut self) {
        self.register_b = self.res(self.register_b, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_0_c(&mut self) {
        self.register_c = self.res(self.register_c, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_0_d(&mut self) {
        self.register_d = self.res(self.register_d, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_0_e(&mut self) {
        self.register_e = self.res(self.register_e, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_0_h(&mut self) {
        self.register_h = self.res(self.register_h, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_0_l(&mut self) {
        self.register_l = self.res(self.register_l, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_0_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.res(value, 0);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn res_0_a(&mut self) {
        self.register_a = self.res(self.register_a, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_1_b(&mut self) {
        self.register_b = self.res(self.register_b, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_1_c(&mut self) {
        self.register_c = self.res(self.register_c, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_1_d(&mut self) {
        self.register_d = self.res(self.register_d, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_1_e(&mut self) {
        self.register_e = self.res(self.register_e, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_1_h(&mut self) {
        self.register_h = self.res(self.register_h, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_1_l(&mut self) {
        self.register_l = self.res(self.register_l, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_1_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.res(value, 1);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn res_1_a(&mut self) {
        self.register_a = self.res(self.register_a, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_2_b(&mut self) {
        self.register_b = self.res(self.register_b, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_2_c(&mut self) {
        self.register_c = self.res(self.register_c, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_2_d(&mut self) {
        self.register_d = self.res(self.register_d, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_2_e(&mut self) {
        self.register_e = self.res(self.register_e, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_2_h(&mut self) {
        self.register_h = self.res(self.register_h, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_2_l(&mut self) {
        self.register_l = self.res(self.register_l, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_2_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.res(value, 2);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn res_2_a(&mut self) {
        self.register_a = self.res(self.register_a, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_3_b(&mut self) {
        self.register_b = self.res(self.register_b, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_3_c(&mut self) {
        self.register_c = self.res(self.register_c, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_3_d(&mut self) {
        self.register_d = self.res(self.register_d, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_3_e(&mut self) {
        self.register_e = self.res(self.register_e, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_3_h(&mut self) {
        self.register_h = self.res(self.register_h, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_3_l(&mut self) {
        self.register_l = self.res(self.register_l, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_3_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.res(value, 3);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn res_3_a(&mut self) {
        self.register_a = self.res(self.register_a, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_4_b(&mut self) {
        self.register_b = self.res(self.register_b, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_4_c(&mut self) {
        self.register_c = self.res(self.register_c, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_4_d(&mut self) {
        self.register_d = self.res(self.register_d, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_4_e(&mut self) {
        self.register_e = self.res(self.register_e, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_4_h(&mut self) {
        self.register_h = self.res(self.register_h, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_4_l(&mut self) {
        self.register_l = self.res(self.register_l, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_4_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.res(value, 4);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn res_4_a(&mut self) {
        self.register_a = self.res(self.register_a, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_5_b(&mut self) {
        self.register_b = self.res(self.register_b, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_5_c(&mut self) {
        self.register_c = self.res(self.register_c, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_5_d(&mut self) {
        self.register_d = self.res(self.register_d, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_5_e(&mut self) {
        self.register_e = self.res(self.register_e, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_5_h(&mut self) {
        self.register_h = self.res(self.register_h, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_5_l(&mut self) {
        self.register_l = self.res(self.register_l, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_5_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.res(value, 5);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn res_5_a(&mut self) {
        self.register_a = self.res(self.register_a, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_6_b(&mut self) {
        self.register_b = self.res(self.register_b, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_6_c(&mut self) {
        self.register_c = self.res(self.register_c, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_6_d(&mut self) {
        self.register_d = self.res(self.register_d, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_6_e(&mut self) {
        self.register_e = self.res(self.register_e, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_6_h(&mut self) {
        self.register_h = self.res(self.register_h, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_6_l(&mut self) {
        self.register_l = self.res(self.register_l, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_6_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.res(value, 6);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn res_6_a(&mut self) {
        self.register_a = self.res(self.register_a, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_7_b(&mut self) {
        self.register_b = self.res(self.register_b, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_7_c(&mut self) {
        self.register_c = self.res(self.register_c, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_7_d(&mut self) {
        self.register_d = self.res(self.register_d, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_7_e(&mut self) {
        self.register_e = self.res(self.register_e, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_7_h(&mut self) {
        self.register_h = self.res(self.register_h, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_7_l(&mut self) {
        self.register_l = self.res(self.register_l, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn res_7_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.res(value, 7);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn res_7_a(&mut self) {
        self.register_a = self.res(self.register_a, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_0_b(&mut self) {
        self.register_b = self.set(self.register_b, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_0_c(&mut self) {
        self.register_c = self.set(self.register_c, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }
    fn set_0_d(&mut self) {
        self.register_d = self.set(self.register_d, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_0_e(&mut self) {
        self.register_e = self.set(self.register_e, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_0_h(&mut self) {
        self.register_h = self.set(self.register_h, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_0_l(&mut self) {
        self.register_l = self.set(self.register_l, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_0_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.set(value, 0);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn set_0_a(&mut self) {
        self.register_a = self.set(self.register_a, 0);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_1_b(&mut self) {
        self.register_b = self.set(self.register_b, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_1_c(&mut self) {
        self.register_c = self.set(self.register_c, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_1_d(&mut self) {
        self.register_d = self.set(self.register_d, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_1_e(&mut self) {
        self.register_e = self.set(self.register_e, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_1_h(&mut self) {
        self.register_h = self.set(self.register_h, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_1_l(&mut self) {
        self.register_l = self.set(self.register_l, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_1_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.set(value, 1);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn set_1_a(&mut self) {
        self.register_a = self.set(self.register_a, 1);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_2_b(&mut self) {
        self.register_b = self.set(self.register_b, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_2_c(&mut self) {
        self.register_c = self.set(self.register_c, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_2_d(&mut self) {
        self.register_d = self.set(self.register_d, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_2_e(&mut self) {
        self.register_e = self.set(self.register_e, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_2_h(&mut self) {
        self.register_h = self.set(self.register_h, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_2_l(&mut self) {
        self.register_l = self.set(self.register_l, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_2_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.set(value, 2);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn set_2_a(&mut self) {
        self.register_a = self.set(self.register_a, 2);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_3_b(&mut self) {
        self.register_b = self.set(self.register_b, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_3_c(&mut self) {
        self.register_c = self.set(self.register_c, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_3_d(&mut self) {
        self.register_d = self.set(self.register_d, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_3_e(&mut self) {
        self.register_e = self.set(self.register_e, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_3_h(&mut self) {
        self.register_h = self.set(self.register_h, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_3_l(&mut self) {
        self.register_l = self.set(self.register_l, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_3_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.set(value, 3);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn set_3_a(&mut self) {
        self.register_a = self.set(self.register_a, 3);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_4_b(&mut self) {
        self.register_b = self.set(self.register_b, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_4_c(&mut self) {
        self.register_c = self.set(self.register_c, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_4_d(&mut self) {
        self.register_d = self.set(self.register_d, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_4_e(&mut self) {
        self.register_e = self.set(self.register_e, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_4_h(&mut self) {
        self.register_h = self.set(self.register_h, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_4_l(&mut self) {
        self.register_l = self.set(self.register_l, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_4_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.set(value, 4);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn set_4_a(&mut self) {
        self.register_a = self.set(self.register_a, 4);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_5_b(&mut self) {
        self.register_b = self.set(self.register_b, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_5_c(&mut self) {
        self.register_c = self.set(self.register_c, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_5_d(&mut self) {
        self.register_d = self.set(self.register_d, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_5_e(&mut self) {
        self.register_e = self.set(self.register_e, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_5_h(&mut self) {
        self.register_h = self.set(self.register_h, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_5_l(&mut self) {
        self.register_l = self.set(self.register_l, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_5_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.set(value, 5);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn set_5_a(&mut self) {
        self.register_a = self.set(self.register_a, 5);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_6_b(&mut self) {
        self.register_b = self.set(self.register_b, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_6_c(&mut self) {
        self.register_c = self.set(self.register_c, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_6_d(&mut self) {
        self.register_d = self.set(self.register_d, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_6_e(&mut self) {
        self.register_e = self.set(self.register_e, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_6_h(&mut self) {
        self.register_h = self.set(self.register_h, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_6_l(&mut self) {
        self.register_l = self.set(self.register_l, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_6_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.set(value, 6);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn set_6_a(&mut self) {
        self.register_a = self.set(self.register_a, 6);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_7_b(&mut self) {
        self.register_b = self.set(self.register_b, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_7_c(&mut self) {
        self.register_c = self.set(self.register_c, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_7_d(&mut self) {
        self.register_d = self.set(self.register_d, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_7_e(&mut self) {
        self.register_e = self.set(self.register_e, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_7_h(&mut self) {
        self.register_h = self.set(self.register_h, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_7_l(&mut self) {
        self.register_l = self.set(self.register_l, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }

    fn set_7_hl_ptr(&mut self) {
        let addr = self.register_concat(self.register_h, self.register_l);
        let value = self.read_u8(addr);
        let res_result = self.set(value, 7);
        self.write_u8(addr, res_result);
        self.advance_program_counter(2);
        self.update_cycles(16);
    }

    fn set_7_a(&mut self) {
        self.register_a = self.set(self.register_a, 7);
        self.advance_program_counter(2);
        self.update_cycles(8);
    }
}
