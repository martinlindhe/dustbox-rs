use std::fmt;

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
    /// 8-bit general purpose register
    Reg8(R),
    /// 16-bit general purpose register
    Reg16(R),
    /// 16-bit segment register
    SReg16(R),
    /// 32-bit general purpose register
    Reg32(R),
    /// 80-bit fpu register
    FPR80(R),

    Imm8(u8),                           // byte 0x80
    ImmS8(i8),                          // byte +0x3f
    Imm16(u16),                         // word 0x8000
    Imm32(u32),                         // dword 0x8000_0000
    Ptr16Imm(u16, u32),                 // jmp far u16:u16 or u16:u32

    Ptr8(Segment, u16),                 // byte [u16], like "byte [0x4040]"
    Ptr8Amode(Segment, AMode),          // byte [amode], like "byte [bx]"
    Ptr8AmodeS8(Segment, AMode, i8),    // byte [amode+s8], like "byte [bp-0x20]"
    Ptr8AmodeS16(Segment, AMode, i16),  // byte [amode+s16], like "byte [bp-0x2020]"

    Ptr16(Segment, u16),                // word [u16], like "word [0x4040]"
    Ptr16Amode(Segment, AMode),         // word [amode], like "word [bx]"
    Ptr16AmodeS8(Segment, AMode, i8),   // word [amode+s8], like "word [bp-0x20]"
    Ptr16AmodeS16(Segment, AMode, i16), // word [amode+s16], like "word [bp-0x2020]"
    Ptr16AmodeS32(Segment, AMode, i32), // word [amode+s32], like "dword [bp-0x20204040]"

    Ptr32(Segment, u32),                // dword [u32], like "dword [0x40404040]"
    Ptr32Amode(Segment, AMode),         // dword [amode], like "dword [bx]"
    Ptr32AmodeS8(Segment, AMode, i8),   // dword [amode+s8], like "dword [bp-0x20]"
    Ptr32AmodeS16(Segment, AMode, i16), // dword [amode+s16], like "dword [bp-0x2020]"

    /// Scaled Index Base
    Ptr16SIB(Segment, SIBDisp, u8, R, SIBBase),
    Ptr16SIBS8(Segment, SIBDisp, u8, R, SIBBase, i8),
    Ptr16SIBS32(Segment, SIBDisp, u8, R, SIBBase, i32),
    None,
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Parameter::Reg8(ref r) |
            Parameter::Reg16(ref r) |
            Parameter::Reg32(ref r) |
            Parameter::SReg16(ref r) |
            Parameter::FPR80(ref r) => write!(f, "{}", r),

            Parameter::Imm8(imm) => write!(f, "0x{:02X}", imm),
            Parameter::Imm16(imm) => write!(f, "0x{:04X}", imm),
            Parameter::Imm32(imm) => write!(f, "0x{:08X}", imm),
            Parameter::ImmS8(imm) => write!(
                f,
                "byte {}0x{:02X}",
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i8).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
            Parameter::Ptr16Imm(seg, v) => write!(f, "{:04X}:{:04X}", seg, v),
            Parameter::Ptr8(seg, v) => write!(f, "byte [{}:0x{:04X}]", seg, v),
            Parameter::Ptr8Amode(seg, ref amode) => write!(f, "byte [{}:{}]", seg, amode),
            Parameter::Ptr8AmodeS8(seg, ref amode, imm) => write!(
                f,
                "byte [{}:{}{}0x{:02X}]",
                seg,
                amode,
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i8).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
            Parameter::Ptr8AmodeS16(seg, ref amode, imm) => write!(
                f,
                "byte [{}:{}{}0x{:04X}]",
                seg,
                amode,
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i16).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
            Parameter::Ptr16(seg, v) => write!(f, "word [{}:0x{:04X}]", seg, v),
            Parameter::Ptr16Amode(seg, ref amode) => write!(f, "word [{}:{}]", seg, amode),
            Parameter::Ptr16AmodeS8(seg, ref amode, imm) => write!(
                f,
                "word [{}:{}{}0x{:02X}]",
                seg,
                amode,
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i8).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
            Parameter::Ptr16AmodeS16(seg, ref amode, imm) => write!(
                f,
                "word [{}:{}{}0x{:04X}]",
                seg,
                amode,
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i16).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
            Parameter::Ptr16AmodeS32(seg, ref amode, imm) => write!(
                f,
                "word [{}:{}{}0x{:08X}]",
                seg,
                amode,
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i32).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
            Parameter::Ptr32(seg, v) => write!(f, "dword [{}:0x{:04X}]", seg, v),
            Parameter::Ptr32Amode(seg, ref amode) => write!(f, "dword [{}:{}]", seg, amode),
            Parameter::Ptr32AmodeS8(seg, ref amode, imm) => write!(
                f,
                "dword [{}:{}{}0x{:02X}]",
                seg,
                amode,
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i8).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
            Parameter::Ptr32AmodeS16(seg, ref amode, imm) => write!(
                f,
                "dword [{}:{}{}0x{:04X}]",
                seg,
                amode,
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i16).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
            Parameter::Ptr16SIB(seg, disp, scale, index, base) => {
                if scale == 1 {
                    write!(f, "word [{}:{}+{}{}]", seg, base, index, disp)
                } else {
                    write!(f, "word [{}:{}+{}*{}{}]", seg, base, index, scale, disp)
                }
            }
            Parameter::Ptr16SIBS8(seg, disp, scale, index, base, imm) => {
                if scale == 1 {
                    write!(f, "word [{}:{}+{}{}0x{:02X}{}]", seg, base, index,
                        if imm < 0 { "-" } else { "+" },
                        if imm < 0 {
                            (0i8).wrapping_sub(imm)
                        } else {
                            imm
                        },
                        disp
                    )
                } else {
                    // XXX all wrong
                    //write!(f, "word [{}:{} + {} * {}]", seg, base, index, scale)
                    panic!("fixme")
                }
            }
            Parameter::Ptr16SIBS32(seg, disp, scale, index, base, imm) => {
                if scale == 1 {
                    write!(f, "word [{}:{}+{}{}0x{:02X}{}]", seg, base, index,
                        if imm < 0 { "-" } else { "+" },
                        if imm < 0 {
                            (0i32).wrapping_sub(imm)
                        } else {
                            imm
                        },
                        disp
                    )
                } else {
                    // XXX all wrong
                    //write!(f, "word [{}:{} + {} * {}]", seg, base, index, scale)
                    panic!("fixme")
                }
            }
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

