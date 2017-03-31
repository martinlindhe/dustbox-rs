#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

use std::fmt;
use std::process::exit;
use std::num::Wrapping;
use std::u8;

pub struct CPU {
    pub ip: u16,
    pub instruction_count: usize,
    memory: Vec<u8>,
    // 8 low = r16, 8 hi = es,cs,ss,ds,fs,gs
    r16: [Register16; 8], // general purpose registers
    sreg16: [Register16; 6], // es,cs,ss,ds,fs,gs
    flags: Flags,
    breakpoints: Vec<usize>,
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
    resume: bool, // 16: Resume flag (386+ only)
    virtual_mode: bool, // 17: Virtual 8086 mode flag (386+ only)
    alignment_check: bool, // 18: Alignment check (486SX+ only)
    virtual_interrupt: bool, // 19: Virtual interrupt flag (Pentium+)
    virtual_interrupt_pending: bool, // 20: Virtual interrupt pending (Pentium+)
    cpuid: bool, // 21: Able to use CPUID instruction (Pentium+)
                 // 22-31: reserved
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
}


#[derive(Copy, Clone)]
struct Register16 {
    val: u16,
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
    command: Op,
    segment: Segment,
    src: Parameter,
    dst: Parameter,
}

impl Instruction {
    fn describe(&self) -> String {
        match self.dst {
            Parameter::None() => format!("{:?}", self.command),
            _ => {
                let cmd = right_pad(&format!("{:?}", self.command), 9);
                match self.src {
                    Parameter::None() => format!("{}{}", cmd, self.dst),
                    _ => format!("{}{}, {}", cmd, self.dst, self.src),
                }
            }
        }
    }
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
    Ptr16Amode(Segment, usize), // word [amode], like "word [bx]"
    Reg8(usize), // index into the low 4 of CPU.r16
    Reg16(usize), // index into CPU.r16
    SReg16(usize), // index into cpu.sreg16
    None(),
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Parameter::Imm8(imm) => write!(f, "0x{:02X}", imm),
            &Parameter::Imm16(imm) => write!(f, "0x{:04X}", imm),
            &Parameter::ImmS8(imm) => {
                write!(f,
                       "byte {}0x{:02X}",
                       if imm < 0 { "-" } else { "+" },
                       if imm < 0 { -imm } else { imm })
            }
            &Parameter::Ptr8(seg, v) => write!(f, "byte [{}0x{:04X}]", seg, v),
            &Parameter::Ptr16(seg, v) => write!(f, "word [{}0x{:04X}]", seg, v),
            &Parameter::Ptr8Amode(seg, v) => write!(f, "byte [{}{}]", seg, amode(v as u8)),
            &Parameter::Ptr8AmodeS8(seg, v, imm) => {
                write!(f,
                       "byte [{}{}{}0x{:02X}]",
                       seg,
                       amode(v as u8),
                       if imm < 0 { "-" } else { "+" },
                       if imm < 0 { -imm } else { imm })
            }
            &Parameter::Ptr16Amode(seg, v) => write!(f, "word [{}{}]", seg, amode(v as u8)),
            &Parameter::Reg8(v) => write!(f, "{}", r8(v as u8)),
            &Parameter::Reg16(v) => write!(f, "{}", r16(v as u8)),
            &Parameter::SReg16(v) => write!(f, "{}", sr16(v as u8)),
            &Parameter::None() => write!(f, ""),
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Segment {
    DS(),
    ES(),
    None(),
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Segment::DS() => write!(f, "ds:"),
            &Segment::ES() => write!(f, "es:"),
            &Segment::None() => write!(f, ""),
        }
    }
}

#[derive(Debug)]
enum Op {
    Add8(),
    Add16(),
    And8(),
    CallNear(),
    Clc(),
    Cld(),
    Cli(),
    Cmp8(),
    Cmp16(),
    Daa(),
    Dec8(),
    Dec16(),
    In8(),
    Inc8(),
    Inc16(),
    Int(),
    Jc(),
    Jg(),
    JmpNear(),
    JmpShort(),
    Jna(),
    Jnl(),
    Jnz(),
    Jz(),
    Lea16(),
    Loop(),
    Mov8(),
    Mov16(),
    Or8(),
    Or16(),
    Out8(),
    Pop16(),
    Push16(),
    Rcl8(),
    Rcr8(),
    RepMovsb(),
    RepMovsw(),
    Retn(),
    Ror8(),
    Shr16(),
    Sti(),
    Stosb(),
    Sub8(),
    Sub16(),
    Xchg16(),
    Xor16(),
    Unknown(),
}

#[derive(Debug)]
pub struct InstructionInfo {
    pub offset: usize,
    pub length: usize,
    pub text: String,
    pub bytes: Vec<u8>,
    pub instruction: Instruction,
}

impl InstructionInfo {
    pub fn pretty_string(&self) -> String {
        let hex = self.to_hex_string(&self.bytes);
        format!("{:06X}: {} {}", self.offset, right_pad(&hex, 17), self.text)
    }

