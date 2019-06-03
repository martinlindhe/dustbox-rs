use std::fmt;
use std::num::Wrapping;

use crate::cpu::segment::Segment;
use crate::cpu::register::{R, AMode};

/// A set of Parameters for an Instruction
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterSet {
    pub dst: Parameter,
    pub src: Parameter,
    pub src2: Parameter,
}

impl ParameterSet {
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

#[derive(Clone, Debug, PartialEq)]
pub enum Parameter {
    Reg8(R),                            // 8-bit general purpose register
    Reg16(R),                           // 16-bit general purpose register
    SReg16(R),                          // 16-bit segment register
    Reg32(R),                           // 32-bit general purpose register

    Imm8(u8),                           // byte 0x80
    ImmS8(i8),                          // byte +0x3f
    Imm16(u16),                         // word 0x8000
    Imm32(u32),                         // dword 0x8000_0000
    Ptr16Imm(u16, u16),                 // jmp far u16:u16

    Ptr8(Segment, u16),                 // byte [u16], like "byte [0x4040]"
    Ptr8Amode(Segment, AMode),          // byte [amode], like "byte [bx]"
    Ptr8AmodeS8(Segment, AMode, i8),    // byte [amode+s8], like "byte [bp-0x20]"
    Ptr8AmodeS16(Segment, AMode, i16),  // byte [amode+s16], like "byte [bp-0x2020]"

    Ptr16(Segment, u16),                // word [u16], like "word [0x4040]"
    Ptr16Amode(Segment, AMode),         // word [amode], like "word [bx]"
    Ptr16AmodeS8(Segment, AMode, i8),   // word [amode+s8], like "word [bp-0x20]"
    Ptr16AmodeS16(Segment, AMode, i16), // word [amode+s16], like "word [bp-0x2020]"

    Ptr32(Segment, u16),                // dword [u16], like "dword [0x4040]"
    Ptr32Amode(Segment, AMode),         // dword [amode], like "dword [bx]"
    Ptr32AmodeS8(Segment, AMode, i8),   // dword [amode+s8], like "dword [bp-0x20]"
    Ptr32AmodeS16(Segment, AMode, i16), // dword [amode+s16], like "dword [bp-0x2020]"
    None,
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Parameter::Reg8(ref r) |
            Parameter::Reg16(ref r) |
            Parameter::Reg32(ref r) |
            Parameter::SReg16(ref r) => write!(f, "{}", r),

            Parameter::Imm8(imm) => write!(f, "0x{:02X}", imm),
            Parameter::Imm16(imm) => write!(f, "0x{:04X}", imm),
            Parameter::Imm32(imm) => write!(f, "0x{:08X}", imm),
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
            Parameter::Ptr16Imm(seg, v) => write!(f, "{:04X}:{:04X}", seg, v),
            Parameter::Ptr8(seg, v) => write!(f, "byte [{}:0x{:04X}]", seg, v),
            Parameter::Ptr8Amode(seg, ref amode) => write!(f, "byte [{}:{}]", seg, amode.as_str()),
            Parameter::Ptr8AmodeS8(seg, ref amode, imm) => write!(
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
            Parameter::Ptr8AmodeS16(seg, ref amode, imm) => write!(
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
            Parameter::Ptr16(seg, v) => write!(f, "word [{}:0x{:04X}]", seg, v),
            Parameter::Ptr16Amode(seg, ref amode) => write!(f, "word [{}:{}]", seg, amode.as_str()),
            Parameter::Ptr16AmodeS8(seg, ref amode, imm) => write!(
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
            Parameter::Ptr16AmodeS16(seg, ref amode, imm) => write!(
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
            Parameter::Ptr32(seg, v) => write!(f, "dword [{}:0x{:04X}]", seg, v),
            Parameter::Ptr32Amode(seg, ref amode) => write!(f, "dword [{}:{}]", seg, amode.as_str()),
            Parameter::Ptr32AmodeS8(seg, ref amode, imm) => write!(
                f,
                "dword [{}:{}{}0x{:02X}]",
                seg,
                amode.as_str(),
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            Parameter::Ptr32AmodeS16(seg, ref amode, imm) => write!(
                f,
                "dword [{}:{}{}0x{:04X}]",
                seg,
                amode.as_str(),
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (Wrapping(0) - Wrapping(imm)).0
                } else {
                    imm
                }
            ),
            Parameter::None => write!(f, ""),
        }
    }
}

impl Parameter {
    pub fn is_imm(&self) -> bool {
        match *self {
            Parameter::Imm8(_) |
            Parameter::Imm16(_) |
            Parameter::Imm32(_) |
            Parameter::ImmS8(_) => true,
            _ => false,
        }
    }

    pub fn is_ptr(&self) -> bool {
        match *self {
            Parameter::Ptr8(_, _) |
            Parameter::Ptr16(_, _) |
            Parameter::Ptr16Imm(_, _) |
            Parameter::Ptr8Amode(_, _) |
            Parameter::Ptr8AmodeS8(_, _, _) |
            Parameter::Ptr8AmodeS16(_, _, _) |
            Parameter::Ptr16Amode(_, _) |
            Parameter::Ptr16AmodeS8(_, _, _) |
            Parameter::Ptr16AmodeS16(_, _, _) => true,
            _ => false,
        }
    }

    pub fn is_reg(&self) -> bool {
        match *self {
            Parameter::Reg8(_) |
            Parameter::Reg16(_) |
            Parameter::Reg32(_) |
            Parameter::SReg16(_) => true,
            _ => false,
        }
    }

    pub fn is_none(&self) -> bool {
        *self == Parameter::None
    }
}
