#![feature(test)]

#![allow(unused_variables)]

use test::Bencher;
use std::{fmt, mem, u8};
use std::process::exit;
use std::num::Wrapping;
use time;

pub struct CPU {
    pub ip: u16,
    pub instruction_count: usize,
    memory: Vec<u8>,
    pub r16: [Register16; 8], // general purpose registers
    pub sreg16: [u16; 6], // segment registers
    flags: Flags,
    breakpoints: Vec<usize>,
    gpu: GPU,
    rom_base: usize,
    pub fatal_error: bool, // for debugging: signals to debugger we hit an error
}

struct GPU {
    scanline: u16,
}
impl GPU {
    fn new() -> GPU {
        GPU { scanline: 0 }
    }
    fn progress_scanline(&mut self) {
        // HACK to have a source of info to toggle CGA status register
        self.scanline += 1;
        if self.scanline > 100 {
            self.scanline = 0;
        }
    }
}

// https://en.wikipedia.org/wiki/FLAGS_register
struct Flags {
    carry: bool, // 0: carry flag
    reserved1: bool, // 1: Reserved, always 1 in EFLAGS
    parity: bool, // 2: parity flag
    reserved3: bool,
    auxiliary_carry: bool, // 4: auxiliary carry flag
    reserved5: bool,
    zero: bool, // 6: zero flag
    sign: bool, // 7: sign flag
    trap: bool, // 8: trap flag (single step)
    interrupt: bool, // 9: interrupt flag
    direction: bool, // 10: direction flag (control with cld, std)
    overflow: bool, // 11: overflow flag
    iopl12: bool, // 12: I/O privilege level (286+ only), always 1 on 8086 and 186
    iopl13: bool, // 13 --""---
    nested_task: bool, // 14: Nested task flag (286+ only), always 1 on 8086 and 186
    reserved15: bool, // 15: Reserved, always 1 on 8086 and 186, always 0 on later models
}

impl Flags {
    fn set_sign_u8(&mut self, v: usize) {
        // Set equal to the most-significant bit of the result,
        // which is the sign bit of a signed integer.
        // (0 indicates a positive value and 1 indicates a negative value.)
        self.sign = v & 0x80 != 0;
    }
    fn set_sign_u16(&mut self, v: usize) {
        self.sign = v & 0x8000 != 0;
    }
    fn set_parity(&mut self, v: usize) {
        // Set if the least-significant byte of the result contains an
        // even number of 1 bits; cleared otherwise.
        self.parity = v & 1 == 0;
    }
    fn set_zero_u8(&mut self, v: usize) {
        // Zero flag — Set if the result is zero; cleared otherwise.
        self.zero = (v & 0xFF) == 0;
    }
    fn set_zero_u16(&mut self, v: usize) {
        self.zero = (v & 0xFFFF) == 0;
    }
    fn set_auxiliary(&mut self, res: usize, v1: usize, v2: usize) {
        // Set if an arithmetic operation generates a carry or a borrow out
        // of bit 3 of the result; cleared otherwise. This flag is used in
        // binary-coded decimal (BCD) arithmetic.
        self.auxiliary_carry = (res ^ (v1 ^ v2)) & 0x10 != 0;
    }
    fn set_overflow_add_u8(&mut self, res: usize, v1: usize, v2: usize) {
        // Set if the integer result is too large a positive number or too
        // small a negative number (excluding the sign-bit) to fit in the
        // destination operand; cleared otherwise. This flag indicates an
        // overflow condition for signed-integer (two’s complement) arithmetic.
        self.overflow = (res ^ v1) & (res ^ v2) & 0x80 != 0;
    }
    fn set_overflow_add_u16(&mut self, res: usize, v1: usize, v2: usize) {
        self.overflow = (res ^ v1) & (res ^ v2) & 0x8000 != 0;
    }
    fn set_overflow_sub_u8(&mut self, res: usize, v1: usize, v2: usize) {
        self.overflow = (v2 ^ v1) & (v2 ^ res) & 0x80 != 0;
    }
    fn set_overflow_sub_u16(&mut self, res: usize, v1: usize, v2: usize) {
        self.overflow = (v2 ^ v1) & (v2 ^ res) & 0x8000 != 0;
    }
    fn set_carry_u8(&mut self, res: usize, v1: usize, v2: usize) {
        // Set if an arithmetic operation generates a carry or a borrow out of
        // the most-significant bit of the result; cleared otherwise. This flag
        // indicates an overflow condition for unsigned-integer arithmetic.
        self.carry = res & 0x100 != 0;
    }
    fn set_carry_u16(&mut self, res: usize, v1: usize, v2: usize) {
        self.carry = res & 0x10000 != 0;
    }
    // returns the FLAGS register
    fn u16(&self) -> u16 {
        let mut val = 0 as u16;

        if self.carry {
            val |= 1;
        }
        if self.parity {
            val |= 1 << 2;
        }
        if self.auxiliary_carry {
            val |= 1 << 4;
        }
        if self.zero {
            val |= 1 << 6;
        }
        if self.sign {
            val |= 1 << 7;
        }
        if self.trap {
            val |= 1 << 8;
        }
        if self.interrupt {
            val |= 1 << 9;
        }
        if self.direction {
            val |= 1 << 10;
        }
        if self.overflow {
            val |= 1 << 11;
        }
        if self.iopl12 {
            val |= 1 << 12;
        }
        if self.iopl13 {
            val |= 1 << 13;
        }
        if self.nested_task {
            val |= 1 << 14;
        }
        val |= 1 << 15; // always 1 on 8086 and 186, always 0 on later models

        val
    }
}


#[derive(Copy, Clone)]
pub struct Register16 {
    pub val: u16,
}

impl Register16 {
    fn set_hi(&mut self, val: u8) {
        self.val = (self.val & 0xFF) + ((val as u16) << 8);
    }
    fn set_lo(&mut self, val: u8) {
        self.val = (self.val & 0xFF00) + val as u16;
    }
    fn set_u16(&mut self, val: u16) {
        self.val = val;
    }
    fn lo_u8(&mut self) -> u8 {
        (self.val & 0xFF) as u8
    }
    fn hi_u8(&mut self) -> u8 {
        (self.val >> 8) as u8
    }
    fn u16(&self) -> u16 {
        self.val
    }
}


fn right_pad(s: &str, len: usize) -> String {
    let mut res = String::new();
    res.push_str(s);
    if s.len() < len {
        let padding_len = len - s.len();
        for _ in 0..padding_len {
            res.push_str(" ");
        }
    }
    res
}

struct ModRegRm {
    md: u8, // NOTE: "mod" is reserved in rust
    reg: u8,
    rm: u8,
}