    fn to_hex_string(&self, bytes: &Vec<u8>) -> String {
        let strs: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
        strs.join(" ")
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
            sreg16: [Register16 { val: 0 }; 6],
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
                resume: false,
                virtual_mode: false,
                alignment_check: false,
                virtual_interrupt: false,
                virtual_interrupt_pending: false,
                cpuid: false,
            },
            breakpoints: vec![0; 0],
        };

        // intializes the cpu as if to run .com programs, info from
        // http://www.delorie.com/djgpp/doc/rbinter/id/51/29.html
        cpu.sreg16[SS].val = 0x0000;
        cpu.r16[SP].val = 0xFFFE; // offset of last word available in first 64k segment

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

    pub fn load_rom(&mut self, data: &Vec<u8>, offset: u16) {
        self.ip = offset;

        // copy up to 64k of rom
        let mut max = (offset as usize) + data.len();
        if max > 0x10000 {
            max = 0x10000;
        }
        let min = offset as usize;
        println!("loading rom to {:04X}..{:04X}", min, max);

        for i in min..max {
            let rom_pos = i - (offset as usize);
            self.memory[i] = data[rom_pos];
        }
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
               self.sreg16[ES].val,
               self.sreg16[CS].val,
               self.sreg16[SS].val,
               self.sreg16[DS].val,
               self.sreg16[FS].val,
               self.sreg16[GS].val);

        println!("");
    }

    pub fn execute_instruction(&mut self) -> bool {
        let op = self.decode_instruction(Segment::None()); // XXX should probably be cs?!
        self.execute(&op);

        match op.command {
            Op::Unknown() => {
                println!("HIT A UNKNOWN COMMAND");
                false
            }
            _ => true,
        }
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
        let op = self.decode_instruction(Segment::None()); // XXX should probably be cs?!
        let length = self.ip - old_ip;
        self.ip = old_ip;

        InstructionInfo {
            offset: old_ip as usize,
            length: length as usize,
            text: op.describe(),
            bytes: self.read_u8_slice(old_ip as usize, length as usize),
            instruction: op,
        }
    }

    fn execute(&mut self, op: &Instruction) {
        self.instruction_count += 1;
        match op.command {
            Op::Add8() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.src) as usize;
                let dst = self.read_parameter_value(&op.dst) as usize;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_carry_u8(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.dst, (res & 0xFF) as u8);
            }
            Op::Add16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.src) as usize;
                let dst = self.read_parameter_value(&op.dst) as usize;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, CF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_carry_u16(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u16(&op.dst, (res & 0xFFFF) as u16);
            }
            Op::And8() => {
                // two parameters (dst=reg)
                println!("XXX impl and8");
                // XXX flags
            }
            Op::CallNear() => {
                // call near rel
                let old_ip = self.ip;
                let temp_ip = self.read_parameter_value(&op.dst) as u16;
                self.push16(old_ip);
                self.ip = temp_ip;
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

                let src = self.read_parameter_value(&op.src) as usize;
                let dst = self.read_parameter_value(&op.dst) as usize;
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

                let src = self.read_parameter_value(&op.src) as usize;
                let dst = self.read_parameter_value(&op.dst) as usize;
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
                let dst = self.read_parameter_value(&op.dst) as usize;
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.flags.set_overflow_sub_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.dst, (res & 0xFF) as u8);
            }
            Op::Dec16() => {
                // single parameter (dst)
                let dst = self.read_parameter_value(&op.dst) as usize;
                let src = 1;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The CF flag is not affected. The OF, SF, ZF, AF,
                // and PF flags are set according to the result.
                self.flags.set_overflow_sub_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u16(&op.dst, (res & 0xFFFF) as u16);
            }
            Op::In8() => {
                // Input from Port
                // two parameters (dst=AL)
                let src = self.read_parameter_value(&op.src) as usize;
                println!("XXX unhandled in8 {:02X}", src);
            }
            Op::Inc8() => {
                let dst = self.read_parameter_value(&op.dst) as usize;
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u8(&op.dst, (res & 0xFF) as u8);
            }
            Op::Inc16() => {
                let dst = self.read_parameter_value(&op.dst) as usize;
                let src = 1;
                let res = (Wrapping(dst) + Wrapping(src)).0;

                // The OF, SF, ZF, AF, and PF flags are set according to the result.
                self.flags.set_overflow_add_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);

                self.write_parameter_u16(&op.dst, (res & 0xFFFF) as u16);
            }
            Op::Int() => {
                // XXX jump to offset 0x21 in interrupt table (look up how hw does this)
                // http://wiki.osdev.org/Interrupt_Vector_Table
                let int = self.read_parameter_value(&op.dst) as u8;
                println!("XXX IMPL: int {:02X}", int);
            }
            Op::Jc() => {
                // Jump short if carry (CF=1).
                if self.flags.carry {
                    self.ip = self.read_parameter_value(&op.dst) as u16;
                }
            }
            Op::Jg() => {
                // Jump short if greater (ZF=0 and SF=OF).
                if !self.flags.zero & self.flags.sign == self.flags.overflow {
                    self.ip = self.read_parameter_value(&op.dst) as u16;
                }
            }
            Op::JmpNear() => {
                self.ip = self.read_parameter_value(&op.dst) as u16;
            }
            Op::JmpShort() => {
                self.ip = self.read_parameter_value(&op.dst) as u16;
            }
            Op::Jna() => {
                // Jump short if not above (CF=1 or ZF=1).
                if self.flags.carry | self.flags.zero {
                    self.ip = self.read_parameter_value(&op.dst) as u16;
                }
            }
            Op::Jnl() => {
                // Jump short if not less (SF=OF).
                if self.flags.sign == self.flags.overflow {
                    self.ip = self.read_parameter_value(&op.dst) as u16;
                }
            }
            Op::Jnz() => {
                // Jump short if not zero (ZF=0).
                if !self.flags.zero {
                    self.ip = self.read_parameter_value(&op.dst) as u16;
                }
            }
            Op::Jz() => {
                // Jump short if zero (ZF ← 1).
                if self.flags.zero {
                    self.ip = self.read_parameter_value(&op.dst) as u16;
                }
            }
            Op::Lea16() => {
                // Load Effective Address
                // Store effective address for m in register r16
                let src = self.read_parameter_address(&op.src) as u16;
                self.write_parameter_u16(&op.dst, src);
            }
            Op::Loop() => {
                let dst = self.read_parameter_value(&op.dst) as u16;
                self.r16[CX].val -= 1;
                if self.r16[CX].val != 0 {
                    self.ip = dst;
                }
                // No flags affected.
            }
            Op::Mov8() => {
                // two parameters (dst=reg)
                let data = self.read_parameter_value(&op.src) as u8;
                self.write_parameter_u8(&op.dst, data);
            }
            Op::Mov16() => {
                // two parameters (dst=reg)
                let data = self.read_parameter_value(&op.src) as u16;
                self.write_parameter_u16(&op.dst, data);
            }
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
                let data = self.read_parameter_value(&op.src) as u8;
                self.out_u8(&op.dst, data);
            }
            Op::Pop16() => {
                // single parameter (dst)
                let data = self.pop16();
                self.write_parameter_u16(&op.dst, data);
            }
            Op::Push16() => {
                // single parameter (dst)
                let data = self.read_parameter_value(&op.dst) as u16;
                self.push16(data);
            }
            Op::Rcl8() => {
                // two arguments
                // rotate 9 bits `src` times
                let src = self.read_parameter_value(&op.src) as u8;
                let dst = self.read_parameter_value(&op.dst) as u8;

                // XXX do + flags + write result
                println!("XXX impl rcl8");
            }
            Op::Rcr8() => {
                // two arguments
                // rotate 9 bits `src` times
                let src = self.read_parameter_value(&op.src) as u8;
                let dst = self.read_parameter_value(&op.dst) as u8;

                // XXX do + flags + write result

                // The RCR instruction shifts the CF flag into the most-significant
                // bit and shifts the least-significant bit into the CF flag.
                // The OF flag is affected only for single-bit rotates; it is undefined
                // for multi-bit rotates. The SF, ZF, AF, and PF flags are not affected.
                println!("XXX impl rcr8");
            }
            Op::RepMovsb() => {
                // Move (E)CX bytes from DS:[(E)SI] to ES:[(E)DI].
                let mut src = (self.sreg16[DS].val as usize) * 16 + (self.r16[SI].val as usize);
                let mut dst = (self.sreg16[ES].val as usize) * 16 + (self.r16[DI].val as usize);
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
                let mut src = (self.sreg16[DS].val as usize) * 16 + (self.r16[SI].val as usize);
                let mut dst = (self.sreg16[ES].val as usize) * 16 + (self.r16[DI].val as usize);
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
            Op::Retn() => {
                // ret near (no arguments)
                self.ip = self.pop16();
            }
            Op::Ror8() => {
                // two arguments
                println!("XXX impl ror8");
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
                let offset = (self.sreg16[ES].val as usize) * 16 + (self.r16[DI].val as usize);
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
                let src = self.read_parameter_value(&op.src) as usize;
                let dst = self.read_parameter_value(&op.dst) as usize;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.flags.set_overflow_sub_u8(res, src, dst);
                self.flags.set_sign_u8(res);
                self.flags.set_zero_u8(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
                self.flags.set_carry_u8(res, src, dst);

                self.write_parameter_u8(&op.dst, (res & 0xFF) as u8);
            }
            Op::Sub16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.src) as usize;
                let dst = self.read_parameter_value(&op.dst) as usize;
                let res = (Wrapping(dst) - Wrapping(src)).0;

                // The OF, SF, ZF, AF, PF, and CF flags are set according to the result.
                self.flags.set_overflow_sub_u16(res, src, dst);
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_auxiliary(res, src, dst);
                self.flags.set_parity(res);
                self.flags.set_carry_u16(res, src, dst);

                self.write_parameter_u16(&op.dst, (res & 0xFFFF) as u16);
            }
            Op::Xchg16() => {
                // two parameters
                println!("XXX impl xchg16");
            }
            Op::Xor16() => {
                // two parameters (dst=reg)
                let src = self.read_parameter_value(&op.src) as usize;
                let dst = self.read_parameter_value(&op.dst) as usize;
                let res = dst ^ src;

                // The OF and CF flags are cleared; the SF, ZF,
                // and PF flags are set according to the result.
                self.flags.overflow = false;
                self.flags.carry = false;
                self.flags.set_sign_u16(res);
                self.flags.set_zero_u16(res);
                self.flags.set_parity(res);

                self.write_parameter_u16(&op.dst, (res & 0xFFFF) as u16);
            }
            _ => {
                println!("execute error: unhandled: {:?} at {:04X}",
                         op.command,
                         self.ip);
            }
        }
    }

    fn decode_instruction(&mut self, seg: Segment) -> Instruction {
        let b = self.read_u8();
        let mut p = Instruction {
            segment: seg,
            command: Op::Unknown(),
            dst: Parameter::None(),
            src: Parameter::None(),
        };

        match b {
            0x03 => {
                // add r16, r/m16
                let part = self.r16_rm16(p.segment);
                p.command = Op::Add16();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x04 => {
                // add AL, imm8
                p.command = Op::Add8();
                p.dst = Parameter::Reg8(AL);
                p.src = Parameter::Imm8(self.read_u8());
                p
            }
            0x05 => {
                // add AX, imm16
                p.command = Op::Add16();
                p.dst = Parameter::Reg16(AX);
                p.src = Parameter::Imm16(self.read_u16());
                p
            }
            0x06 => {
                // push es
                p.command = Op::Push16();
                p.dst = Parameter::SReg16(ES);
                p
            }
            0x07 => {
                // pop es
                p.command = Op::Pop16();
                p.dst = Parameter::SReg16(ES);
                p
            }
            0x0C => {
                // or AL, imm8
                p.command = Op::Or8();
                p.dst = Parameter::Reg8(AL);
                p.src = Parameter::Imm8(self.read_u8());
                p
            }
            0x0D => {
                // or AX, imm16
                p.command = Op::Or16();
                p.dst = Parameter::Reg16(AX);
                p.src = Parameter::Imm16(self.read_u16());
                p
            }
            0x1E => {
                // push ds
                p.command = Op::Push16();
                p.dst = Parameter::SReg16(DS);
                p
            }
            0x1F => {
                // pop ds
                p.command = Op::Pop16();
                p.dst = Parameter::SReg16(DS);
                p
            }
            0x24 => {
                // and AL, imm8
                p.command = Op::And8();
                p.dst = Parameter::Reg8(AL);
                p.src = Parameter::Imm8(self.read_u8());
                p
            }
            0x26 => {
                // es segment prefix
                self.decode_instruction(Segment::ES())
            }
            0x27 => {
                // daa
                p.command = Op::Daa();
                p
            }
            0x2C => {
                // sub AL, imm8
                p.command = Op::Sub8();
                p.dst = Parameter::Reg8(AL);
                p.src = Parameter::Imm8(self.read_u8());
                p
            }
            0x31 => {
                // xor r/m16, r16
                let part = self.rm16_r16(p.segment);
                p.command = Op::Xor16();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x33 => {
                // xor r16, r/m16
                let part = self.r16_rm16(p.segment);
                p.command = Op::Xor16();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x3A => {
                // cmp r8, r/m8
                let part = self.r8_rm8(p.segment);
                p.command = Op::Cmp8();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x3C => {
                // cmp AL, imm8
                p.command = Op::Cmp8();
                p.dst = Parameter::Reg8(AL);
                p.src = Parameter::Imm8(self.read_u8());
                p
            }
            0x3E => {
                // ds segment prefix
                // XXX if next op is a Jcc, then this is a "branch taken" hint
                self.decode_instruction(Segment::DS())
            }
            0x40...0x47 => {
                // inc r16
                p.command = Op::Inc16();
                p.dst = Parameter::Reg16((b & 7) as usize);
                p
            }
            0x48...0x4F => {
                // dec r16
                p.command = Op::Dec16();
                p.dst = Parameter::Reg16((b & 7) as usize);
                p
            }
            0x50...0x57 => {
                // push r16
                p.command = Op::Push16();
                p.dst = Parameter::Reg16((b & 7) as usize);
                p
            }
            0x58...0x5F => {
                // pop r16
                p.command = Op::Pop16();
                p.dst = Parameter::Reg16((b & 7) as usize);
                p
            }
            0x72 => {
                // jc rel8    (alias: jb, jnae)
                p.command = Op::Jc();
                p.dst = Parameter::Imm16(self.read_rel8());
                p
            }

            0x74 => {
                // jz rel8    (alias: je)
                p.command = Op::Jz();
                p.dst = Parameter::Imm16(self.read_rel8());
                p
            }
            0x75 => {
                // jnz rel8   (alias: jne)
                p.command = Op::Jnz();
                p.dst = Parameter::Imm16(self.read_rel8());
                p
            }
            0x76 => {
                // jna rel8    (alias: jbe)
                p.command = Op::Jna();
                p.dst = Parameter::Imm16(self.read_rel8());
                p
            }
            0x7D => {
                // jnl rel8    (alias: jge)
                p.command = Op::Jnl();
                p.dst = Parameter::Imm16(self.read_rel8());
                p
            }
            0x7F => {
                // jg rel8    (alias: jnle)
                p.command = Op::Jg();
                p.dst = Parameter::Imm16(self.read_rel8());
                p
            }
            0x80 => {
                // arithmetic 8-bit
                self.decode_80(p.segment)
            }
            0x81 => {
                // arithmetic 16-bit
                self.decode_81(p.segment)
            }
            0x83 => {
                // arithmetic 16-bit with signed 8-bit value
                self.decode_83(p.segment)
            }
            0x87 => {
                // xchg r/m16, r16 | xchg r16, r/m16
                let part = self.rm16_r16(p.segment);
                p.command = Op::Xchg16();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x88 => {
                // mov r/m8, r8
                let part = self.rm8_r8(p.segment);
                p.command = Op::Mov8();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x89 => {
                // mov r/m16, r16
                let part = self.rm16_r16(p.segment);
                p.command = Op::Mov16();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x8A => {
                // mov r8, r/m8
                let part = self.r8_rm8(p.segment);
                p.command = Op::Mov8();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x8B => {
                // mov r16, r/m16
                let part = self.r16_rm16(p.segment);
                p.command = Op::Mov16();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x8C => {
                // mov r/m16, sreg
                let part = self.rm16_sreg(p.segment);
                p.command = Op::Mov16();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x8D => {
                // lea r16, m
                let part = self.r16_m16(p.segment);
                p.command = Op::Lea16();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0x8E => {
                // mov sreg, r/m16
                let part = self.sreg_rm16(p.segment);
                p.command = Op::Mov16();
                p.dst = part.dst;
                p.src = part.src;
                p
            }
            0xA0 => {
                // mov AL, moffs8
                p.command = Op::Mov8();
                p.dst = Parameter::Reg8(AL);
                p.src = Parameter::Ptr8(p.segment, self.read_u16());
                p
            }
            0xA2 => {
                // mov moffs8,AL
                p.command = Op::Mov8();
                p.dst = Parameter::Ptr8(p.segment, self.read_u16());
                p.src = Parameter::Reg8(AL);
                p
            }
            0xA3 => {
                // mov moffs16, AX
                p.command = Op::Mov16();
                p.dst = Parameter::Ptr16(p.segment, self.read_u16());
                p.src = Parameter::Reg16(AX);
                p
            }
            0xAA => {
                // stosb
                p.command = Op::Stosb();
                p
            }
            0xB0...0xB7 => {
                // mov r8, u8
                p.command = Op::Mov8();
                p.dst = Parameter::Reg8((b & 7) as usize);
                p.src = Parameter::Imm8(self.read_u8());
                p
            }
            0xB8...0xBF => {
                // mov r16, u16
                p.command = Op::Mov16();
                p.dst = Parameter::Reg16((b & 7) as usize);
                p.src = Parameter::Imm16(self.read_u16());
                p
            }
            0xC3 => {
                // ret [near]
                p.command = Op::Retn();
                p
            }
            0xC6 => {
                // mov r/m8, imm8
                let x = self.read_mod_reg_rm();
                if x.reg != 0 {
                    // should be 0
                    println!("XXX ERROR C6 reg = {}", x.reg);
                }
                p.command = Op::Mov8();
                p.dst = self.rm8(p.segment, x.rm, x.md);
                p.src = Parameter::Imm8(self.read_u8());
                p
            }
            0xC7 => {
                // mov r/m16, imm16
                let x = self.read_mod_reg_rm();
                if x.reg != 0 {
                    // should be 0
                    println!("XXX ERROR C7 reg = {}", x.reg);
                }
                p.command = Op::Mov16();
                p.dst = self.rm16(p.segment, x.rm, x.md);
                p.src = Parameter::Imm16(self.read_u16());
                p
            }
            0xCD => {
                p.command = Op::Int();
                p.dst = Parameter::Imm8(self.read_u8());
                p
            }
            0xD0 => {
                // bit shift byte
                let x = self.read_mod_reg_rm();
                p.command = match x.reg {
                    // 0 => Op::Rol8(),
                    1 => Op::Ror8(),
                    2 => Op::Rcl8(),
                    3 => Op::Rcr8(),
                    // 4 => Op::Shl8(), // alias: sal
                    // 5 => Op::Shr8(),
                    // 7 => Op::Sar8(),
                    _ => {
                        println!("XXX 0xD0 unhandled reg = {}", x.reg);
                        Op::Unknown()
                    }
                };
                p.dst = self.rm8(p.segment, x.rm, x.md);
                p.src = Parameter::Imm8(1);
                p
            }
            0xD1 => {
                // bit shift word
                let x = self.read_mod_reg_rm();
                p.command = match x.reg {
                    // 0 => Op::Rol16(),
                    // 1 => Op::Ror16(),
                    //2 => Op::Rcl16(),
                    //3 => Op::Rcr16(),
                    // 4 => Op::Shl16(), // alias: sal
                    5 => Op::Shr16(),
                    // 7 => Op::Sar16(),
                    _ => {
                        println!("XXX 0xD1 unhandled reg = {}", x.reg);
                        Op::Unknown()
                    }
                };
                p.dst = self.rm8(p.segment, x.rm, x.md);
                p.src = Parameter::Imm16(1);
                p
            }
            0xE2 => {
                // loop rel8
                p.command = Op::Loop();
                p.dst = Parameter::Imm16(self.read_rel8());
                p
            }
            0xE4 => {
                // in AL, imm8
                p.command = Op::In8();
                p.dst = Parameter::Reg8(AL);
                p.src = Parameter::Imm8(self.read_u8());
                p
            }
            0xE6 => {
                // OUT imm8, AL
                p.command = Op::Out8();
                p.dst = Parameter::Imm8(self.read_u8());
                p.src = Parameter::Reg8(AL);
                p
            }
            0xE8 => {
                // call near s16
                p.command = Op::CallNear();
                p.dst = Parameter::Imm16(self.read_rel16());
                p
            }
            0xE9 => {
                // jmp near rel16
                p.command = Op::JmpNear();
                p.dst = Parameter::Imm16(self.read_rel16());
                p
            }
            0xEB => {
                // jmp short rel8
                p.command = Op::JmpShort();
                p.dst = Parameter::Imm16(self.read_rel8());
                p
            }
            0xEE => {
                p.command = Op::Out8();
                p.dst = Parameter::Reg16(DX);
                p.src = Parameter::Reg8(AL);
                p
            }
            0xF3 => {
                // rep
                self.decode_f3(p.segment)
            }
            0xF8 => {
                // clc
                p.command = Op::Clc();
                p
            }
            0xFA => {
                // cli
                p.command = Op::Cli();
                p
            }
            0xFB => {
                // sti
                p.command = Op::Sti();
                p
            }
            0xFC => {
                // cld
                p.command = Op::Cld();
                p
            }
            0xFE => {
                // byte size
                self.decode_fe(p.segment)
            }
            0xFF => {
                // word size
                self.decode_ff(p.segment)
            }
            _ => {
                println!("cpu: unknown op {:02X} at {:04X}", b, self.ip - 1);
                p
            }
        }
    }

    // arithmetic 8-bit
    fn decode_80(&mut self, seg: Segment) -> Instruction {
        let x = self.read_mod_reg_rm();
        let mut p = Instruction {
            segment: seg,
            command: Op::Unknown(),
            dst: self.rm8(seg, x.rm, x.md),
            src: Parameter::Imm8(self.read_u8()),
        };

        match x.reg {
            0 => {
                p.command = Op::Add8();
            }
            4 => {
                p.command = Op::And8();
            }
            7 => {
                p.command = Op::Cmp8();
            }
            /*
            5 => {
                p.command = Op::Sub8();
            }
            case 0:
                op.Cmd = "add"
            case 1:
                op.Cmd = "or"
            case 2:
                op.Cmd = "adc"
            case 3:
                op.Cmd = "sbb"
            case 6:
                op.Cmd = "xor"
            }*/
            _ => {
                println!("decode_80 error: unknown reg {}", x.reg);
                p.command = Op::Unknown();
            }
        }
        p
    }

    // arithmetic 16-bit with signed 8-bit value
    fn decode_83(&mut self, seg: Segment) -> Instruction {
        let x = self.read_mod_reg_rm();
        let mut p = Instruction {
            segment: seg,
            command: Op::Unknown(),
            dst: self.rm16(seg, x.rm, x.md),
            src: Parameter::ImmS8(self.read_s8()),
        };

        match x.reg {
            0 => {
                p.command = Op::Add16();
            }
            5 => {
                p.command = Op::Sub16();
            }
            7 => {
                p.command = Op::Cmp16();
            }
            /*
            case 1:
                op.Cmd = "or"
            case 2:
                op.Cmd = "adc"
            case 3:
                op.Cmd = "sbb"
            case 4:
                op.Cmd = "and"
            case 6:
                op.Cmd = "xor"
            }*/
            _ => {
                println!("decode_83 error: unknown reg {}", x.reg);
                p.command = Op::Unknown();
            }
        }
        p
    }

    // arithmetic 16-bit
    fn decode_81(&mut self, seg: Segment) -> Instruction {
        let x = self.read_mod_reg_rm();
        let mut p = Instruction {
            segment: seg,
            command: Op::Unknown(),
            dst: self.rm16(seg, x.rm, x.md),
            src: Parameter::Imm16(self.read_u16()),
        };

        match x.reg {
            0 => {
                p.command = Op::Add16();
            }
            5 => {
                p.command = Op::Sub16();
            }
            7 => {
                p.command = Op::Cmp16();
            }
            /*
            case 1:
                op.Cmd = "or"
            case 2:
                op.Cmd = "adc"
            case 3:
                op.Cmd = "sbb"
            case 4:
                op.Cmd = "and"
            case 6:
                op.Cmd = "xor"
            }*/
            _ => {
                println!("decode_81 error: unknown reg {}", x.reg);
                p.command = Op::Unknown();
            }
        }
        p
    }

    // rep
    fn decode_f3(&mut self, seg: Segment) -> Instruction {
        let mut p = Instruction {
            segment: seg,
            command: Op::Unknown(),
            dst: Parameter::None(),
            src: Parameter::None(),
        };
        let b = self.read_u8();
        match b {
            0xA4 => {
                // rep movs byte
                p.command = Op::RepMovsb();
            }
            0xA5 => {
                // rep movs word
                p.command = Op::RepMovsw();
            }
            _ => {
                println!("decode_f3 error: unhandled op {:02X}", b);
            }
        }
        p
    }

    // byte size
    fn decode_fe(&mut self, seg: Segment) -> Instruction {
        let x = self.read_mod_reg_rm();
        let mut p = Instruction {
            segment: seg,
            command: Op::Unknown(),
            dst: self.rm8(seg, x.rm, x.md),
            src: Parameter::None(),
        };
        match x.reg {
            0 => {
                p.command = Op::Inc8();
            }
            1 => {
                p.command = Op::Dec8();
            }
            _ => {
                println!("decode_fe error: unknown reg {}", x.reg);
            }
        }
        p
    }

    // word size
    fn decode_ff(&mut self, seg: Segment) -> Instruction {
        let x = self.read_mod_reg_rm();
        let mut p = Instruction {
            segment: seg,
            command: Op::Unknown(),
            dst: self.rm16(seg, x.rm, x.md),
            src: Parameter::None(),
        };
        match x.reg {
            0 => {
                p.command = Op::Inc16();
            }
            1 => {
                p.command = Op::Dec16();
            }
            _ => {
                println!("decode_ff error: unknown reg {}", x.reg);
            }
        }
        p
    }

    // decode r8, r/m8
    fn r8_rm8(&mut self, seg: Segment) -> Instruction {
        let mut res = self.rm8_r8(seg);
        let tmp = res.src;
        res.src = res.dst;
        res.dst = tmp;
        res
    }

    // decode r/m8, r8
    fn rm8_r8(&mut self, seg: Segment) -> Instruction {
        let x = self.read_mod_reg_rm();
        Instruction {
            segment: seg,
            command: Op::Unknown(),
            src: Parameter::Reg8(x.reg as usize),
            dst: self.rm8(seg, x.rm, x.md),
        }
    }

    // decode Sreg, r/m16
    fn sreg_rm16(&mut self, seg: Segment) -> Instruction {
        let mut res = self.rm16_sreg(seg);
        let tmp = res.src;
        res.src = res.dst;
        res.dst = tmp;
        res
    }

    // decode r/m16, Sreg
    fn rm16_sreg(&mut self, seg: Segment) -> Instruction {
        let x = self.read_mod_reg_rm();
        Instruction {
            segment: seg,
            command: Op::Unknown(),
            src: Parameter::SReg16(x.reg as usize),
            dst: self.rm16(seg, x.rm, x.md),
        }
    }

    // decode r16, r/m16
    fn r16_rm16(&mut self, seg: Segment) -> Instruction {
        let mut res = self.rm16_r16(seg);
        let tmp = res.src;
        res.src = res.dst;
        res.dst = tmp;
        res
    }

    // decode r/m16, r16
    fn rm16_r16(&mut self, seg: Segment) -> Instruction {
        let x = self.read_mod_reg_rm();
        Instruction {
            segment: seg,
            command: Op::Unknown(),
            src: Parameter::Reg16(x.reg as usize),
            dst: self.rm16(seg, x.rm, x.md),
        }
    }

    // decode r16, m16
    fn r16_m16(&mut self, seg: Segment) -> Instruction {
        let x = self.read_mod_reg_rm();
        if x.md == 3 {
            println!("r16_m16 error: invalid encoding, ip={:04X}", self.ip);
        }
        Instruction {
            segment: seg,
            command: Op::Unknown(),
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
            // [reg+d8]
            1 => Parameter::Ptr8AmodeS8(seg, rm as usize, self.read_s8()),
            // [reg+d16]
            2 => {
                println!("XXX FIXME rm8 broken [reg+d16], ip={:04X}", self.ip);
                let pos = self.read_s16() as u16; // XXX handle signed properly

                // XXX new type PtrAmode8 + s16
                exit(0);
                Parameter::Ptr8(seg, pos)
            }
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
            // [reg+d8]
            1 => {
                // XXX read value of amode(x.rm) into pos
                println!("XXX FIXME broken rm16 [reg+d8], ip={:04X}", self.ip);
                let pos = self.read_s8() as u16; // XXX handle signed properly

                exit(0); // XXX new type ptr16Amode + s8
                Parameter::Ptr16(seg, pos)
            }
            // [reg+d16]
            2 => {
                // XXX read value of amode(x.rm) into pos
                println!("XXX FIXME rm16 [reg+d16], ip={:04X}", self.ip);
                let pos = self.read_s16() as u16; // XXX handle signed properly

                exit(0); // XXX new type ptr16Amode + s16
                Parameter::Ptr16(seg, pos)
            }
            // [reg]
            _ => Parameter::Reg16(rm as usize),
        }
    }

    fn push16(&mut self, data: u16) {
        self.r16[SP].val -= 2;
        let offset = (self.sreg16[SS].val as usize) * 16 + (self.r16[SP].val as usize);
        /*
        println!("push16 {:04X}  to {:04X}:{:04X}  =>  {:06X}       instr {}",
                 data,
                 self.sreg16[SS].val,
                 self.r16[SP].val,
                 offset,
                 self.instruction_count);
        */
        self.write_u16(offset, data);
    }

    fn pop16(&mut self) -> u16 {
        let offset = (self.sreg16[SS].val as usize) * 16 + (self.r16[SP].val as usize);
        let data = self.peek_u16_at(offset);
        /*
        println!("pop16 {:04X}  from {:04X}:{:04X}  =>  {:06X}       instr {}",
                 data,
                 self.sreg16[SS].val,
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
        (self.sreg16[CS].val as usize) + self.ip as usize
    }

    fn read_u8(&mut self) -> u8 {
        let offset = self.get_offset();
        let b = self.memory[offset];
        /*
        println!("___ DBG: read u8 {:02X} from {:06X} ... {:04X}:{:04X}",
              b,
              offset,
              self.sreg16[CS].val,
              self.ip);
        */
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

    // output byte to I/O port
    fn out_u8(&mut self, p: &Parameter, data: u8) {
        let dst = match *p {
            Parameter::Reg16(r) => self.r16[r].val,
            _ => {
                println!("out_u8 unhandled type {:?}", p);
                0
            }
        };

        println!("XXX unhandled out_u8 to {:04X}, data {:02X}", dst, data);
    }

    fn read_parameter_address(&mut self, p: &Parameter) -> usize {
        match p {
            &Parameter::Ptr16(seg, imm) => (self.segment(seg) as usize * 16) + imm as usize,
            _ => {
                println!("read_parameter_address error: unhandled parameter: {:?}", p);
                0
            }
        }
    }

    fn read_parameter_value(&mut self, p: &Parameter) -> isize {
        match p {
            &Parameter::Imm8(imm) => imm as isize,
            &Parameter::Imm16(imm) => imm as isize,
            &Parameter::ImmS8(imm) => imm as isize,
            &Parameter::Ptr8(seg, imm) => {
                let offset = (self.segment(seg) as usize * 16) + imm as usize;
                self.peek_u8_at(offset) as isize
            }
            &Parameter::Ptr16(seg, imm) => {
                let offset = (self.segment(seg) as usize * 16) + imm as usize;
                self.peek_u16_at(offset) as isize
            }
            &Parameter::Ptr8Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.peek_u8_at(offset) as isize
            }
            &Parameter::Ptr16Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.peek_u16_at(offset) as isize
            }
            &Parameter::Reg8(r) => {
                let lor = r & 3;
                if r & 4 == 0 {
                    self.r16[lor].lo_u8() as isize
                } else {
                    self.r16[lor].hi_u8() as isize
                }
            }
            &Parameter::Reg16(r) => self.r16[r].val as isize,
            &Parameter::SReg16(r) => self.sreg16[r].val as isize,
            _ => {
                println!("read_parameter_value error: unhandled parameter: {:?}", p);
                0
            }
        }
    }

    fn write_parameter_u8(&mut self, p: &Parameter, data: u8) {
        match p {
            &Parameter::Reg8(r) => {
                let lor = r & 3;
                if r & 4 == 0 {
                    self.r16[lor].set_lo(data);
                } else {
                    self.r16[lor].set_hi(data);
                }
            }
            &Parameter::Ptr8(seg, imm) => {
                let offset = (self.segment(seg) as usize * 16) + imm as usize;
                self.write_u8(offset, data);
            }
            &Parameter::Ptr8Amode(seg, r) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r);
                self.write_u8(offset, data);
            }
            &Parameter::Ptr8AmodeS8(seg, r, imm) => {
                let offset = (self.segment(seg) as usize * 16) + self.amode16(r) + imm as usize;
                self.write_u8(offset, data);
            }
            _ => {
                println!("write_parameter_u8 unhandled type {:?}", p);
            }
        }
    }

    fn write_parameter_u16(&mut self, p: &Parameter, data: u16) {
        match *p {
            Parameter::Reg16(r) => {
                self.r16[r].val = data;
            }
            Parameter::SReg16(r) => {
                self.sreg16[r].val = data;
            }
            Parameter::Imm16(v) => {
                self.write_u16(v as usize, data);
            }
            Parameter::Ptr16(seg, imm) => {
                let offset = (self.segment(seg) as usize * 16) + imm as usize;
                self.write_u16(offset, data);
            }
            _ => {
                println!("write_u16_param unhandled type {:?}", p);
            }
        }
    }

    fn write_u8(&mut self, offset: usize, data: u8) {
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
            Segment::DS() => self.sreg16[DS].val,
            Segment::ES() => self.sreg16[ES].val,
            Segment::None() => 0,
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
    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction(); // mov
    cpu.execute_instruction(); // mov

    assert_eq!(0xFFFE, cpu.r16[SP].val);
    cpu.execute_instruction(); // push
    assert_eq!(0xFFFC, cpu.r16[SP].val);
    cpu.execute_instruction(); // pop
    assert_eq!(0xFFFE, cpu.r16[SP].val);

    assert_eq!(0x107, cpu.ip);
    assert_eq!(0x8888, cpu.r16[AX].val);
    assert_eq!(0x8888, cpu.sreg16[DS].val);
    assert_eq!(0x8888, cpu.sreg16[ES].val);
}

#[test]
fn can_execute_mov_r8() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xB2, 0x13, // mov dl,0x13
        0x88, 0xD0, // mov al,dl
    ];
    cpu.load_rom(&code, 0x100);

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

    cpu.load_rom(&code, 0x100);

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
    cpu.load_rom(&code, 0x100);

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
    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x123, cpu.r16[CX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x123, cpu.sreg16[ES].val);
}

#[test]
fn can_execute_mov_rm16_sreg() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x34, 0x12,       // mov bx,0x1234
        0x8E, 0xC3,             // mov es,bx
        0x8C, 0x06, 0x09, 0x01, // mov [0x109],es  | r/m16, sreg
    ];
    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x1234, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x1234, cpu.sreg16[ES].val);

    cpu.execute_instruction();
    assert_eq!(0x109, cpu.ip);
    assert_eq!(0x1234, cpu.peek_u16_at(0x0109));
}

#[test]
fn can_execute_mov_data() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xC6, 0x06, 0x31, 0x10, 0x38,       // mov byte [0x1031],0x38
    ];
    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x38, cpu.peek_u8_at(0x1031));
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

    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction();
    assert_eq!(0x103, cpu.ip);
    assert_eq!(0x1234, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0x105, cpu.ip);
    assert_eq!(0x1234, cpu.sreg16[ES].val);

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

    cpu.load_rom(&code, 0x100);

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

    cpu.load_rom(&code, 0x100);

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

    cpu.load_rom(&code, 0x100);

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
fn can_execute_lea() {
    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0x8D, 0x26, 0xF0, 0x1B, // lea sp,[0x1bf0]
    ];

    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction();
    assert_eq!(0x104, cpu.ip);
    assert_eq!(0x1BF0, cpu.r16[SP].val);
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

    cpu.load_rom(&code, 0x100);

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.r16[SI].val);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.r16[DI].val);

    cpu.execute_instruction();
    assert_eq!(0x5, cpu.r16[CX].val);

    cpu.execute_instruction(); // rep movsb
    assert_eq!(0x0, cpu.r16[CX].val);
    for i in 0x100..0x105 {
        assert_eq!(cpu.memory[i], cpu.memory[i + 0x100]);
    }
}

