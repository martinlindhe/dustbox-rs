use std::convert::From;

use cpu::flags::Flags;

#[derive(Copy, Clone, Debug, Default)]
pub struct Register16 {
    pub val: u16,
}

impl Register16 {
    pub fn set_hi(&mut self, val: u8) {
        self.val = (self.val & 0xFF) + (u16::from(val) << 8);
    }
    pub fn set_lo(&mut self, val: u8) {
        self.val = (self.val & 0xFF00) + u16::from(val);
    }
    pub fn lo_u8(&self) -> u8 {
        (self.val & 0xFF) as u8
    }
    pub fn hi_u8(&self) -> u8 {
        (self.val >> 8) as u8
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum R {
    AL, CL, DL, BL, AH, CH, DH, BH,
    AX, CX, DX, BX, SP, BP, SI, DI,
}

impl R {
    pub fn index(&self) -> usize {
          match *self {
            R::AL => 0,
            R::CL => 1,
            R::DL => 2,
            R::BL => 3,
            R::AH => 4,
            R::CH => 5,
            R::DH => 6,
            R::BH => 7,

            R::AX => 0,
            R::CX => 1,
            R::DX => 2,
            R::BX => 3,
            R::SP => 4,
            R::BP => 5,
            R::SI => 6,
            R::DI => 7,
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

#[derive(Clone, Debug, PartialEq)]
pub enum SR {
    ES, CS, SS, DS, FS, GS
}

impl SR {
   pub fn index(&self) -> usize {
        match *self {
            SR::ES => 0,
            SR::CS => 1,
            SR::SS => 2,
            SR::DS => 3,
            SR::FS => 4,
            SR::GS => 5,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match *self {
            SR::ES => "es",
            SR::CS => "cs",
            SR::SS => "ss",
            SR::DS => "ds",
            SR::FS => "fs",
            SR::GS => "gs",
        }
    }
}

impl Into<SR> for u8 {
    fn into(self) -> SR {
        match self {
            0 => SR::ES,
            1 => SR::CS,
            2 => SR::SS,
            3 => SR::DS,
            4 => SR::FS,
            5 => SR::GS,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AMode {
    BXSI, BXDI, BPSI, BPDI, SI, DI, BP, BX
}

impl AMode {
   pub fn index(&self) -> usize {
        match *self {
            AMode::BXSI => 0,
            AMode::BXDI => 1,
            AMode::BPSI => 2,
            AMode::BPDI => 3,
            AMode::SI => 4,
            AMode::DI => 5,
            AMode::BP => 6,
            AMode::BX => 7,
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
        }
    }
}

impl Into<AMode> for u8 {
    fn into(self) -> AMode {
        match self {
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
}

pub struct RegisterSnapshot {
    pub ip: u16,
    pub r16: [Register16; 8], // general purpose registers
    pub sreg16: [u16; 6],     // segment registers
    pub flags: Flags,
}