#[derive(Debug)]
pub struct Instruction {
    pub command: Op,
    segment: Segment,
    params: ParameterPair,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.params.dst {
            Parameter::None() => write!(f, "{:?}", self.command),
            _ => {
                let cmd = right_pad(&format!("{:?}", self.command), 9);
                match self.params.src {
                    Parameter::None() => write!(f, "{}{}", cmd, self.params.dst),
                    _ => write!(f, "{}{}, {}", cmd, self.params.dst, self.params.src),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ParameterPair {
    src: Parameter,
    dst: Parameter,
}

#[derive(Debug)]
enum Parameter {
    Imm8(u8),
    Imm16(u16),
    ImmS8(i8), // byte +0x3f
    Ptr8(Segment, u16), // byte [u16]
    Ptr16(Segment, u16), // word [u16]
    Ptr8Amode(Segment, usize), // byte [amode], like "byte [bp+si]"
    Ptr8AmodeS8(Segment, usize, i8), // byte [amode+s8], like "byte [bp-0x20]"
    Ptr8AmodeS16(Segment, usize, i16), // byte [amode+s16], like "byte [bp-0x2020]"
    Ptr16Amode(Segment, usize), // word [amode], like "word [bx]"
    Ptr16AmodeS8(Segment, usize, i8), // word [amode+s8], like "word [bp-0x20]"
    Ptr16AmodeS16(Segment, usize, i16), // word [amode+s16], like "word [bp-0x2020]"
    Reg8(usize), // index into the low 4 of CPU.r16
    Reg16(usize), // index into CPU.r16
    SReg16(usize), // index into cpu.sreg16
    None(),
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Parameter::Imm8(imm) => write!(f, "0x{:02X}", imm),
            Parameter::Imm16(imm) => write!(f, "0x{:04X}", imm),
            Parameter::ImmS8(imm) => {
                write!(f,
                       "byte {}0x{:02X}",
                       if imm < 0 { "-" } else { "+" },
                       if imm < 0 { -imm } else { imm })
            }
            Parameter::Ptr8(seg, v) => write!(f, "byte [{}0x{:04X}]", seg, v),
            Parameter::Ptr16(seg, v) => write!(f, "word [{}0x{:04X}]", seg, v),
            Parameter::Ptr8Amode(seg, v) => write!(f, "byte [{}{}]", seg, amode(v as u8)),
            Parameter::Ptr8AmodeS8(seg, v, imm) => {
                write!(f,
                       "byte [{}{}{}0x{:02X}]",
                       seg,
                       amode(v as u8),
                       if imm < 0 { "-" } else { "+" },
                       if imm < 0 { -imm } else { imm })
            }
            Parameter::Ptr8AmodeS16(seg, v, imm) => {
                write!(f,
                       "byte [{}{}{}0x{:04X}]",
                       seg,
                       amode(v as u8),
                       if imm < 0 { "-" } else { "+" },
                       if imm < 0 { -imm } else { imm })
            }
            Parameter::Ptr16Amode(seg, v) => write!(f, "word [{}{}]", seg, amode(v as u8)),
            Parameter::Ptr16AmodeS8(seg, v, imm) => {
                write!(f,
                       "word [{}{}{}0x{:02X}]",
                       seg,
                       amode(v as u8),
                       if imm < 0 { "-" } else { "+" },
                       if imm < 0 { -imm } else { imm })
            }
            Parameter::Ptr16AmodeS16(seg, v, imm) => {
                write!(f,
                       "word [{}{}{}0x{:04X}]",
                       seg,
                       amode(v as u8),
                       if imm < 0 { "-" } else { "+" },
                       if imm < 0 { -imm } else { imm })
            }
            Parameter::Reg8(v) => write!(f, "{}", r8(v as u8)),
            Parameter::Reg16(v) => write!(f, "{}", r16(v as u8)),
            Parameter::SReg16(v) => write!(f, "{}", sr16(v as u8)),
            Parameter::None() => write!(f, ""),
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Segment {
    CS(),
    DS(),
    ES(),
    SS(),
    Default(), // is treated as CS
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Segment::CS() => write!(f, "cs:"),
            Segment::DS() => write!(f, "ds:"),
            Segment::ES() => write!(f, "es:"),
            Segment::SS() => write!(f, "ss:"),
            Segment::Default() => write!(f, ""),
        }
    }
}

#[derive(Debug)]
pub enum Op {
    Add8(),
    Add16(),
    And8(),
    And16(),
    CallNear(),
    Cbw(),
    Clc(),
    Cld(),
    Cli(),
    Cmp8(),
    Cmp16(),
    Daa(),
    Dec8(),
    Dec16(),
    Div8(),
    Div16(),
    Hlt(),
    In8(),
    Inc8(),
    Inc16(),
    Int(),
    Ja(),
    Jc(),
    Jg(),
    JmpNear(),
    JmpShort(),
    Jna(),
    Jnc(),
    Jnl(),
    Jnz(),
    Js(),
    Jz(),
    Lea16(),
    Lodsb(),
    Lodsw(),
    Loop(),
    Mov8(),
    Mov16(),
    Mul8(),
    Nop(),
    Or8(),
    Or16(),
    Out8(),
    Pop16(),
    Push16(),
    Pushf(),
    Rcl8(),
    Rcr8(),
    RepMovsb(),
    RepMovsw(),
    Retf(),
    Retn(),
    Ror8(),
    Ror16(),
    Sahf(),
    Shl8(),
    Shl16(),
    Shr8(),
    Shr16(),
    Sti(),
    Stosb(),
    Sub8(),
    Sub16(),
    Test8(),
    Test16(),
    Xchg8(),
    Xchg16(),
    Xor8(),
    Xor16(),
    Unknown(),
}

#[derive(Debug)]
pub struct InstructionInfo {
    pub segment: usize,
    pub offset: usize,
    pub length: usize,
    pub text: String,
    pub bytes: Vec<u8>,
    pub instruction: Instruction,
}

impl InstructionInfo {
    pub fn pretty_string(&self) -> String {
        let hex = self.to_hex_string(&self.bytes);
        format!("[{:04X}:{:04X}] {} {}",
                self.segment,
                self.offset,
                right_pad(&hex, 10),
                self.text)
    }
    fn to_hex_string(&self, bytes: &[u8]) -> String {
        let strs: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
        strs.join("")
    }
}

// r8 (4 low of r16)
const AL: usize = 0;
const CL: usize = 1;
const DL: usize = 2;
const BL: usize = 3;
const AH: usize = 4;
const CH: usize = 5;
const DH: usize = 6;
const BH: usize = 7;

// r16
const AX: usize = 0;
const CX: usize = 1;
const DX: usize = 2;
const BX: usize = 3;
const SP: usize = 4;
const BP: usize = 5;
const SI: usize = 6;
const DI: usize = 7;

// sreg16
const ES: usize = 0;
pub const CS: usize = 1;
const SS: usize = 2;
const DS: usize = 3;
const FS: usize = 4;
const GS: usize = 5;

impl CPU {
    pub fn new() -> CPU {
        let mut cpu = CPU {
            ip: 0,
            instruction_count: 0,
            memory: vec![0u8; 0x10000 * 64],
            r16: [Register16 { val: 0 }; 8],
            sreg16: [0; 6],
            flags: Flags {
                carry: false,
                reserved1: false,
                parity: false,
                reserved3: false,
                auxiliary_carry: false,
                reserved5: false,
                zero: false,
                sign: false,
                trap: false,
                interrupt: false,
                direction: false,
                overflow: false,
                iopl12: false,
                iopl13: false,
                nested_task: false,
                reserved15: false,
            },
            breakpoints: vec![0; 0],
            gpu: GPU::new(),
            rom_base: 0,
            fatal_error: false,
        };

        // intializes the cpu as if to run .com programs, info from
        // http://www.delorie.com/djgpp/doc/rbinter/id/51/29.html

        // offset of last word available in first 64k segment
        cpu.r16[SP].val = 0xFFFE;

        cpu
    }

    pub fn add_breakpoint(&mut self, bp: usize) {
        self.breakpoints.push(bp);
    }

    pub fn get_breakpoints(&self) -> Vec<usize> {
        self.breakpoints.clone()
    }

    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    pub fn reset(&mut self) {
        self.ip = 0;
        self.instruction_count = 0;
        // XXX clear memory
    }

    pub fn load_bios(&mut self, data: &[u8]) {
        self.sreg16[CS] = 0xF000;
        self.ip = 0x0000;
        let min = 0xF0000;
        let max = min + data.len();
        println!("loading bios to {:06X}..{:06X}", min, max);
        self.rom_base = min;

        let mut rom_pos = 0;
        for i in min..max {
            self.memory[i] = data[rom_pos];
            rom_pos += 1;
        }
    }


    // load .com program into CS:0100 and set IP to program start
    pub fn load_com(&mut self, data: &[u8]) {
        // CS,DS,ES,SS = PSP segment
        let psp_segment = 0x085F; // is what dosbox used
        self.sreg16[CS] = psp_segment;
        self.sreg16[DS] = psp_segment;
        self.sreg16[ES] = psp_segment;
        self.sreg16[SS] = psp_segment;

        self.ip = 0x100;
        let min = self.get_offset();
        let max = min + data.len();
        println!("loading rom to {:06X}..{:06X}", min, max);
        self.rom_base = min;

        let mut rom_pos = 0;
        for i in min..max {
            self.memory[i] = data[rom_pos];
            rom_pos += 1;
        }
    }

    // base address the rom was loaded to
    pub fn get_rom_base(&self) -> usize {
        self.rom_base
    }

    pub fn print_registers(&mut self) {
        print!("ip:{:04X}  ax:{:04X} bx:{:04X} cx:{:04X} dx:{:04X}",
               self.ip,
               self.r16[AX].val,
               self.r16[BX].val,
               self.r16[CX].val,
               self.r16[DX].val);
        print!("  sp:{:04X} bp:{:04X} si:{:04X} di:{:04X}",
               self.r16[SP].val,
               self.r16[BP].val,
               self.r16[SI].val,
               self.r16[DI].val);

        print!("   es:{:04X} cs:{:04X} ss:{:04X} ds:{:04X} fs:{:04X} gs:{:04X}",
               self.sreg16[ES],
               self.sreg16[CS],
               self.sreg16[SS],
               self.sreg16[DS],
               self.sreg16[FS],
               self.sreg16[GS]);

        println!("");
    }

    pub fn execute_instruction(&mut self) {
        let op = self.decode_instruction(Segment::CS());
        if self.fatal_error {
            println!("XXX fatal error occured");
            return;
        }
        self.execute(&op);
        self.gpu.progress_scanline();
    }

    pub fn disassemble_block(&mut self, origin: u16, count: usize) -> String {
        let old_ip = self.ip;
        self.ip = origin as u16;
        let mut res = String::new();

        for i in 0..count {
            let op = self.disasm_instruction();
            res.push_str(&op.pretty_string());
            res.push_str("\n");
            self.ip += op.length as u16;
        }

        self.ip = old_ip;
        res
    }

    pub fn disasm_instruction(&mut self) -> InstructionInfo {
        let old_ip = self.ip;
        let op = self.decode_instruction(Segment::Default());
        let length = self.ip - old_ip;
        self.ip = old_ip;
        let offset = ((self.sreg16[CS] as usize) * 16) + old_ip as usize;

        InstructionInfo {
            segment: self.sreg16[CS] as usize,
            offset: old_ip as usize,
            length: length as usize,
            text: format!("{}", op),
            bytes: self.read_u8_slice(offset, length as usize),
            instruction: op,
        }
    }

    fn execute(&mut self, op: &Instruction) {
        self.instruction_count += 1;
        match op.command {
            Op::Add8() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_carry_u8(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Add16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_carry_u16(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u16(&op.params.dst, op.segment, (res & 0xFFFF) as u16);
            }
            Op::And8() => {
                // two parameters (dst=reg)
                println!("XXX impl and8");
                // XXX flags
            }
            Op::And16() => {
                // two parameters (dst=reg)
                println!("XXX impl and16");
                // XXX flags
            }
            Op::CallNear() => {
                // call near rel
                let old_ip = self.ip;
                let temp_ip = self.read_parameter_value(&op.params.dst);
                self.push16(old_ip);
                self.ip = temp_ip as u16;
            }
            Op::Cbw() => {
                // Convert Byte to Word
                if self.r16[AX].lo_u8() & 0x80 != 0 {
                    self.r16[AX].set_hi(0xFF);
                } else {
                    self.r16[AX].set_hi(0x00);
                }
            }
            Op::Clc() => {
                self.flags.carry = false;
            }
            Op::Cld() => {
                self.flags.direction = false;
            }
            Op::Cli() => {
                self.flags.interrupt = false;
            }
            Op::Cmp8() => {
                // two parameters
                // Modify status flags in the same manner as the SUB instruction

                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF, OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_carry_u8(res, src, dst);
                self.flags.set_overflow_sub_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
            }
            Op::Cmp16() => {
                // XXX identical to Op::Sub16() except we dont use the result
                // two parameters
                // Modify status flags in the same manner as the SUB instruction

                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF, OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_carry_u16(res, src, dst);
                self.flags.set_overflow_sub_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
            }
            Op::Daa() => {
                // Decimal Adjust AL after Addition
                println!("XXX impl daa");
                // XXX there is examples in manual that can be made into tests
            }
            Op::Dec8() => {
                // single parameter (dst)
                let dst = self.read_parameter_value(&op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.flags.set_overflow_sub_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Dec16() => {
                // single parameter (dst)
                let dst = self.read_parameter_value(&op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.flags.set_overflow_sub_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u16(&op.params.dst, op.segment, (res & 0xFFFF) as u16);
            }
            Op::Div8() => {
                println!("XXX impl div8");
                // XXX flags
            }
            Op::Div16() => {
                println!("XXX impl div16");
                // XXX flags
            }
            Op::Hlt() => {
                println!("XXX impl hlt");
            }
            Op::In8() => {
                // Input from Port
                // two parameters (dst=AL)
                let src = self.read_parameter_value(&op.params.src);
                let data = self.in_port(src as u16);
                self.write_parameter_u8(&op.params.dst, data);
            }
            Op::Inc8() => {
                let dst = self.read_parameter_value(&op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Inc16() => {
                let dst = self.read_parameter_value(&op.params.dst);
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u16(&op.params.dst, op.segment, (res & 0xFFFF) as u16);
            }
            Op::Int() => {
                let int = self.read_parameter_value(&op.params.dst);
                self.int(int as u8);
            }
            Op::Ja() => {
                // Jump short if above (CF=0 and ZF=0).
                if !self.flags.carry & !self.flags.zero {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jc() => {
                // Jump short if carry (CF=1).
                if self.flags.carry {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jg() => {
                // Jump short if greater (ZF=0 and SF=OF).
                if !self.flags.zero & self.flags.sign == self.flags.overflow {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::JmpNear() | Op::JmpShort() => {
                self.ip = self.read_parameter_value(&op.params.dst) as u16;
            }
            Op::Jna() => {
                // Jump short if not above (CF=1 or ZF=1).
                if self.flags.carry | self.flags.zero {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jnc() => {
                // Jump short if not carry (CF=0).
                if !self.flags.carry {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jnl() => {
                // Jump short if not less (SF=OF).
                if self.flags.sign == self.flags.overflow {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jnz() => {
                // Jump short if not zero (ZF=0).
                if !self.flags.zero {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Js() => {
                // Jump short if sign (SF=1).
                if self.flags.sign {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Jz() => {
                // Jump short if zero (ZF ← 1).
                if self.flags.zero {
                    self.ip = self.read_parameter_value(&op.params.dst) as u16;
                }
            }
            Op::Lea16() => {
                // Load Effective Address
                // Store effective address for m in register r16
                let src = self.read_parameter_address(&op.params.src) as u16;
                self.write_parameter_u16(&op.params.dst, op.segment, src);
            }
            Op::Lodsb() => {
                println!("XXX impl lodsb");
            }
            Op::Lodsw() => {
                println!("XXX impl lodsw");
            }
            Op::Loop() => {
                let dst = self.read_parameter_value(&op.params.dst) as u16;
                self.r16[CX].val -= 1;
                if self.r16[CX].val != 0 {
                    self.ip = dst;
                }
                // No flags affected.
            }
            Op::Mov8() => {
                // two parameters (dst=reg)
                let data = self.read_parameter_value(&op.params.src) as u8;
                self.write_parameter_u8(&op.params.dst, data);
            }
            Op::Mov16() => {
                // two parameters (dst=reg)
                let data = self.read_parameter_value(&op.params.src) as u16;
                self.write_parameter_u16(&op.params.dst, op.segment, data);
            }
            Op::Mul8() => {
                println!("XXX impl mul8");
            }
            Op::Nop() => {}
            Op::Or8() => {
                // two arguments (dst=AL)
                println!("XXX impl or8");
            }
            Op::Or16() => {
                // two arguments (dst=AX)
                println!("XXX impl or16");
            }
            Op::Out8() => {
                // two arguments (dst=DX or imm8)
                let data = self.read_parameter_value(&op.params.src) as u8;
                self.out_u8(&op.params.dst, data);
            }
            Op::Pop16() => {
                // single parameter (dst)
                let data = self.pop16();
                self.write_parameter_u16(&op.params.dst, op.segment, data);
            }
            Op::Push16() => {
                // single parameter (dst)
                let data = self.read_parameter_value(&op.params.dst) as u16;
                self.push16(data);
            }
            Op::Pushf() => {
                // push FLAGS register onto stack
                let data = self.flags.u16();
                println!("XXX push flags: {:04X}", data);
                self.push16(data);
            }
            Op::Rcl8() => {
                // two arguments
                // rotate 9 bits `src` times
                let src = self.read_parameter_value(&op.params.src) as u8;
                let dst = self.read_parameter_value(&op.params.dst) as u8;

                // XXX do + flags + write result
                println!("XXX impl rcl8");
            }
            Op::Rcr8() => {
                // two arguments
                // rotate 9 bits `src` times
                let src = self.read_parameter_value(&op.params.src) as u8;
                let dst = self.read_parameter_value(&op.params.dst) as u8;

                // XXX do + flags + write result

                // The RCR instruction shifts the CF flag into the most-significant
                // bit and shifts the least-significant bit into the CF flag.
                // The OF flag is affected only for single-bit rotates; it is undefined
                // for multi-bit rotates. The SF, ZF, AF, and PF flags are not affected.
                println!("XXX impl rcr8");
            }
            Op::RepMovsb() => {
                // Move (E)CX bytes from DS:[(E)SI] to ES:[(E)DI].
                let mut src = (self.sreg16[DS] as usize) * 16 + (self.r16[SI].val as usize);
                let mut dst = (self.sreg16[ES] as usize) * 16 + (self.r16[DI].val as usize);
                let count = self.r16[CX].val as usize;
                println!("rep movsb   src = {:04X}, dst = {:04X}, count = {:04X}",
                         src,
                         dst,
                         count);
                loop {
                    let b = self.peek_u8_at(src);
                    src += 1;
                    // println!("rep movsb   write {:02X} to {:04X}", b, dst);
                    self.write_u8(dst, b);
                    dst += 1;
                    self.r16[CX].val -= 1;
                    if self.r16[CX].val == 0 {
                        break;
                    }
                }
            }
            Op::RepMovsw() => {
                // Move (E)CX bytes from DS:[(E)SI] to ES:[(E)DI].
                let mut src = (self.sreg16[DS] as usize) * 16 + (self.r16[SI].val as usize);
                let mut dst = (self.sreg16[ES] as usize) * 16 + (self.r16[DI].val as usize);
                let count = self.r16[CX].val as usize;
                println!("rep movsw   src = {:04X}, dst = {:04X}, count = {:04X}",
                         src,
                         dst,
                         count);
                loop {
                    let b = self.peek_u16_at(src);
                    src += 1;
                    // println!("rep movsb   write {:02X} to {:04X}", b, dst);
                    self.write_u16(dst, b);
                    dst += 1;
                    self.r16[CX].val -= 1;
                    if self.r16[CX].val == 0 {
                        break;
                    }
                }
            }
            Op::Retf() => {
                //no arguments
                self.ip = self.pop16();
                self.sreg16[CS] = self.pop16();
            }
            Op::Retn() => {
                // no arguments
                self.ip = self.pop16();
            }
            Op::Ror8() => {
                // two arguments
                println!("XXX impl ror8");
                // XXX flags
            }
            Op::Ror16() => {
                // two arguments
                println!("XXX impl ror16");
                // XXX flags
            }
            Op::Sahf() => {
                // Store AH into Flags
                println!("XXX impl sahf");
            }
            Op::Shl8() => {
                // two arguments
                println!("XXX impl shl8");
                // XXX flags
            }
            Op::Shl16() => {
                // two arguments
                println!("XXX impl shl16");
                // XXX flags
            }
            Op::Shr8() => {
                // two arguments
                println!("XXX impl shr8");
                // XXX flags
            }
            Op::Shr16() => {
                // two arguments
                println!("XXX impl shr16");
                // XXX flags
            }
            Op::Sti() => {
                // Set Interrupt Flag
                self.flags.interrupt = true;
            }
            Op::Stosb() => {
                // no parameters
                // store AL at ES:(E)DI
                let offset = (self.sreg16[ES] as usize) * 16 + (self.r16[DI].val as usize);
                let data = self.r16[AX].lo_u8(); // = AL
                self.write_u8(offset, data);
                if !self.flags.direction {
                    self.r16[DI].val += 1;
                } else {
                    self.r16[DI].val -= 1;
                }
            }
            Op::Sub8() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.flags.set_overflow_sub_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
                self.flags.set_carry_u8(res, src, dst);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Sub16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.flags.set_overflow_sub_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
                self.flags.set_carry_u16(res, src, dst);

                self.write_parameter_u16(&op.params.dst, op.segment, (res & 0xFFFF) as u16);
            }
            Op::Test8() => {
                // two parameters
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst & src;
                // set SF, ZF, PF according to result.
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_parity(res);
            }
            Op::Test16() => {
                // two parameters
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst & src;
                // set SF, ZF, PF according to result.
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);
            }
            Op::Xchg8() => {
                // two parameters (registers)
                let mut src = self.read_parameter_value(&op.params.src);
                let mut dst = self.read_parameter_value(&op.params.dst);
                mem::swap(&mut src, &mut dst);
                self.write_parameter_u8(&op.params.dst, dst as u8);
                self.write_parameter_u8(&op.params.src, src as u8);
            }
            Op::Xchg16() => {
                // two parameters (registers)
                let mut src = self.read_parameter_value(&op.params.src);
                let mut dst = self.read_parameter_value(&op.params.dst);
                mem::swap(&mut src, &mut dst);
                self.write_parameter_u16(&op.params.dst, op.segment, dst as u16);
                self.write_parameter_u16(&op.params.src, op.segment, src as u16);
            }
            Op::Xor8() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.flags.overflow = false;
                self.flags.carry = false;
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.params.dst, (res & 0xFF) as u8);
            }
            Op::Xor16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.params.src);
                let dst = self.read_parameter_value(&op.params.dst);
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.flags.overflow = false;
                self.flags.carry = false;
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);

                self.write_parameter_u16(&op.params.dst, op.segment, (res & 0xFFFF) as u16);
            }
            _ => {
                println!("execute error: unhandled: {:?} at {:06X}",
                         op.command,
                         self.get_offset());
            }
        }
    }

    fn decode_instruction(&mut self, seg: Segment) -> Instruction {
        let b = self.read_u8();
        let mut op = Instruction {
            segment: seg,
            command: Op::Unknown(),
            params: ParameterPair {
                dst: Parameter::None(),
                src: Parameter::None(),
            },
        };

        match b {
            0x00 => {
                // add r/m8, r8
                op.command = Op::Add8();
                op.params = self.rm8_r8(op.segment);
            }
            0x02 => {
                // add r8, r/m8
                op.command = Op::Add8();
                op.params = self.r8_rm8(op.segment);
            }
            0x03 => {
                // add r16, r/m16
                op.command = Op::Add16();
                op.params = self.r16_rm16(op.segment);
            }
            0x04 => {
                // add AL, imm8
                op.command = Op::Add8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x05 => {
                // add AX, imm16
                op.command = Op::Add16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0x06 => {
                // push es
                op.command = Op::Push16();
                op.params.dst = Parameter::SReg16(ES);
            }
            0x07 => {
                // pop es
                op.command = Op::Pop16();
                op.params.dst = Parameter::SReg16(ES);
            }
            0x09 => {
                // or r/m16, r16
                op.command = Op::Or16();
                op.params = self.rm16_r16(op.segment);
            }
            0x0A => {
                // or r8, r/m8
                op.command = Op::Or8();
                op.params = self.r8_rm8(op.segment);
            }
            0x0B => {
                // or r16, r/m16
                op.command = Op::Or16();
                op.params = self.r16_rm16(op.segment);
            }
            0x0C => {
                // or AL, imm8
                op.command = Op::Or8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x0D => {
                // or AX, imm16
                op.command = Op::Or16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0x1E => {
                // push ds
                op.command = Op::Push16();
                op.params.dst = Parameter::SReg16(DS);
            }
            0x1F => {
                // pop ds
                op.command = Op::Pop16();
                op.params.dst = Parameter::SReg16(DS);
            }
            0x21 => {
                // and r/m16, r16
                op.command = Op::And16();
                op.params = self.rm16_r16(op.segment);
            }
            0x24 => {
                // and AL, imm8
                op.command = Op::And8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x26 => {
                // es segment prefix
                op = self.decode_instruction(Segment::ES());
            }
            0x27 => {
                // daa
                op.command = Op::Daa();
            }
            0x2A => {
                // sub r8, r/m8
                op.command = Op::Sub8();
                op.params = self.r8_rm8(op.segment);
            }
            0x2B => {
                // sub r16, r/m16
                op.command = Op::Sub16();
                op.params = self.r16_rm16(op.segment);
            }
            0x2C => {
                // sub AL, imm8
                op.command = Op::Sub8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x2D => {
                // sub AX, imm16
                op.command = Op::Sub16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0x2E => {
                // XXX if next op is a Jcc, then this is a "branch not taken" hint
                op = self.decode_instruction(Segment::CS());
            }
            0x30 => {
                // xor r/m8, r8
                op.command = Op::Xor8();
                op.params = self.rm8_r8(op.segment);
            }
            0x31 => {
                // xor r/m16, r16
                op.command = Op::Xor16();
                op.params = self.rm16_r16(op.segment);
            }
            0x32 => {
                // xor r8, r/m8
                op.command = Op::Xor8();
                op.params = self.r8_rm8(op.segment);
            }
            0x33 => {
                // xor r16, r/m16
                op.command = Op::Xor16();
                op.params = self.r16_rm16(op.segment);
            }
            0x36 => {
                // ss segment prefix
                op = self.decode_instruction(Segment::SS());
            }
            0x38 => {
                // cmp r/m8, r8
                op.command = Op::Cmp8();
                op.params = self.rm8_r8(op.segment);
            }
            0x3A => {
                // cmp r8, r/m8
                op.command = Op::Cmp8();
                op.params = self.r8_rm8(op.segment);
            }
            0x3B => {
                // CMP r16, r/m16
                op.command = Op::Cmp16();
                op.params = self.r16_rm16(op.segment);
            }
            0x3C => {
                // cmp AL, imm8
                op.command = Op::Cmp8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0x3D => {
                // cmp AX, imm16
                op.command = Op::Cmp16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0x3E => {
                // ds segment prefix
                // XXX if next op is a Jcc, then this is a "branch taken" hint
                op = self.decode_instruction(Segment::DS());
            }
            0x40...0x47 => {
                // inc r16
                op.command = Op::Inc16();
                op.params.dst = Parameter::Reg16((b & 7) as usize);
            }
            0x48...0x4F => {
                // dec r16
                op.command = Op::Dec16();
                op.params.dst = Parameter::Reg16((b & 7) as usize);
            }
            0x50...0x57 => {
                // push r16
                op.command = Op::Push16();
                op.params.dst = Parameter::Reg16((b & 7) as usize);
            }
            0x58...0x5F => {
                // pop r16
                op.command = Op::Pop16();
                op.params.dst = Parameter::Reg16((b & 7) as usize);
            }
            0x72 => {
                // jc rel8    (alias: jb, jnae)
                op.command = Op::Jc();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x73 => {
                // jnc rel8    (alias: jae, jnb)
                op.command = Op::Jnc();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x74 => {
                // jz rel8    (alias: je)
                op.command = Op::Jz();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x75 => {
                // jnz rel8   (alias: jne)
                op.command = Op::Jnz();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x76 => {
                // jna rel8    (alias: jbe)
                op.command = Op::Jna();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x77 => {
                // ja rel8    (alias: jnbe)
                op.command = Op::Ja();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x78 => {
                // js rel8
                op.command = Op::Js();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x7D => {
                // jnl rel8    (alias: jge)
                op.command = Op::Jnl();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x7F => {
                // jg rel8    (alias: jnle)
                op.command = Op::Jg();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0x80 => {
                // arithmetic 8-bit
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8());
                match x.reg {
                    0 => {
                        op.command = Op::Add8();
                    }
                    1 => {
                        op.command = Op::Or8();
                    }
                    /*
                    2 => {
                        op.command = Op::Adc8();
                    }
                    3 => {
                        op.command = Op::Sbb8();
                    }
                    */
                    4 => {
                        op.command = Op::And8();
                    }
                    5 => {
                        op.command = Op::Sub8();
                    }
                    6 => {
                        op.command = Op::Xor8();
                    }
                    7 => {
                        op.command = Op::Cmp8();
                    }
                    _ => {
                        println!("op 80 error: unknown reg {}", x.reg);
                        self.fatal_error = true;
                    }
                }
            }
            0x81 => {
                // arithmetic 16-bit
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm16(self.read_u16());
                match x.reg {
                    0 => {
                        op.command = Op::Add16();
                    }
                    /*
                    1 => {
                        op.command = Op::Or16();
                    }
                    2 => {
                        op.command = Op::Adc16();
                    }
                    3 => {
                        op.command = Op::Sbb16();
                    }
                    4 => {
                        op.command = Op::And16();
                    }
                    */
                    5 => {
                        op.command = Op::Sub16();
                    }
                    /*
                    6 => {
                        op.command = Op::Xor16();
                    }
                    */
                    7 => {
                        op.command = Op::Cmp16();
                    }
                    _ => {
                        println!("op 81 error: unknown reg {}", x.reg);
                        self.fatal_error = true;
                    }
                }
            }
            0x83 => {
                // arithmetic 16-bit with signed 8-bit value
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::ImmS8(self.read_s8());
                match x.reg {
                    0 => {
                        op.command = Op::Add16();
                    }
                    /*
                    1 => {
                        op.command = Op::Or16();
                    }
                    2 => {
                        op.command = Op::Adc16();
                    }
                    3 => {
                        op.command = Op::Sbb16();
                    }
                    4 => {
                        op.command = Op::And16();
                    }
                    */
                    5 => {
                        op.command = Op::Sub16();
                    }
                    /*
                    6 => {
                        op.command = Op::Xor16();
                    }
                    */
                    7 => {
                        op.command = Op::Cmp16();
                    }
                    _ => {
                        println!("op 83 error: unknown reg {}", x.reg);
                        self.fatal_error = true;
                    }
                }
            }
            0x84 => {
                // test r/m8, r8
                op.command = Op::Test8();
                op.params = self.rm8_r8(op.segment);
            }
            0x85 => {
                // test r/m16, r16
                op.command = Op::Test16();
                op.params = self.rm16_r16(op.segment);
            }
            0x86 => {
                // xchg r/m8, r8 | xchg r8, r/m8
                op.command = Op::Xchg8();
                op.params = self.rm8_r8(op.segment);
            }
            0x87 => {
                // xchg r/m16, r16 | xchg r16, r/m16
                op.command = Op::Xchg16();
                op.params = self.rm16_r16(op.segment);
            }
            0x88 => {
                // mov r/m8, r8
                op.command = Op::Mov8();
                op.params = self.rm8_r8(op.segment);
            }
            0x89 => {
                // mov r/m16, r16
                op.command = Op::Mov16();
                op.params = self.rm16_r16(op.segment);
            }
            0x8A => {
                // mov r8, r/m8
                op.command = Op::Mov8();
                op.params = self.r8_rm8(op.segment);
            }
            0x8B => {
                // mov r16, r/m16
                op.command = Op::Mov16();
                op.params = self.r16_rm16(op.segment);
            }
            0x8C => {
                // mov r/m16, sreg
                op.command = Op::Mov16();
                op.params = self.rm16_sreg(op.segment);
            }
            0x8D => {
                // lea r16, m
                op.command = Op::Lea16();
                op.params = self.r16_m16(op.segment);
            }
            0x8E => {
                // mov sreg, r/m16
                op.command = Op::Mov16();
                op.params = self.sreg_rm16(op.segment);
            }
            0x8F => {
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                match x.reg {
                    0 => {
                        // pop r/m16
                        op.command = Op::Pop16();
                    }
                    _ => {
                        println!("op 8F unknown reg = {}", x.reg);
                        self.fatal_error = true;
                    }
                }
            }
            0x90 => {
                // nop
                op.command = Op::Nop();
            }
            0x91...0x97 => {
                // xchg AX, r16  | xchg r16, AX
                // NOTE:  ("xchg ax,ax" is an alias of "nop")
                op.command = Op::Xchg16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Reg16((b & 7) as usize);
            }
            0x98 => {
                // cbw
                op.command = Op::Cbw();
            }
            0x9C => {
                // pushf
                op.command = Op::Pushf();
            }
            0x9E => {
                // sahf
                op.command = Op::Sahf();
            }
            0xA0 => {
                // mov AL, moffs8
                op.command = Op::Mov8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Ptr8(op.segment, self.read_u16());
            }
            0xA1 => {
                // MOV AX, moffs16
                op.command = Op::Mov16();
                op.params.dst = Parameter::Reg16(AX);
                op.params.src = Parameter::Ptr16(op.segment, self.read_u16());
            }
            0xA2 => {
                // mov moffs8, AL
                op.command = Op::Mov8();
                op.params.dst = Parameter::Ptr8(op.segment, self.read_u16());
                op.params.src = Parameter::Reg8(AL);
            }
            0xA3 => {
                // mov moffs16, AX
                op.command = Op::Mov16();
                op.params.dst = Parameter::Ptr16(op.segment, self.read_u16());
                op.params.src = Parameter::Reg16(AX);
            }
            0xA8 => {
                op.command = Op::Test8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0xAA => {
                // stosb
                op.command = Op::Stosb();
            }
            0xAC => {
                // lodsb
                op.command = Op::Lodsb();
            }
            0xAD => {
                // lodsw
                op.command = Op::Lodsw();
            }
            0xB0...0xB7 => {
                // mov r8, u8
                op.command = Op::Mov8();
                op.params.dst = Parameter::Reg8((b & 7) as usize);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0xB8...0xBF => {
                // mov r16, u16
                op.command = Op::Mov16();
                op.params.dst = Parameter::Reg16((b & 7) as usize);
                op.params.src = Parameter::Imm16(self.read_u16());
            }
            0xC3 => {
                // ret [near]
                op.command = Op::Retn();
            }
            0xC6 => {
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm8(self.read_u8());
                match x.reg {
                    0 => {
                        // mov r/m8, imm8
                        op.command = Op::Mov8();
                    }
                    _ => {
                        println!("op C6 unknown reg = {}", x.reg);
                        self.fatal_error = true;
                    }
                }
            }
            0xC7 => {
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm16(self.read_u16());
                match x.reg {
                    0 => {
                        // mov r/m16, imm16
                        op.command = Op::Mov16();
                    }
                    _ => {
                        println!("op C7 unknown reg = {}", x.reg);
                        self.fatal_error = true;
                    }
                }
            }
            0xCB => {
                // retf
                op.command = Op::Retf();
            }
            0xCD => {
                // int imm8
                op.command = Op::Int();
                op.params.dst = Parameter::Imm8(self.read_u8());
            }
            0xD0 => {
                // bit shift byte
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    // 0 => Op::Rol8(),
                    1 => Op::Ror8(),
                    2 => Op::Rcl8(),
                    3 => Op::Rcr8(),
                    4 => Op::Shl8(), // alias: sal
                    5 => Op::Shr8(),
                    // 7 => Op::Sar8(),
                    _ => {
                        println!("XXX 0xD0 unhandled reg = {}", x.reg);
                        self.fatal_error = true;
                        Op::Unknown()
                    }
                };
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm8(1);
            }
            0xD1 => {
                // bit shift word
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    // 0 => Op::Rol16(),
                    // 1 => Op::Ror16(),
                    //2 => Op::Rcl16(),
                    //3 => Op::Rcr16(),
                    4 => Op::Shl16(), // alias: sal
                    5 => Op::Shr16(),
                    // 7 => Op::Sar16(),
                    _ => {
                        println!("XXX 0xD1 unhandled reg = {}", x.reg);
                        self.fatal_error = true;
                        Op::Unknown()
                    }
                };
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Imm16(1);
            }
            0xD3 => {
                // bit shift word
                let x = self.read_mod_reg_rm();
                op.command = match x.reg {
                    //0 => Op::Rol16(),
                    1 => Op::Ror16(),
                    //2 => Op::Rcl16(),
                    //3 => Op::Rcr16(),
                    4 => Op::Shl16(), // alias: sal
                    //5 => Op::Shr16(),
                    //7 => Op::Sar16(),
                    _ => {
                        println!("XXX 0xD3 unhandled reg = {}", x.reg);
                        self.fatal_error = true;
                        Op::Unknown()
                    }
                };
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                op.params.src = Parameter::Reg8(CL);
            }
            0xE2 => {
                // loop rel8
                op.command = Op::Loop();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0xE4 => {
                // in AL, imm8
                op.command = Op::In8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Imm8(self.read_u8());
            }
            0xE6 => {
                // OUT imm8, AL
                op.command = Op::Out8();
                op.params.dst = Parameter::Imm8(self.read_u8());
                op.params.src = Parameter::Reg8(AL);
            }
            0xE8 => {
                // call near s16
                op.command = Op::CallNear();
                op.params.dst = Parameter::Imm16(self.read_rel16());
            }
            0xE9 => {
                // jmp near rel16
                op.command = Op::JmpNear();
                op.params.dst = Parameter::Imm16(self.read_rel16());
            }
            0xEB => {
                // jmp short rel8
                op.command = Op::JmpShort();
                op.params.dst = Parameter::Imm16(self.read_rel8());
            }
            0xEC => {
                // in AL, DX
                op.command = Op::In8();
                op.params.dst = Parameter::Reg8(AL);
                op.params.src = Parameter::Reg16(DX);
            }
            0xEE => {
                op.command = Op::Out8();
                op.params.dst = Parameter::Reg16(DX);
                op.params.src = Parameter::Reg8(AL);
            }
            0xF3 => {
                // rep
                let b = self.read_u8();
                match b {
                    0xA4 => {
                        // rep movs byte
                        op.command = Op::RepMovsb();
                    }
                    0xA5 => {
                        // rep movs word
                        op.command = Op::RepMovsw();
                    }
                    _ => {
                        println!("op f3 error: unhandled op {:02X}", b);
                        self.fatal_error = true;
                    }
                }
            }
            0xF4 => {
                op.command = Op::Hlt();
            }
            0xF6 => {
                // byte sized math
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                match x.reg {
                    0 => {
                        // test r/m8, imm8
                        op.command = Op::Test8();
                        op.params.src = Parameter::Imm8(self.read_u8());
                    }
                    // 2 => op.Cmd = "not"
                    // 3 => op.Cmd = "neg"
                    4 => {
                        // mul r/m8
                        op.command = Op::Mul8();
                    }
                    // 5 => op.Cmd = "imul"
                    6 => {
                        // div r/m8
                        op.command = Op::Div8();
                    }
                    // 7 => op.Cmd = "idiv"
                    _ => {
                        println!("op F6 unknown reg={}", x.reg);
                        self.fatal_error = true;
                    }
                }
            }
            0xF7 => {
                // word sized math
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                match x.reg {
                    0 => {
                        // test r/m16, imm16
                        op.command = Op::Test16();
                        op.params.src = Parameter::Imm16(self.read_u16());
                    }
                    // 2 => op.Cmd = "not"
                    // 3 => op.Cmd = "neg"
                    // 4 => op.Cmd = "mul"
                    // 5 => op.Cmd = "imul"
                    6 => {
                        // div r/m16
                        op.command = Op::Div16();
                    }
                    // 7 => op.Cmd = "idiv"
                    _ => {
                        println!("op F7 unknown reg={}", x.reg);
                        self.fatal_error = true;
                    }
                }
            }
            0xF8 => {
                // clc
                op.command = Op::Clc();
            }
            0xFA => {
                // cli
                op.command = Op::Cli();
            }
            0xFB => {
                // sti
                op.command = Op::Sti();
            }
            0xFC => {
                // cld
                op.command = Op::Cld();
            }
            0xFE => {
                // byte size
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm8(op.segment, x.rm, x.md);
                match x.reg {
                    0 => {
                        op.command = Op::Inc8();
                    }
                    1 => {
                        op.command = Op::Dec8();
                    }
                    _ => {
                        println!("op FE error: unknown reg {}", x.reg);
                        self.fatal_error = true;
                    }
                }
            }
            0xFF => {
                // word size
                let x = self.read_mod_reg_rm();
                op.params.dst = self.rm16(op.segment, x.rm, x.md);
                match x.reg {
                    0 => {
                        // inc r/m16
                        op.command = Op::Inc16();
                    }
                    1 => {
                        // dec r/m16
                        op.command = Op::Dec16();
                    }
                    4 => {
                        // jmp r/m16
                        op.command = Op::JmpNear();
                    }
                    _ => {
                        println!("op FF error: unknown reg {}", x.reg);
                        self.fatal_error = true;
                    }
                }
            }
            _ => {
                println!("cpu: unknown op {:02X} at {:06X}", b, self.get_offset() - 1);
                self.fatal_error = true;
            }
        }
        op
    }

    // decode r8, r/m8
    fn r8_rm8(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: Parameter::Reg8(x.reg as usize),
            src: self.rm8(seg, x.rm, x.md),
        }
    }

    // decode r/m8, r8
    fn rm8_r8(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: self.rm8(seg, x.rm, x.md),
            src: Parameter::Reg8(x.reg as usize),
        }
    }

    // decode Sreg, r/m16
    fn sreg_rm16(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: Parameter::SReg16(x.reg as usize),
            src: self.rm16(seg, x.rm, x.md),
        }
    }

    // decode r/m16, Sreg
    fn rm16_sreg(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: self.rm16(seg, x.rm, x.md),
            src: Parameter::SReg16(x.reg as usize),
        }
    }

    // decode r16, r/m16
    fn r16_rm16(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: Parameter::Reg16(x.reg as usize),
            src: self.rm16(seg, x.rm, x.md),
        }
    }

    // decode r/m16, r16
    fn rm16_r16(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        ParameterPair {
            dst: self.rm16(seg, x.rm, x.md),
            src: Parameter::Reg16(x.reg as usize),
        }
    }

    // decode r16, m16
    fn r16_m16(&mut self, seg: Segment) -> ParameterPair {
        let x = self.read_mod_reg_rm();
        if x.md == 3 {
            println!("r16_m16 error: invalid encoding, ip={:04X}", self.ip);
        }
        ParameterPair {
            dst: Parameter::Reg16(x.reg as usize),
            src: self.rm16(seg, x.rm, x.md),
        }
    }

    // decode rm8
    fn rm8(&mut self, seg: Segment, rm: u8, md: u8) -> Parameter {
        match md {
            0 => {
                if rm == 6 {
                    // [u16]
                    Parameter::Ptr8(seg, self.read_u16())
                } else {
                    // [amode]
                    Parameter::Ptr8Amode(seg, rm as usize)
                }
            }
            // [amode+s8]
            1 => Parameter::Ptr8AmodeS8(seg, rm as usize, self.read_s8()),
            // [amode+s16]
            2 => Parameter::Ptr8AmodeS16(seg, rm as usize, self.read_s16()),
            // [reg]
            _ => Parameter::Reg8(rm as usize),
        }
    }

    // decode rm16
    fn rm16(&mut self, seg: Segment, rm: u8, md: u8) -> Parameter {
        match md {
            0 => {
                if rm == 6 {
                    // [u16]
                    Parameter::Ptr16(seg, self.read_u16())
                } else {
                    // [amode]
                    Parameter::Ptr16Amode(seg, rm as usize)
                }
            }
            // [amode+s8]
            1 => Parameter::Ptr16AmodeS8(seg, rm as usize, self.read_s8()),
            // [amode+s16]
            2 => Parameter::Ptr16AmodeS16(seg, rm as usize, self.read_s16()),
            // [reg]
            _ => Parameter::Reg16(rm as usize),
        }
    }

    fn push16(&mut self, data: u16) {
        self.r16[SP].val -= 2;
        let offset = (self.sreg16[SS] as usize) * 16 + (self.r16[SP].val as usize);
        /*
        println!("push16 {:04X}  to {:04X}:{:04X}  =>  {:06X}       instr {}",
                 data,
                 self.sreg16[SS],
                 self.r16[SP].val,
                 offset,
                 self.instruction_count);
        */
        self.write_u16(offset, data);
    }

    fn pop16(&mut self) -> u16 {
        let offset = (self.sreg16[SS] as usize) * 16 + (self.r16[SP].val as usize);
        let data = self.peek_u16_at(offset);
        /*
        println!("pop16 {:04X}  from {:04X}:{:04X}  =>  {:06X}       instr {}",
                 data,
                 self.sreg16[SS],
                 self.r16[SP].val,
                 offset,
                 self.instruction_count);
        */
        self.r16[SP].val += 2;
        data
    }

    fn read_mod_reg_rm(&mut self) -> ModRegRm {
        let b = self.read_u8();
        ModRegRm {
            md: b >> 6, // high 2 bits
            reg: (b >> 3) & 7, // mid 3 bits
            rm: b & 7, // low 3 bits
        }
    }

    pub fn get_offset(&self) -> usize {
        ((self.sreg16[CS] as usize) * 16) + self.ip as usize
    }

    fn read_u8(&mut self) -> u8 {
        let offset = self.get_offset();
        let b = self.memory[offset];
        /*
        println!("___ DBG: read u8 {:02X} from {:06X} ... {:04X}:{:04X}",
              b,
              offset,
              self.sreg16[CS],
              self.ip);
        */

        // self.ip = (Wrapping(self.ip) + Wrapping(1)).0;  // XXX what do if ip wraps?
        self.ip += 1;
        b
    }

    fn read_u16(&mut self) -> u16 {
        let lo = self.read_u8();
        let hi = self.read_u8();
        (hi as u16) << 8 | lo as u16
    }

    fn read_s8(&mut self) -> i8 {
        self.read_u8() as i8
    }

    fn read_s16(&mut self) -> i16 {
        self.read_u16() as i16
    }

    fn read_rel8(&mut self) -> u16 {
        let val = self.read_u8() as i8;
        (self.ip as i16 + (val as i16)) as u16
    }

    fn read_rel16(&mut self) -> u16 {
        let val = self.read_u16() as i16;
        (self.ip as i16 + val) as u16
    }

    fn peek_u8_at(&mut self, pos: usize) -> u8 {
        // println!("peek_u8_at   pos {:04X}  = {:02X}", pos, self.memory[pos]);
        self.memory[pos]
    }

    fn peek_u16_at(&mut self, pos: usize) -> u16 {
        let lo = self.peek_u8_at(pos);
        let hi = self.peek_u8_at(pos + 1);
        (hi as u16) << 8 | lo as u16
    }

    fn write_u16(&mut self, offset: usize, data: u16) {
        // println!("write_u16 [{:04X}] = {:04X}", offset, data);
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.write_u8(offset, lo);
        self.write_u8(offset + 1, hi);
    }

    // returns the offset part, excluding segment. used by LEA
    fn read_parameter_address(&mut self, p: &Parameter) -> usize {
        match *p {
            Parameter::Ptr16AmodeS8(seg, r, imm) => self.amode16(r) + imm as usize,
            Parameter::Ptr16(seg, imm) => imm as usize,
            _ => {
                println!("read_parameter_address error: unhandled parameter: {:?} at {:06X}",
                         p,
                         self.get_offset());
                0
            }
        }
    }

    fn read_parameter_value(&mut self, p: &Parameter) -> usize {
        match *p {
            Parameter::Imm8(imm) => imm as usize,
            Parameter::Imm16(imm) => imm as usize,
            Parameter::ImmS8(imm) => imm as usize,
            Parameter::Ptr8(seg, imm) => {
                let offset = (self.segment(seg) as usize * 16) + imm as usize;
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr16(seg, imm) => {
                let offset = (self.segment(seg) as usize * 16) + imm as usize;
                self.peek_u16_at(offset) as usize
            }
            Parameter::Ptr8Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr8AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr8AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.peek_u8_at(offset) as usize
            }
            Parameter::Ptr16Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.peek_u16_at(offset) as usize
            }
            Parameter::Ptr16AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.peek_u16_at(offset) as usize
            }
            Parameter::Ptr16AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.peek_u16_at(offset) as usize
            }
            Parameter::Reg8(r) => {
                let lor = r & 3;
                if r & 4 == 0 {
                    self.r16[lor].lo_u8() as usize
                } else {
                    self.r16[lor].hi_u8() as usize
                }
            }
            Parameter::Reg16(r) => self.r16[r].val as usize,
            Parameter::SReg16(r) => self.sreg16[r] as usize,
            _ => {
                println!("read_parameter_value error: unhandled parameter: {:?} at {:06X}",
                         p,
                         self.get_offset());
                0
            }
        }
    }

    fn write_parameter_u8(&mut self, p: &Parameter, data: u8) {
        match *p {
            Parameter::Reg8(r) => {
                let lor = r & 3;
                if r & 4 == 0 {
                    self.r16[lor].set_lo(data);
                } else {
                    self.r16[lor].set_hi(data);
                }
            }
            Parameter::Ptr8(seg, imm) => {
                let offset = (self.segment(seg) as usize * 16) + imm as usize;
                self.write_u8(offset, data);
            }
            Parameter::Ptr8Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.write_u8(offset, data);
            }
            Parameter::Ptr8AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.write_u8(offset, data);
            }
            Parameter::Ptr8AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.write_u8(offset, data);
            }
            _ => {
                println!("write_parameter_u8 unhandled type {:?} at {:06X}",
                         p,
                         self.get_offset());
            }
        }
    }

    fn write_parameter_u16(&mut self, p: &Parameter, segment: Segment, data: u16) {
        match *p {
            Parameter::Reg16(r) => {
                self.r16[r].val = data;
            }
            Parameter::SReg16(r) => {
                self.sreg16[r] = data;
            }
            Parameter::Imm16(imm) => {
                let offset = (self.segment(segment) as usize * 16) + imm as usize;
                self.write_u16(offset, data);
            }
            Parameter::Ptr16(seg, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) + Wrapping(imm as usize)).0;
                self.write_u16(offset, data);
            }
            Parameter::Ptr16Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.write_u16(offset, data);
            }
            Parameter::Ptr16AmodeS8(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.write_u16(offset, data);
            }
            Parameter::Ptr16AmodeS16(seg, r, imm) => {
                let offset = (Wrapping(self.segment(seg) as usize * 16) +
                              Wrapping(self.amode16(r)) +
                              Wrapping(imm as usize))
                        .0;
                self.write_u16(offset, data);
            }
            _ => {
                println!("write_u16_param unhandled type {:?} at {:06X}",
                         p,
                         self.get_offset());
            }
        }
    }

    fn write_u8(&mut self, offset: usize, data: u8) {
        // println!("debug: write_u8 to {:06X} = {:02X}", offset, data);
        self.memory[offset] = data;
    }

    // used by disassembler
    pub fn read_u8_slice(&mut self, offset: usize, length: usize) -> Vec<u8> {
        let mut res = vec![0u8; length];
        for i in offset..offset + length {
            res[i - offset] = self.memory[i];
        }
        res
    }

    fn segment(&self, seg: Segment) -> u16 {
        match seg {
            Segment::CS() |
            Segment::Default() => self.sreg16[CS],
            Segment::DS() => self.sreg16[DS],
            Segment::ES() => self.sreg16[ES],
            Segment::SS() => self.sreg16[SS],
        }
    }

    fn amode16(&mut self, idx: usize) -> usize {
        match idx {
            0 => self.r16[BX].val as usize + self.r16[SI].val as usize,
            1 => self.r16[BX].val as usize + self.r16[DI].val as usize,
            2 => self.r16[BP].val as usize + self.r16[SI].val as usize,
            3 => self.r16[BP].val as usize + self.r16[DI].val as usize,
            4 => self.r16[SI].val as usize,
            5 => self.r16[DI].val as usize,
            6 => self.r16[BP].val as usize,
            7 => self.r16[BX].val as usize,
            _ => {
                println!("Impossible amode16, idx {}", idx);
                0
            }
        }
    }

    // output byte to I/O port
    fn out_u8(&mut self, p: &Parameter, data: u8) {
        let dst = match *p {
            Parameter::Reg16(r) => self.r16[r].val,
            Parameter::Imm8(imm) => imm as u16,
            _ => {
                println!("out_u8 unhandled type {:?}", p);
                0
            }
        };

        println!("XXX unhandled out_u8 to {:04X}, data {:02X}", dst, data);
    }

    // read byte from I/O port
    fn in_port(&mut self, port: u16) -> u8 {
        match port {
            0x03DA => {
                // R-  CGA status register
                // color EGA/VGA: input status 1 register
                //
                // Bitfields for CGA status register:
                // Bit(s)	Description	(Table P0818)
                // 7-6	not used
                // 7	(C&T Wingine) vertical sync in progress (if enabled by XR14)
                // 5-4	color EGA, color ET4000, C&T: diagnose video display feedback, select
                //      from color plane enable
                // 3	in vertical retrace
                //      (C&T Wingine) video active (retrace/video selected by XR14)
                // 2	(CGA,color EGA) light pen switch is off
                //      (MCGA,color ET4000) reserved (0)
                //      (VGA) reserved (1)
                // 1	(CGA,color EGA) positive edge from light pen has set trigger
                //      (VGA,MCGA,color ET4000) reserved (0)
                // 0	horizontal retrace in progress
                //    =0  do not use memory
                //    =1  memory access without interfering with display
                //        (VGA,Genoa SuperEGA) horizontal or vertical retrace
                //    (C&T Wingine) display enabled (retrace/DE selected by XR14)
                let mut flags = 0;

                // HACK: fake bit 0:
                if self.gpu.scanline == 0 {
                    flags |= 1; // set bit 0
                } else {
                    flags &= !(1 << 1); // clear bit 0
                }
                println!("XXX read io port CGA status register at {:06X} = {:02X}",
                         self.get_offset(),
                         flags);
                flags
            }
            _ => {
                println!("in_port: unhandled in8 {:04X} at {:06X}",
                         port,
                         self.get_offset());
                0
            }
        }
    }

    fn int(&mut self, int: u8) {
        // XXX jump to offset 0x21 in interrupt table (look up how hw does this)
        // http://wiki.osdev.org/Interrupt_Vector_Table
        match int {
            0x10 => self.int10(),
            0x20 => {
                // DOS 1+ - TERMINATE PROGRAM
                // NOTE: Windows overloads INT 20
                println!("INT 20 - Terminating program");
                exit(0);
            }
            0x21 => self.int21(),
            _ => {
                println!("int error: unknown interrupt {:02X}, AX={:04X}, BX={:04X}",
                         int,
                         self.r16[AX].val,
                         self.r16[BX].val);
            }
        }
    }

    // video related interrupts
    fn int10(&mut self) {
        match self.r16[AX].hi_u8() {
            0x00 => {
                // VIDEO - SET VIDEO MODE
                //
                // AL = desired video mode
                //
                // Return:
                // AL = video mode flag (Phoenix, AMI BIOS)
                // 20h mode > 7
                // 30h modes 0-5 and 7
                // 3Fh mode 6
                // AL = CRT controller mode byte (Phoenix 386 BIOS v1.10)
                //
                // Desc: Specify the display mode for the currently
                // active display adapter
                //
                // more info and video modes: http://www.ctyme.com/intr/rb-0069.htm
                match self.r16[AX].lo_u8() {
                    0x04 => {
                        // G  40x25  8x8   320x200    4       .   B800 CGA,PCjr,EGA,MCGA,VGA
                        println!("XXX video: set video mode to 320x200, 4 colors");
                        self.r16[AX].set_lo(0x30);
                    }
                    0x06 => {
                        //   G  80x25  8x8   640x200    2       .   B800 CGA,PCjr,EGA,MCGA,VGA
                        // = G  80x25   .       .     mono      .   B000 HERCULES.COM on HGC [14]
                        println!("XXX video: set video mode to 640x200, 2 colors");
                        self.r16[AX].set_lo(0x3F);
                    }
                    _ => {
                        println!("video error: unknown video mode {:02X}",
                                 self.r16[AX].lo_u8());
                    }
                }
            }
            0x02 => {
                // VIDEO - SET CURSOR POSITION
                //
                // BH = page number
                // 0-3 in modes 2&3
                // 0-7 in modes 0&1
                // 0 in graphics modes
                // DH = row (00h is top)
                // DL = column (00h is left)
                // Return: Nothing
                println!("XXX set cursor position, page={}, row={}, column={}",
                         self.r16[BX].hi_u8(),
                         self.r16[DX].hi_u8(),
                         self.r16[DX].lo_u8());
            }
            0x06 => {
                // VIDEO - SCROLL UP WINDOW

                // AL = number of lines by which to scroll up (00h = clear entire window)
                // BH = attribute used to write blank lines at bottom of window
                // CH,CL = row,column of window's upper left corner
                // DH,DL = row,column of window's lower right corner
                // Return: Nothing
                //
                // Note: Affects only the currently active page (see AH=05h)
                println!("XXX scroll window up: lines={},attrib={},topleft={},{},btmright={},{}",
                         self.r16[AL].lo_u8(),
                         self.r16[BX].hi_u8(),
                         self.r16[CX].hi_u8(),
                         self.r16[CX].lo_u8(),
                         self.r16[DX].hi_u8(),
                         self.r16[DX].lo_u8());
            }
            0x09 => {
                // VIDEO - WRITE CHARACTER AND ATTRIBUTE AT CURSOR POSITION
                //
                // AL = character to display
                // BH = page number (00h to number of pages - 1) (see #00010)
                //      background color in 256-color graphics modes (ET4000)
                // BL = attribute (text mode) or color (graphics mode)
                //      if bit 7 set in <256-color graphics mode, character
                //      is XOR'ed onto screen
                // CX = number of times to write character
                // Return: Nothing
                //
                // Notes: All characters are displayed, including CR, LF, and BS.
                // Replication count in CX may produce an unpredictable result
                // in graphics modes if it is greater than the number of positions
                // remaining in the current row. With PhysTechSoft's PTS ROM-DOS
                // the BH, BL, and CX values are ignored on entry.

                println!("XXX write character at pos: char={}, page={}, attrib={}, count={}",
                         self.r16[AX].lo_u8() as char,
                         self.r16[BX].hi_u8(),
                         self.r16[BX].lo_u8(),
                         self.r16[CX].val);
            }
            0x0B => {
                match self.r16[BX].hi_u8() {
                    0x00 => {
                        // VIDEO - SET BACKGROUND/BORDER COLOR
                        // BL = background/border color (border only in text modes)
                        // Return: Nothing
                        println!("XXX set bg/border color to {:02X}", self.r16[BX].lo_u8());
                    }
                    0x01 => {
                        // VIDEO - SET PALETTE
                        // BL = palette ID
                        //    00h background, green, red, and brown/yellow
                        //    01h background, cyan, magenta, and white
                        // Return: Nothing
                        //
                        // Note: This call was only valid in 320x200 graphics on
                        // the CGA, but newer cards support it in many or all
                        // graphics modes
                        println!("XXX set palette id to {:02X}", self.r16[BX].lo_u8());
                    }
                    _ => {
                        println!("video error: unknown int 10, ah=0B, bh={:02X}",
                                 self.r16[BX].hi_u8());
                    }
                }
            }
            0x0E => {
                // VIDEO - TELETYPE OUTPUT
                // Display a character on the screen, advancing the cursor
                // and scrolling the screen as necessary
                //
                // AL = character to write
                // BH = page number
                // BL = foreground color (graphics modes only)
                // Return: Nothing
                //
                // Notes: Characters 07h (BEL), 08h (BS), 0Ah (LF),
                // and 0Dh (CR) are interpreted and do the expected things.
                // IBM PC ROMs dated 1981/4/24 and 1981/10/19 require
                // that BH be the same as the current active page
                //
                // BUG: If the write causes the screen to scroll, BP is destroyed
                // by BIOSes for which AH=06h destroys BP
                print!("{}", self.r16[AX].lo_u8() as char);
            }
            0x0F => {
                // VIDEO - GET CURRENT VIDEO MODE
                //
                // Return:
                // AH = number of character columns
                // AL = display mode (see AH=00h)
                // BH = active page (see AH=05h)
                //
                // more info: http://www.ctyme.com/intr/rb-0108.htm
                println!("XXX int10,0F - get video mode impl");
            }
            _ => {
                println!("int10 error: unknown AH={:02X}, AX={:04X}",
                         self.r16[AX].hi_u8(),
                         self.r16[AX].val);
            }
        }
    }

    // dos related interrupts
    fn int21(&mut self) {
        match self.r16[AX].hi_u8() {
            0x06 => {
                // DOS 1+ - DIRECT CONSOLE OUTPUT
                //
                // DL = character (except FFh)
                //
                // Notes: Does not check ^C/^Break. Writes to standard output,
                // which is always the screen under DOS 1.x, but may be redirected
                // under DOS 2+
                let b = self.r16[DX].lo_u8();
                if b != 0xFF {
                    print!("{}", b as char);
                } else {
                    println!("XXX character out: {:02X}", b);
                }
                // Return:
                // AL = character output (despite official docs which
                // state nothing is returned) (at least DOS 2.1-7.0)
                self.r16[AX].set_lo(b);
            }
            0x09 => {
                // DOS 1+ - WRITE STRING TO STANDARD OUTPUT
                //
                // DS:DX -> '$'-terminated string
                //
                // Return:
                // AL = 24h (the '$' terminating the string, despite official docs which
                // state that nothing is returned) (at least DOS 2.1-7.0 and NWDOS)
                //
                // Notes: ^C/^Break are checked, and INT 23 is called if either pressed.
                // Standard output is always the screen under DOS 1.x, but may be
                // redirected under DOS 2+. Under the FlashTek X-32 DOS extender,
                // the pointer is in DS:EDX
                let mut offset = (self.sreg16[DS] as usize) * 16 + (self.r16[DX].val as usize);
                loop {
                    let b = self.peek_u8_at(offset) as char;
                    offset += 1;
                    if b == '$' {
                        break;
                    }
                    print!("{}", b as char);
                }
                self.r16[AX].set_lo(b'$');
            }
            0x0C => {
                // DOS 1+ - FLUSH BUFFER AND READ STANDARD INPUT
                // AL = STDIN input function to execute after flushing buffer
                // other registers as appropriate for the input function
                // Return: As appropriate for the specified input function
                //
                // Note: If AL is not one of 01h,06h,07h,08h, or 0Ah, the
                // buffer is flushed but no input is attempted
                println!("XXX int21, 0x0c - read stdin");
            }
            0x2C => {
                // DOS 1+ - GET SYSTEM TIME
                //
                // Note: On most systems, the resolution of the system clock
                // is about 5/100sec, so returned times generally do not increment
                // by 1. On some systems, DL may always return 00h

                let now = time::now();
                let centi_sec = now.tm_nsec / 10000000; // nanosecond to 1/100 sec

                // Return:
                self.r16[CX].set_hi(now.tm_hour as u8); // CH = hour
                self.r16[CX].set_lo(now.tm_min as u8); // CL = minute
                self.r16[DX].set_hi(now.tm_sec as u8); // DH = second
                self.r16[DX].set_lo(centi_sec as u8); // DL = 1/100 second
            }
            0x30 => {
                // DOS 2+ - GET DOS VERSION
                // ---DOS 5+ ---
                // AL = what to return in BH
                // 00h OEM number (see #01394)
                // 01h version flag
                //
                // Return:
                // AL = major version number (00h if DOS 1.x)
                // AH = minor version number
                // BL:CX = 24-bit user serial number (most versions do not use this)
                // ---if DOS <5 or AL=00h---
                // BH = MS-DOS OEM number (see #01394)
                // ---if DOS 5+ and AL=01h---
                // BH = version flag
                //
                // bit 3: DOS is in ROM


                // (Table 01394)
                // Values for DOS OEM number:
                // 00h *  IBM
                // -  (Novell DOS, Caldera OpenDOS, DR-OpenDOS, and DR-DOS 7.02+ report IBM
                // as their OEM)
                // 01h *  Compaq
                // 02h *  MS Packaged Product
                // 04h *  AT&T
                // 05h *  ZDS (Zenith Electronics, Zenith Electronics).

                // fake MS-DOS 3.10, as needed by msdos32/APPEND.COM
                self.r16[AX].set_lo(3); // AL = major version number (00h if DOS 1.x)
                self.r16[AX].set_hi(10); // AH = minor version number
            }
            0x40 => {
                // DOS 2+ - WRITE - WRITE TO FILE OR DEVICE

                // BX = file handle
                // CX = number of bytes to write
                // DS:DX -> data to write
                //
                // Return:
                // CF clear if successful
                // AX = number of bytes actually written
                // CF set on error
                // AX = error code (05h,06h) (see #01680 at AH=59h/BX=0000h)

                // Notes: If CX is zero, no data is written, and the file is truncated or extended
                // to the current position. Data is written beginning at the current file position,
                // and the file position is updated after a successful write. For FAT32 drives, the
                // file must have been opened with AX=6C00h with the "extended size" flag in order
                // to expand the file beyond 2GB; otherwise the write will fail with error code
                // 0005h (access denied). The usual cause for AX < CX on return is a full disk
                println!("XXX DOS - WRITE TO FILE OR DEVICE, handle={:04X}, count={:04X}, data from {:04X}:{:04X}",
                         self.r16[BX].val,
                         self.r16[CX].val,
                         self.sreg16[DS],
                         self.r16[DX].val);
            }
            0x4C => {
                // DOS 2+ - EXIT - TERMINATE WITH RETURN CODE
                // AL = return code

                // Notes: Unless the process is its own parent (see #01378 [offset 16h] at AH=26h),
                // all open files are closed and all memory belonging to the process is freed. All
                // network file locks should be removed before calling this function
                let al = self.r16[AX].lo_u8();
                print!("DOS - TERMINATE WITH RETURN CODE {:02X}", al);
                exit(0);
            }
            _ => {
                println!("int21 error: unknown AH={:02X}, AX={:04X}",
                         self.r16[AX].hi_u8(),
                         self.r16[AX].val);
            }
        }
    }
}

fn r8(reg: u8) -> &'static str {
    match reg {
        0 => "al",
        1 => "cl",
        2 => "dl",
        3 => "bl",
        4 => "ah",
        5 => "ch",
        6 => "dh",
        7 => "bh",
        _ => "?",
    }
}

fn r16(reg: u8) -> &'static str {
    match reg {
        0 => "ax",
        1 => "cx",
        2 => "dx",
        3 => "bx",
        4 => "sp",
        5 => "bp",
        6 => "si",
        7 => "di",
        _ => "?",
    }
}

fn sr16(reg: u8) -> &'static str {
    match reg {
        0 => "es",
        1 => "cs",
        2 => "ss",
        3 => "ds",
        4 => "fs",
        5 => "gs",
        _ => "?",
    }
}

// 16 bit addressing modes
fn amode(reg: u8) -> &'static str {
    match reg {
        0 => "bx+si",
        1 => "bx+di",
        2 => "bp+si",
        3 => "bp+di",
        4 => "si",
        5 => "di",
        6 => "bp",
        7 => "bx",
        _ => "?",
    }
}

#[test]
fn can_handle_stack() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x88, 0x88, // mov ax,0x8888
        0x8E, 0xD8,       // mov ds,ax
        0x1E,             // push ds
        0x07,             // pop es
    ];
    cpu.load_com(&code);

    cpu.execute_instruction(); // mov
    cpu.execute_instruction(); // mov

    assert_eq!(0xFFFE, cpu.r16[SP].val);
    cpu.execute_instruction(); // push
    assert_eq!(0xFFFC, cpu.r16[SP].val);
    cpu.execute_instruction(); // pop
    assert_eq!(0xFFFE, cpu.r16[SP].val);

    assert_eq!(0x107, cpu.ip);
    assert_eq!(0x8888, cpu.r16[AX].val);
    assert_eq!(0x8888, cpu.sreg16[DS]);
    assert_eq!(0x8888, cpu.sreg16[ES]);
}

#[test]
fn can_execute_mov_r8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB2, 0x13, // mov dl,0x13
        0x88, 0xD0, // mov al,dl
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x102, cpu.ip);
    assert_eq!(0x13, cpu.r16[DX].lo_u8());

    cpu.execute_instruction();
    assert_eq!(0x104, cpu.ip);
    assert_eq!(0x13, cpu.r16[AX].lo_u8());
}

#[test]
fn can_execute_mov_r8_rm8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x05, 0x01, // mov bx,0x105
        0x8A, 0x27,       // mov ah,[bx]   | r8, r/m8
        0x99,             // db 0x99
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x105, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x99, cpu.r16[AX].hi_u8());
}

#[test]
fn can_execute_mv_r16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB8, 0x23, 0x01, // mov ax,0x123
        0x8B, 0xE0,       // mov sp,ax   | r16, r16
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x123, cpu.r16[AX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x123, cpu.r16[SP].val);
}

#[test]
fn can_execute_mov_r16_rm16() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB9, 0x23, 0x01, // mov cx,0x123
        0x8E, 0xC1,       // mov es,cx   | r/m16, r16
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x123, cpu.r16[CX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x123, cpu.sreg16[ES]);
}

#[test]
fn can_execute_mov_rm16_sreg() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x34, 0x12,       // mov bx,0x1234
        0x8E, 0xC3,             // mov es,bx
        0x8C, 0x06, 0x09, 0x01, // mov [0x109],es  | r/m16, sreg
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x1234, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x1234, cpu.sreg16[ES]);

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);
    let cs = cpu.sreg16[CS] as usize;
    assert_eq!(0x1234, cpu.peek_u16_at((cs * 16) + 0x0109));
}

#[test]
fn can_execute_mov_data() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xC6, 0x06, 0x31, 0x10, 0x38,       // mov byte [0x1031],0x38
    ];
    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    let cs = cpu.sreg16[CS] as usize;
    assert_eq!(0x38, cpu.peek_u8_at((cs * 16) + 0x1031));
}

#[test]
fn can_execute_segment_prefixed() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x34, 0x12, // mov bx,0x1234
        0x8E, 0xC3,       // mov es,bx
        0xB4, 0x88,       // mov ah,0x88
        0x26, 0x88, 0x25, // mov [es:di],ah
        0x26, 0x8A, 0x05, // mov al,[es:di]
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x1234, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x1234, cpu.sreg16[ES]);

    cpu.execute_instruction();
    assert_eq!(0x107, cpu.ip);
    assert_eq!(0x88, cpu.r16[AX].hi_u8());

    cpu.execute_instruction();
    assert_eq!(0x10A, cpu.ip);
    let offset = (cpu.segment(Segment::ES()) as usize * 16) + cpu.amode16(5); // 5=amode DI
    assert_eq!(0x88, cpu.peek_u8_at(offset));

    cpu.execute_instruction();
    assert_eq!(0x10D, cpu.ip);
    assert_eq!(0x88, cpu.r16[AX].lo_u8());
}

#[test]
fn can_execute_imms8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBF, 0x00, 0x01, // mov di,0x100
        0x83, 0xC7, 0x3A, // add di,byte +0x3a
        0x83, 0xC7, 0xC6, // add di,byte -0x3a
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x0100, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x106, cpu.ip);
    assert_eq!(0x013A, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);
    assert_eq!(0x0100, cpu.r16[DI].val);
}

#[test]
fn can_execute_with_flags() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB4, 0xFE,       // mov ah,0xfe
        0x80, 0xC4, 0x02, // add ah,0x2   - OF and ZF should be set
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x102, cpu.ip);
    assert_eq!(0xFE, cpu.r16[AX].hi_u8());
    assert_eq!(false, cpu.flags.carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(false, cpu.flags.parity);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x00, cpu.r16[AX].hi_u8());
    assert_eq!(true, cpu.flags.carry);
    assert_eq!(true, cpu.flags.zero);
    assert_eq!(false, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(true, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_cmp() {
    // make sure we dont overflow (0 - 0x2000 = overflow)
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x00,       // mov bx,0x0
        0x89, 0xDF,             // mov di,bx
        0x81, 0xFF, 0x00, 0x20, // cmp di,0x2000
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);

    assert_eq!(true, cpu.flags.carry);
    assert_eq!(false, cpu.flags.zero);
    assert_eq!(true, cpu.flags.sign);
    assert_eq!(false, cpu.flags.overflow);
    assert_eq!(false, cpu.flags.auxiliary_carry);
    assert_eq!(true, cpu.flags.parity);
}

#[test]
fn can_execute_xchg() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x91, // xchg ax,cx
    ];

    cpu.load_com(&code);

    cpu.r16[AX].val = 0x1234;
    cpu.r16[CX].val = 0xFFFF;

    cpu.execute_instruction();
    assert_eq!(0xFFFF, cpu.r16[AX].val);
    assert_eq!(0x1234, cpu.r16[CX].val);
}