#[test]
fn can_execute_addressing() {

    let mut cpu = CPU::new();
    let code: Vec<u8> = vec![
        0xBB, 0x00, 0x02,       // mov bx,0x200
        0xC6, 0x47, 0x2C, 0xFF, // mov byte [bx+0x2c],0xff  | rm8 [amode+s8]
        0x8D, 0x36, 0x00, 0x01, // lea si,[0x100]
        0x8B, 0x14,             // mov dx,[si]  | rm16 [reg]
    ];

    cpu.load_rom(&code, 0x100);

    let res = cpu.disassemble_block(0x100, 4);
    assert_eq!("000100: BB 00 02          Mov16    bx, 0x0200
000103: C6 47 2C FF       Mov8     byte [bx+0x2C], 0xFF
000107: 8D 36 00 01       Lea16    si, word [0x0100]
00010B: 8B 14             Mov16    dx, word [si]
",
               res);

    cpu.execute_instruction();
    assert_eq!(0x200, cpu.r16[BX].val);

    cpu.execute_instruction();
    assert_eq!(0xFF, cpu.memory[0x22C]);

    cpu.execute_instruction();
    assert_eq!(0x100, cpu.r16[SI].val);

    cpu.execute_instruction();
    // should have read word at [0x100]
    assert_eq!(0x00BB, cpu.r16[DX].val);
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
    cpu.load_rom(&code, 0x100);
    let res = cpu.disassemble_block(0x100, 5);

    assert_eq!("000100: E8 05 00          CallNear 0x0108
000103: BA 0B 01          Mov16    dx, 0x010B
000106: B4 09             Mov8     ah, 0x09
000108: CD 21             Int      0x21
00010A: E8 FB FF          CallNear 0x0108
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
    cpu.load_rom(&code, 0x100);
    let res = cpu.disassemble_block(0x100, 2);

    assert_eq!("000100: 26 88 25          Mov8     byte [es:di], ah
000103: 26 8A 25          Mov8     ah, byte [es:di]
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
    cpu.load_rom(&code, 0x100);
    let res = cpu.disassemble_block(0x100, 4);

    assert_eq!("000100: 80 3E 31 10 00    Cmp8     byte [0x1031], 0x00
000105: 81 C7 C0 00       Add16    di, 0x00C0
000109: 83 C7 3A          Add16    di, byte +0x3A
00010C: 83 C7 C6          Add16    di, byte -0x3A
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
    cpu.load_rom(&code, 0x100);
    let res = cpu.disassemble_block(0x100, 4);

    assert_eq!("000100: 74 04             Jz       0x0106
000102: 74 FE             Jz       0x0102
000104: 74 00             Jz       0x0106
000106: 74 FA             Jz       0x0102
",
               res);
}
