use std::fmt;
use std::num::Wrapping;

use segment::Segment;

// translates a segment:offset address into a flat address
pub fn seg_offs_as_flat(segment: u16, offset: u16) -> usize {
    (segment as usize * 16) + offset as usize
}

#[derive(Copy, Clone, Debug)]
pub enum RepeatMode {
    None,
    Rep,
    Repne, // (alias repnz)
}

impl RepeatMode {
    fn as_str(&self) -> &str {
        match *self {
            RepeatMode::None => "",
            RepeatMode::Rep => "Rep ",
            RepeatMode::Repne => "Repne ",
        }
    }
}

#[derive(Debug)]
pub struct Instruction {
    pub command: Op,
    pub segment: Segment,
    pub params: ParameterPair,
    pub length: u8,
    pub repeat: RepeatMode, // REPcc prefix
    pub lock: bool,         // LOCK prefix
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let instr = self.describe_instruction();
        if self.segment == Segment::Default || self.hide_segment_prefix() {
            write!(f, "{}", instr)
        } else {
            write!(f, "{} {}", self.segment.as_str(), instr)
        }
    }
}

impl Instruction {
    fn hide_segment_prefix(&self) -> bool {
        self.command == Op::Mov8() ||
        self.command == Op::Mov16()
    }

    fn describe_instruction(&self) -> String {
        let prefix = self.repeat.as_str();

        match self.params.dst {
            Parameter::None() => format!("{}{:?}", prefix, self.command),
            _ => {
                let cmd = right_pad(&format!("{}{:?}", prefix, self.command), 9);

                match self.params.src2 {
                    Parameter::None() => match self.params.src {
                        Parameter::None() => format!("{}{}", cmd, self.params.dst),
                        _ => format!("{}{}, {}", cmd, self.params.dst, self.params.src),
                    },
                    _ => format!(
                        "{}{}, {}, {}",
                        cmd,
                        self.params.dst,
                        self.params.src,
                        self.params.src2
                    ),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ParameterPair {
    pub dst: Parameter,
    pub src: Parameter,
    pub src2: Parameter,
}

impl ParameterPair {
    // returns the number of parameters
    pub fn count(&self) -> usize {
        match self.dst {
            Parameter::None() => 0,
            _ => match self.src {
                Parameter::None() => 1,
                _ => match self.src2 {
                    Parameter::None() => 2,
                    _ => 3,
                },
            },
        }
    }
}

#[derive(Debug)]
pub enum Parameter {
    Imm8(u8),
    Imm16(u16),
    ImmS8(i8),                          // byte +0x3f
    Ptr8(Segment, u16),                 // byte [u16]
    Ptr16(Segment, u16),                // word [u16]
    Ptr16Imm(u16, u16),                 // jmp far u16:u16
    Ptr8Amode(Segment, usize),          // byte [amode], like "byte [bp+si]"
    Ptr8AmodeS8(Segment, usize, i8),    // byte [amode+s8], like "byte [bp-0x20]"
    Ptr8AmodeS16(Segment, usize, i16),  // byte [amode+s16], like "byte [bp-0x2020]"
    Ptr16Amode(Segment, usize),         // word [amode], like "word [bx]"
    Ptr16AmodeS8(Segment, usize, i8),   // word [amode+s8], like "word [bp-0x20]"
    Ptr16AmodeS16(Segment, usize, i16), // word [amode+s16], like "word [bp-0x2020]"
    Reg8(usize),                        // index into the low 4 of CPU.r16
    Reg16(usize),                       // index into CPU.r16
    SReg16(usize),                      // index into cpu.sreg16
    None(),
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Parameter::Imm8(imm) => write!(f, "0x{:02X}", imm),
            Parameter::Imm16(imm) => write!(f, "0x{:04X}", imm),
            Parameter::ImmS8(imm) => write!(
                f,
                "byte {}0x{:02X}",
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            Parameter::Ptr8(seg, v) => write!(f, "byte [{}:0x{:04X}]", seg, v),
            Parameter::Ptr16(seg, v) => write!(f, "word [{}:0x{:04X}]", seg, v),
            Parameter::Ptr16Imm(seg, v) => write!(f, "{:04X}:{:04X}", seg, v),
            Parameter::Ptr8Amode(seg, v) => write!(f, "byte [{}:{}]", seg, amode(v as u8)),
            Parameter::Ptr8AmodeS8(seg, v, imm) => write!(
                f,
                "byte [{}:{}{}0x{:02X}]",
                seg,
                amode(v as u8),
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            Parameter::Ptr8AmodeS16(seg, v, imm) => write!(
                f,
                "byte [{}:{}{}0x{:04X}]",
                seg,
                amode(v as u8),
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            Parameter::Ptr16Amode(seg, v) => write!(f, "word [{}:{}]", seg, amode(v as u8)),
            Parameter::Ptr16AmodeS8(seg, v, imm) => write!(
                f,
                "word [{}:{}{}0x{:02X}]",
                seg,
                amode(v as u8),
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            Parameter::Ptr16AmodeS16(seg, v, imm) => write!(
                f,
                "word [{}:{}{}0x{:04X}]",
                seg,
                amode(v as u8),
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            Parameter::Reg8(v) => write!(f, "{}", r8(v as u8)),
            Parameter::Reg16(v) => write!(f, "{}", r16(v as u8)),
            Parameter::SReg16(v) => write!(f, "{}", sr16(v as u8)),
            Parameter::None() => write!(f, ""),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Op {
    Aaa,
    Aad(),
    Aam(),
    Aas(),
    Adc8(),
    Adc16(),
    Add8(),
    Add16(),
    And8(),
    And16(),
    Arpl(),
    Bound(),
    Bsf,
    Bt,
    CallNear(),
    Cbw(),
    Clc(),
    Cld(),
    Cli(),
    Cmc(),
    Cmp8(),
    Cmp16(),
    Cmpsb(),
    Cmpsw(),
    Cwd(),
    Daa(),
    Das(),
    Dec8(),
    Dec16(),
    Div8(),
    Div16(),
    Enter,
    Hlt(),
    Idiv8(),
    Idiv16(),
    Imul8(),
    Imul16(),
    In8(),
    In16(),
    Inc8(),
    Inc16(),
    Int(),
    Insb(),
    Insw(),
    Ja(),
    Jc(),
    Jcxz(),
    Jg(),
    Jl(),
    JmpFar(),
    JmpNear(),
    JmpShort(),
    Jna(),
    Jnc(),
    Jng(),
    Jnl(),
    Jno(),
    Jns(),
    Jnz(),
    Jo(),
    Jpe(),
    Jpo(),
    Js(),
    Jz(),
    Lahf(),
    Lds(),
    Lea16(),
    Leave,
    Les(),
    Lodsb(),
    Lodsw(),
    Loop(),
    Loope(),
    Loopne(),
    Mov8(),
    Mov16(),
    Movsb(),
    Movsw(),
    Movsx16(),
    Movzx16(),
    Mul8(),
    Mul16(),
    Neg8(),
    Neg16(),
    Nop(),
    Not8(),
    Not16(),
    Or8(),
    Or16(),
    Out8(),
    Out16(),
    Outsb(),
    Outsw(),
    Pop16(),
    Popa(),
    Popf(),
    Push16(),
    Pusha(),
    Pushf(),
    Rcl8(),
    Rcl16(),
    Rcr8(),
    Rcr16(),
    Retf,
    Retn,
    RetImm16,
    Rol8(),
    Rol16(),
    Ror8(),
    Ror16(),
    Sahf(),
    Salc(),
    Sar8(),
    Sar16(),
    Sbb8(),
    Sbb16(),
    Scasb(),
    Scasw(),
    Setc,
    Setnz,
    Shl8(),
    Shl16(),
    Shr8(),
    Shr16(),
    Shrd(),
    Stc(),
    Std(),
    Sti(),
    Stosb(),
    Stosw(),
    Sub8(),
    Sub16(),
    Test8(),
    Test16(),
    Xchg8(),
    Xchg16(),
    Xlatb(),
    Xor8(),
    Xor16(),
    Unknown(),
    Invalid(InvalidOp),
}

#[derive(Debug, PartialEq)]
pub enum InvalidOp {
    Reg(u8),
    Op(Vec<u8>),
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

impl fmt::Display for InstructionInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hex = self.to_hex_string(&self.bytes);
        write!(
            f,
            "[{:04X}:{:04X}] {} {}",
            self.segment,
            self.offset,
            right_pad(&hex, 16),
            self.text
        )
    }
}

impl InstructionInfo {
    fn to_hex_string(&self, bytes: &[u8]) -> String {
        let strs: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
        strs.join("")
    }
}

pub struct ModRegRm {
    pub md: u8, // NOTE: "mod" is reserved in rust
    pub reg: u8,
    pub rm: u8,
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