#[test]
fn can_execute_rep() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        // copy first 5 bytes into 0x200
        0x8D, 0x36, 0x00, 0x01, // lea si,[0x100]
        0x8D, 0x3E, 0x00, 0x02, // lea di,[0x200]
        0xB9, 0x05, 0x00,       // mov cx,0x5
        0xF3, 0xA4,             // rep movsb
    ];

    cpu.load_com(&code);

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.r16[SI].val);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x5, cpu.r16[CX].val);

    cpu.execute_instruction(); // rep movsb
    assert_eq!(0x0, cpu.r16[CX].val);
    let min = ((cpu.sreg16[CS] as usize) * 16) + 0x100;
    let max = min + 5;
    for i in min..max {
        assert_eq!(cpu.memory[i], cpu.memory[i + 0x100]);
    }
}

#[test]
fn can_execute_addressing() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x02,             // mov bx,0x200
        0xC6, 0x47, 0x2C, 0xFF,       // mov byte [bx+0x2c],0xff  | rm8 [amode+s8]
        0x8D, 0x36, 0x00, 0x01,       // lea si,[0x100]
        0x8B, 0x14,                   // mov dx,[si]  | rm16 [reg]
        0x8B, 0x47, 0x2C,             // mov ax,[bx+0x2c]  | rm16 [amode+s8]
        0x89, 0x87, 0x30, 0x00,       // mov [bx+0x0030],ax  | rm [amode+s16]
        0x89, 0x05,                   // mov [di],ax  | rm16 [amode]
        0xC6, 0x85, 0xAE, 0x06, 0xFE, // mov byte [di+0x6ae],0xfe  | rm8 [amode+s16]
        0x8A, 0x85, 0xAE, 0x06,       // mov al,[di+0x6ae]
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 9);
    assert_eq!("[085F:0100] BB0002     Mov16    bx, 0x0200
[085F:0103] C6472CFF   Mov8     byte [bx+0x2C], 0xFF
[085F:0107] 8D360001   Lea16    si, word [0x0100]
[085F:010B] 8B14       Mov16    dx, word [si]
[085F:010D] 8B472C     Mov16    ax, word [bx+0x2C]
[085F:0110] 89873000   Mov16    word [bx+0x0030], ax
[085F:0114] 8905       Mov16    word [di], ax
[085F:0116] C685AE06FE Mov8     byte [di+0x06AE], 0xFE
[085F:011B] 8A85AE06   Mov8     al, byte [di+0x06AE]
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.r16[BX].val);

    cpu.execute_instruction();
    let cs = cpu.sreg16[CS] as usize;
    assert_eq!(0xFF, cpu.peek_u8_at((cs * 16) + 0x22C));

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.r16[SI].val);

    cpu.execute_instruction();
    // should have read word at [0x100]
    assert_eq!(0x00BB, cpu.r16[DX].val);

    cpu.execute_instruction();
    // should have read word at [0x22C]
    assert_eq!(0x00FF, cpu.r16[AX].val);

    cpu.execute_instruction();
    // should have written word to [0x230]
    assert_eq!(0x00FF, cpu.peek_u16_at((cs * 16) + 0x230));

    cpu.execute_instruction();
    // should have written ax to [di]
    let di = cpu.r16[DI].val as usize;
    assert_eq!(0x00FF, cpu.peek_u16_at((cs * 16) + di));

    cpu.execute_instruction();
    // should have written byte to [di+0x06AE]
    assert_eq!(0xFE, cpu.peek_u8_at((cs * 16) + di + 0x06AE));

    cpu.execute_instruction();
    // should have read byte from [di+0x06AE] to al
    assert_eq!(0xFE, cpu.r16[AX].lo_u8());
}

