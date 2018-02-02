use std::fmt;
use std::num::Wrapping;

use cpu::segment::Segment;
use cpu::register::{R8, R16, SR, AMode};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RepeatMode {
    None,
    Rep,
    Repe, // alias repz
    Repne, // alias repnz
}

impl RepeatMode {
    fn as_str(&self) -> &str {
        match *self {
            RepeatMode::None => "",
            RepeatMode::Rep => "Rep ",
            RepeatMode::Repe => "Repe ",
            RepeatMode::Repne => "Repne ",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Instruction {
    pub command: Op,
    pub params: ParameterPair,
    pub segment_prefix: Segment,
    pub length: u8,
    pub repeat: RepeatMode, // REPcc prefix
    pub lock: bool,         // LOCK prefix
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let instr = self.describe_instruction();
        if self.segment_prefix == Segment::Default || self.hide_segment_prefix() {
            write!(f, "{}", instr)
        } else {
            write!(f, "{} {}", self.segment_prefix.as_str(), instr)
        }
    }
}

impl Instruction {
    pub fn new1(op: Op, dst: Parameter) -> Self {
        Instruction {
            command: op,
            segment_prefix: Segment::Default,
            lock: false,
            repeat: RepeatMode::None,
            params: ParameterPair {
                dst: dst,
                src: Parameter::None,
                src2: Parameter::None,
            },
            length: 0, // XXX remove length here, cannot be known
        }
    }

    pub fn new2(op: Op, dst: Parameter, src: Parameter) -> Self {
        Instruction {
            command: op,
            segment_prefix: Segment::Default,
            lock: false,
            repeat: RepeatMode::None,
            params: ParameterPair {
                dst: dst,
                src: src,
                src2: Parameter::None,
            },
            length: 0, // XXX remove length here, cannot be known
        }
    }

    fn hide_segment_prefix(&self) -> bool {
        self.command == Op::Mov8() ||
        self.command == Op::Mov16()
    }

    fn describe_instruction(&self) -> String {
        let prefix = self.repeat.as_str();

        match self.params.dst {
            Parameter::None => format!("{}{:?}", prefix, self.command),
            _ => {
                let cmd = right_pad(&format!("{}{:?}", prefix, self.command), 9);

                match self.params.src2 {
                    Parameter::None => match self.params.src {
                        Parameter::None => format!("{}{}", cmd, self.params.dst),
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

#[derive(Debug, PartialEq)]
pub struct ParameterPair {
    pub dst: Parameter,
    pub src: Parameter,
    pub src2: Parameter,
}

impl ParameterPair {
    // returns the number of parameters
    pub fn count(&self) -> usize {
        match self.dst {
            Parameter::None => 0,
            _ => match self.src {
                Parameter::None => 1,
                _ => match self.src2 {
                    Parameter::None => 2,
                    _ => 3,
                },
            },
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Parameter {
    Imm8(u8),                           // byte 0x80
    Imm16(u16),                         // word 0x8000
    ImmS8(i8),                          // byte +0x3f
    Ptr8(Segment, u16),                 // byte [u16]
    Ptr16(Segment, u16),                // word [u16]
    Ptr16Imm(u16, u16),                 // jmp far u16:u16
    Ptr8Amode(Segment, AMode),          // byte [amode], like "byte [bp+si]"
    Ptr8AmodeS8(Segment, AMode, i8),    // byte [amode+s8], like "byte [bp-0x20]"
    Ptr8AmodeS16(Segment, AMode, i16),  // byte [amode+s16], like "byte [bp-0x2020]"
    Ptr16Amode(Segment, AMode),         // word [amode], like "word [bx]"
    Ptr16AmodeS8(Segment, AMode, i8),   // word [amode+s8], like "word [bp-0x20]"
    Ptr16AmodeS16(Segment, AMode, i16), // word [amode+s16], like "word [bp-0x2020]"
    Reg8(R8),                           // index into the low 4 of CPU.r16
    Reg16(R16),                         // index into CPU.r16
    SReg16(SR),                         // index into cpu.sreg16
    None,
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Parameter::Imm8(imm) => write!(f, "0x{:02X}", imm),
            &Parameter::Imm16(imm) => write!(f, "0x{:04X}", imm),
            &Parameter::ImmS8(imm) => write!(
                f,
                "byte {}0x{:02X}",
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            &Parameter::Ptr8(seg, v) => write!(f, "byte [{}:0x{:04X}]", seg, v),
            &Parameter::Ptr16(seg, v) => write!(f, "word [{}:0x{:04X}]", seg, v),
            &Parameter::Ptr16Imm(seg, v) => write!(f, "{:04X}:{:04X}", seg, v),
            &Parameter::Ptr8Amode(seg, ref amode) => write!(f, "byte [{}:{}]", seg, amode.as_str()),
            &Parameter::Ptr8AmodeS8(seg, ref amode, imm) => write!(
                f,
                "byte [{}:{}{}0x{:02X}]",
                seg,
                amode.as_str(),
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            &Parameter::Ptr8AmodeS16(seg, ref amode, imm) => write!(
                f,
                "byte [{}:{}{}0x{:04X}]",
                seg,
                amode.as_str(),
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            &Parameter::Ptr16Amode(seg, ref amode) => write!(f, "word [{}:{}]", seg, amode.as_str()),
            &Parameter::Ptr16AmodeS8(seg, ref amode, imm) => write!(
                f,
                "word [{}:{}{}0x{:02X}]",
                seg,
                amode.as_str(),
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            &Parameter::Ptr16AmodeS16(seg, ref amode, imm) => write!(
                f,
                "word [{}:{}{}0x{:04X}]",
                seg,
                amode.as_str(),
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            &Parameter::Reg8(ref v) => write!(f, "{}", v.as_str()),
            &Parameter::Reg16(ref v) => write!(f, "{}", v.as_str()),
            &Parameter::SReg16(ref v) => write!(f, "{}", v.as_str()),
            &Parameter::None => write!(f, ""),
        }
    }
}

impl Parameter {
    pub fn is_imm(&self) -> bool {
        match *self {
            Parameter::Imm8(_) => true,
            Parameter::Imm16(_) => true,
            Parameter::ImmS8(_) => true,
            _ => false,
        }
    }

    pub fn is_ptr(&self) -> bool {
        match *self {
            Parameter::Ptr8(_, _) => true,
            Parameter::Ptr16(_, _) => true,
            Parameter::Ptr16Imm(_, _) => true,
            Parameter::Ptr8Amode(_, _) => true,
            Parameter::Ptr8AmodeS8(_, _, _) => true,
            Parameter::Ptr8AmodeS16(_, _, _) => true,
            Parameter::Ptr16Amode(_, _) => true,
            Parameter::Ptr16AmodeS8(_, _, _) => true,
            Parameter::Ptr16AmodeS16(_, _, _) => true,
            _ => false,
        }
    }

    pub fn is_reg(&self) -> bool {
        match *self {
            Parameter::Reg8(_) => true,
            Parameter::Reg16(_) => true,
            Parameter::SReg16(_) => true,
            _ => false,
        }
    }

    pub fn is_none(&self) -> bool {
        return *self == Parameter::None
    }
}

#[derive(Debug, PartialEq)]
pub enum Op {
    // NOTE: currently removing paranthesis of enums to track if flags have been fully emulated & impl tested //martin, jan 2018
    Aaa(),
    Aad(),
    Aam(),
    Aas(),
    Adc8(),
    Adc16(),
    Add8,
    Add16,
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
    Idiv8,
    Idiv16,
    Imul8,
    Imul16,
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
    Rcl8,
    Rcl16,
    Rcr8,
    Rcr16,
    Retf,
    Retn,
    RetImm16,
    Rol8,
    Rol16,
    Ror8,
    Ror16,
    Sahf(),
    Salc(),
    Sar8,
    Sar16,
    Sbb8(),
    Sbb16(),
    Scasb(),
    Scasw(),
    Setc,
    Setnz,
    Shl8,
    Shl16,
    Shld(),
    Shr8,
    Shr16,
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

#[derive(Debug, PartialEq)]
pub struct InstructionInfo {
    pub segment: usize,
    pub offset: usize,
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

#[derive(Debug)]
pub struct ModRegRm {
    pub md: u8,  /// "mod" is correct name, but is reserved keyword
    pub reg: u8,
    pub rm: u8,
}

impl ModRegRm {
    pub fn u8(&self) -> u8 {
        (self.md << 6) |  // high 2 bits
        (self.reg << 3) | // mid 3 bits
        self.rm           // low 3 bits
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
