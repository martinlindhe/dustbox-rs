use std::convert::From;

#[derive(Copy, Clone, Default)]
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
pub enum R8 {
    AL, CL, DL, BL, AH, CH, DH, BH
}

impl R8 {
    pub fn as_str(&self) -> &'static str {
        match self {
            &R8::AL => "al",
            &R8::CL => "cl",
            &R8::DL => "dl",
            &R8::BL => "bl",
            &R8::AH => "ah",
            &R8::CH => "ch",
            &R8::DH => "dh",
            &R8::BH => "bh",
        }
    }
}

impl Into<R8> for u8 {
    fn into(self) -> R8 {
        match self {
            0 => R8::AL,
            1 => R8::CL,
            2 => R8::DL,
            3 => R8::BL,
            4 => R8::AH,
            5 => R8::CH,
            6 => R8::DH,
            7 => R8::BH,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum R16 {
    AX, CX, DX, BX, SP, BP, SI, DI
}

impl R16 {
    pub fn index(&self) -> usize {
        match self {
            &R16::AX => 0,
            &R16::CX => 1,
            &R16::DX => 2,
            &R16::BX => 3,
            &R16::SP => 4,
            &R16::BP => 5,
            &R16::SI => 6,
            &R16::DI => 7,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            &R16::AX => "ax",
            &R16::CX => "cx",
            &R16::DX => "dx",
            &R16::BX => "bx",
            &R16::SP => "sp",
            &R16::BP => "bp",
            &R16::SI => "si",
            &R16::DI => "di",
        }
    }
}

impl Into<R16> for u8 {
    fn into(self) -> R16 {
        match self {
            0 => R16::AX,
            1 => R16::CX,
            2 => R16::DX,
            3 => R16::BX,
            4 => R16::SP,
            5 => R16::BP,
            6 => R16::SI,
            7 => R16::DI,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SR {
    ES, CS, SS, DS, FS, GS
}

impl SR {
   pub fn index(&self) -> usize {
        match self {
            &SR::ES => 0,
            &SR::CS => 1,
            &SR::SS => 2,
            &SR::DS => 3,
            &SR::FS => 4,
            &SR::GS => 5,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            &SR::ES => "es",
            &SR::CS => "cs",
            &SR::SS => "ss",
            &SR::DS => "ds",
            &SR::FS => "fs",
            &SR::GS => "gs",
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

#[derive(Debug, PartialEq)]
pub enum AMode {
    BXSI, BXDI, BPSI, BPDI, SI, DI, BP, BX
}

impl AMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            &AMode::BXSI => "bx+si",
            &AMode::BXDI => "bx+di",
            &AMode::BPSI => "bp+si",
            &AMode::BPDI => "bp+di",
            &AMode::SI => "si",
            &AMode::DI => "di",
            &AMode::BP => "bp",
            &AMode::BX => "bx",
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