#[test]
fn can_execute_math() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xF6, 0x06, 0x2C, 0x12, 0xFF, // test byte [0x122c],0xff
    ];

    cpu.load_com(&code);

    let res = cpu.disassemble_block(0x100, 1);
    assert_eq!("[085F:0100] F6062C12FF Test8    byte [0x122C], 0xFF
",
               res);

    // XXX also execute
}

#[test]
fn can_disassemble_basic() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xE8, 0x05, 0x00, // call l_0x108   ; call a later offset
        0xBA, 0x0B, 0x01, // mov dx,0x10b
        0xB4, 0x09,       // mov ah,0x9
        0xCD, 0x21,       // l_0x108: int 0x21
        0xE8, 0xFB, 0xFF, // call l_0x108   ; call an earlier offset
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 5);

    assert_eq!("[085F:0100] E80500     CallNear 0x0108
[085F:0103] BA0B01     Mov16    dx, 0x010B
[085F:0106] B409       Mov8     ah, 0x09
[085F:0108] CD21       Int      0x21
[085F:010A] E8FBFF     CallNear 0x0108
",
               res);
}

#[test]
fn can_disassemble_segment_prefixed() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x26, 0x88, 0x25, // mov [es:di],ah
        0x26, 0x8A, 0x25, // mov ah,[es:di]
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 2);

    assert_eq!("[085F:0100] 268825     Mov8     byte [es:di], ah
[085F:0103] 268A25     Mov8     ah, byte [es:di]
",
               res);
}

