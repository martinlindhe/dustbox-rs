use std::convert::From;

use cpu::flag::Flags;
use cpu::decoder::AddressSize;

#[cfg(test)]
#[path = "./register_test.rs"]
mod register_test;

// 32-bit general purpose register (AL->AX->EAX)
#[derive(Copy, Clone, Debug, Default)]
pub struct GPR {
    val: u32,
}

impl GPR {
    /// sets the high byte of the word register
    pub fn set_hi(&mut self, val: u8) {
        self.val = (self.val & 0xFFFF_00FF) + (u32::from(val) << 8);
    }

    /// sets the low byte of the word register
    pub fn set_lo(&mut self, val: u8) {
        self.val = (self.val & 0xFFFF_FF00) + u32::from(val);
    }

    // gets the low byte of the word register
    pub fn lo_u8(&self) -> u8 {
        (self.val & 0xFF) as u8
    }

    // gets the hi byte of the word register
    pub fn hi_u8(&self) -> u8 {
        (self.val >> 8) as u8
    }

    pub fn set16(&mut self, val: u16) {
        self.val = (self.val & 0xFFFF_0000) | u32::from(val);
    }

    pub fn set32(&mut self, val: u32) {
        self.val = val;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum R {
    AL, CL, DL, BL, AH, CH, DH, BH,         // 8-bit gpr
    AX, CX, DX, BX, SP, BP, SI, DI,         // 16-bit gpr
    ES, CS, SS, DS, FS, GS,                 // sr
    IP,                                     // ip
    EAX, ECX, EDX, EBX, ESP, EBP, ESI, EDI, //
}

impl R {
    pub fn index(&self) -> usize {
          match *self {
            R::AL | R::AX | R::EAX | R::ES => 0,
            R::CL | R::CX | R::ECX | R::CS => 1,
            R::DL | R::DX | R::EDX | R::SS => 2,
            R::BL | R::BX | R::EBX | R::DS => 3,
            R::AH | R::SP | R::ESP | R::FS => 4,
            R::CH | R::BP | R::EBP | R::GS => 5,
            R::DH | R::SI | R::ESI => 6,
            R::BH | R::DI | R::EDI => 7,
            _ => unreachable!(),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match *self {
            R::AL => "al",
            R::CL => "cl",
            R::DL => "dl",
            R::BL => "bl",
            R::AH => "ah",
            R::CH => "ch",
            R::DH => "dh",
            R::BH => "bh",

            R::AX => "ax",
            R::CX => "cx",
            R::DX => "dx",
            R::BX => "bx",
            R::SP => "sp",
            R::BP => "bp",
            R::SI => "si",
            R::DI => "di",
            R::ES => "es",
            R::CS => "cs",
            R::SS => "ss",
            R::DS => "ds",
            R::FS => "fs",
            R::GS => "gs",
            R::IP => "ip",

            R::EAX => "eax",
            R::ECX => "ecx",
            R::EDX => "edx",
            R::EBX => "ebx",
            R::ESP => "esp",
            R::EBP => "ebp",
            R::ESI => "esi",
            R::EDI => "edi",
        }
    }
}

pub fn r8(v: u8) -> R {
    match v {
        0 => R::AL,
        1 => R::CL,
        2 => R::DL,
        3 => R::BL,
        4 => R::AH,
        5 => R::CH,
        6 => R::DH,
        7 => R::BH,
        _ => unreachable!(),
    }
}

pub fn r16(v: u8) -> R {
    match v {
        0 => R::AX,
        1 => R::CX,
        2 => R::DX,
        3 => R::BX,
        4 => R::SP,
        5 => R::BP,
        6 => R::SI,
        7 => R::DI,
        _ => unreachable!(),
    }
}

pub fn r32(v: u8) -> R {
    match v {
        0 => R::EAX,
        1 => R::ECX,
        2 => R::EDX,
        3 => R::EBX,
        4 => R::ESP,
        5 => R::EBP,
        6 => R::ESI,
        7 => R::EDI,
        _ => unreachable!(),
    }
}

pub fn sr(v: u8) -> R {
    match v {
        0 => R::ES,
        1 => R::CS,
        2 => R::SS,
        3 => R::DS,
        4 => R::FS,
        5 => R::GS,
        _ => unreachable!(),
    }
}


#[derive(Clone, Debug, PartialEq)]
pub enum AMode {
    // 16-bit addressing modes
    BXSI, BXDI, BPSI, BPDI, SI, DI, BP, BX,

    // 32-bit addressing modes
    EAX, ECX, EDX, EBX, ESP, EBP, ESI, EDI,
}

impl AMode {
   pub fn index(&self) -> usize {
        match *self {
            AMode::BXSI | AMode::EAX => 0,
            AMode::BXDI | AMode::ECX => 1,
            AMode::BPSI | AMode::EDX => 2,
            AMode::BPDI | AMode::EBX => 3,
            AMode::SI | AMode::ESP => 4,
            AMode::DI | AMode::EBP => 5,
            AMode::BP | AMode::ESI => 6,
            AMode::BX | AMode::EDI => 7,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match *self {
            AMode::BXSI => "bx+si",
            AMode::BXDI => "bx+di",
            AMode::BPSI => "bp+si",
            AMode::BPDI => "bp+di",
            AMode::SI => "si",
            AMode::DI => "di",
            AMode::BP => "bp",
            AMode::BX => "bx",

            AMode::EAX => "eax",
            AMode::ECX => "ecx",
            AMode::EDX => "edx",
            AMode::EBX => "ebx",
            AMode::ESP => "esp",
            AMode::EBP => "ebp",
            AMode::ESI => "esi",
            AMode::EDI => "edi",
        }
    }
}

impl AddressSize {
    pub fn amode_from(&self, val: u8) -> AMode {
        match self {
            AddressSize::_16bit => {
                match val {
                    0 => AMode::BXSI,
                    1 => AMode::BXDI,
                    2 => AMode::BPSI,
                    3 => AMode::BPDI,
                    4 => AMode::SI,
                    5 => AMode::DI,
                    6 => AMode::BP,
                    7 => AMode::BX,
                    _ => unreachable!(),
                }
            }
            AddressSize::_32bit => {
                match val {
                    0 => AMode::EAX,
                    1 => AMode::ECX,
                    2 => AMode::EDX,
                    3 => AMode::EBX,
                    4 => AMode::ESP,
                    5 => AMode::EBP,
                    6 => AMode::ESI,
                    7 => AMode::EDI,
                    _ => unreachable!(),
                }
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct RegisterSnapshot {
    pub ip: u16,
    pub gpr: [GPR; 8 + 6 + 1],   // 8 general purpose registers, 6 segment registers, 1 ip
    pub sreg16: [u16; 6],        // segment registers
    pub flags: Flags,
}

impl RegisterSnapshot {
    pub fn get_r8(&self, r: &R) -> u8 {
        match *r {
            R::AL => self.gpr[0].lo_u8(),
            R::CL => self.gpr[1].lo_u8(),
            R::DL => self.gpr[2].lo_u8(),
            R::BL => self.gpr[3].lo_u8(),
            R::AH => self.gpr[0].hi_u8(),
            R::CH => self.gpr[1].hi_u8(),
            R::DH => self.gpr[2].hi_u8(),
            R::BH => self.gpr[3].hi_u8(),
            _ => unreachable!(),
        }
    }

    pub fn set_r8(&mut self, r: &R, val: u8) {
        match *r {
            R::AL => self.gpr[0].set_lo(val),
            R::CL => self.gpr[1].set_lo(val),
            R::DL => self.gpr[2].set_lo(val),
            R::BL => self.gpr[3].set_lo(val),
            R::AH => self.gpr[0].set_hi(val),
            R::CH => self.gpr[1].set_hi(val),
            R::DH => self.gpr[2].set_hi(val),
            R::BH => self.gpr[3].set_hi(val),
            _ => unreachable!(),
        }
    }

    pub fn get_r16(&self, r: &R) -> u16 {
        match *r {
            R::AX => self.gpr[0].val as u16,
            R::CX => self.gpr[1].val as u16,
            R::DX => self.gpr[2].val as u16,
            R::BX => self.gpr[3].val as u16,
            R::SP => self.gpr[4].val as u16,
            R::BP => self.gpr[5].val as u16,
            R::SI => self.gpr[6].val as u16,
            R::DI => self.gpr[7].val as u16,
            R::ES => self.sreg16[0],
            R::CS => self.sreg16[1],
            R::SS => self.sreg16[2],
            R::DS => self.sreg16[3],
            R::FS => self.sreg16[4],
            R::GS => self.sreg16[5],
            R::IP => self.ip,
            _ => unreachable!(),
        }
    }

    pub fn set_r16(&mut self, r: &R, val: u16) {
        match *r {
            R::AX => self.gpr[0].set16(val),
            R::CX => self.gpr[1].set16(val),
            R::DX => self.gpr[2].set16(val),
            R::BX => self.gpr[3].set16(val),
            R::SP => self.gpr[4].set16(val),
            R::BP => self.gpr[5].set16(val),
            R::SI => self.gpr[6].set16(val),
            R::DI => self.gpr[7].set16(val),
            R::ES => self.sreg16[0] = val,
            R::CS => self.sreg16[1] = val,
            R::SS => self.sreg16[2] = val,
            R::DS => self.sreg16[3] = val,
            R::FS => self.sreg16[4] = val,
            R::GS => self.sreg16[5] = val,
            _ => unreachable!(),
          }
    }

    pub fn get_r32(&self, r: &R) -> u32 {
        match *r {
            R::EAX => self.gpr[0].val,
            R::ECX => self.gpr[1].val,
            R::EDX => self.gpr[2].val,
            R::EBX => self.gpr[3].val,
            R::ESP => self.gpr[4].val,
            R::EBP => self.gpr[5].val,
            R::ESI => self.gpr[6].val,
            R::EDI => self.gpr[7].val,
            _ => unreachable!(),
        }
    }

    pub fn set_r32(&mut self, r: &R, val: u32) {
        match *r {
            R::EAX => self.gpr[0].set32(val),
            R::ECX => self.gpr[1].set32(val),
            R::EDX => self.gpr[2].set32(val),
            R::EBX => self.gpr[3].set32(val),
            R::ESP => self.gpr[4].set32(val),
            R::EBP => self.gpr[5].set32(val),
            R::ESI => self.gpr[6].set32(val),
            R::EDI => self.gpr[7].set32(val),
            _ => unreachable!(),
        }
    }
}