/// Instruction encoding layout for Scale/Index/Base byte
#[derive(Debug)]
pub struct SIB {
    /// High 2 bits
    pub scale: u8,
    /// Mid 3 bits
    pub index: u8,
    /// Low 3 bits
    pub base: u8,
}

/// Instruction encoding layout for the SIB Base
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SIBBase {
    Register(R),
    Empty,
}

impl fmt::Display for SIBBase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SIBBase::Register(r) => write!(f, "{}", r),
            SIBBase::Empty => write!(f, ""),
        }
    }
}

/// Instruction encoding layout for the SIB Displacement
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SIBDisp {
    Empty,
    Disp32(i32),
    Disp8EBP(i8),
    Disp32EBP(i32),
}

impl fmt::Display for SIBDisp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SIBDisp::Empty => write!(f, ""),
            SIBDisp::Disp32(imm) => write!(
                f, "{}0x{:08X}",
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i32).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
            SIBDisp::Disp8EBP(imm) => write!(
                f, "{}0x{:02X}+EBP",
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i8).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
            SIBDisp::Disp32EBP(imm) => write!(
                f, "{}0x{:08X}+EBP",
                if imm < 0 { "-" } else { "+" },
                if imm < 0 {
                    (0i32).wrapping_sub(imm)
                } else {
                    imm
                }
            ),
        }
    }
}

/// Instruction encoding layout for Mod/Reg/RM byte
#[derive(Debug)]
pub struct ModRegRm {
    /// "mod" is correct name, but is reserved keyword
    /// High 2 bits
    pub md: u8,

    /// mid 3 bits
    pub reg: u8,

    /// low 3 bits
    pub rm: u8,
}

impl ModRegRm {
    pub fn u8(&self) -> u8 {
        (self.md << 6) |  // high 2 bits
        (self.reg << 3) | // mid 3 bits
        self.rm           // low 3 bits
    }

    pub fn rm_reg(rm: u8, reg: u8) -> u8 {
        // md 3 = register adressing
        // XXX ModRegRm.rm really should use enum AMode, not like AMode is now. naming there is wrong
        ModRegRm{md: 3, rm, reg}.u8()
    }
}