#[test]
fn can_disassemble_arithmetic() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x80, 0x3E, 0x31, 0x10, 0x00, // cmp byte [0x1031],0x0
        0x81, 0xC7, 0xC0, 0x00,       // add di,0xc0
        0x83, 0xC7, 0x3A,             // add di,byte +0x3a
        0x83, 0xC7, 0xC6,             // add di,byte -0x3a
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 4);

    assert_eq!("[085F:0100] 803E311000 Cmp8     byte [0x1031], 0x00
[085F:0105] 81C7C000   Add16    di, 0x00C0
[085F:0109] 83C73A     Add16    di, byte +0x3A
[085F:010C] 83C7C6     Add16    di, byte -0x3A
",
               res);
}

#[test]
fn can_disassemble_jz_rel() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x74, 0x04, // jz 0x106
        0x74, 0xFE, // jz 0x102
        0x74, 0x00, // jz 0x106
        0x74, 0xFA, // jz 0x102
    ];
    cpu.load_com(&code);
    let res = cpu.disassemble_block(0x100, 4);

    assert_eq!("[085F:0100] 7404       Jz       0x0106
[085F:0102] 74FE       Jz       0x0102
[085F:0104] 7400       Jz       0x0106
[085F:0106] 74FA       Jz       0x0102
",
               res);
}


#[bench]
fn exec_simple_loop(b: &mut Bencher) {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB9, 0xFF, 0xFF, // mov cx,0xffff
        0x49,             // dec cx
        0xEB, 0xFA,       // jmp short 0x100
    ];

    cpu.load_com(&code);

    b.iter(|| cpu.execute_instruction())
}

#[bench]
fn disasm_block(b: &mut Bencher) {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB9, 0xFF, 0xFF, // mov cx,0xffff
        0x49,             // dec cx
        0xEB, 0xFA,       // jmp short 0x100
    ];

    cpu.load_com(&code);

    b.iter(|| cpu.disassemble_block(0x100, 3))
}
