#[derive(Copy, Clone)]
pub struct Register16 {
    pub val: u16,
}

impl Register16 {
    pub fn set_hi(&mut self, val: u8) {
        self.val = (self.val & 0xFF) + ((val as u16) << 8);
    }
    pub fn set_lo(&mut self, val: u8) {
        self.val = (self.val & 0xFF00) + val as u16;
    }
    pub fn lo_u8(&mut self) -> u8 {
        (self.val & 0xFF) as u8
    }
    pub fn hi_u8(&mut self) -> u8 {
        (self.val >> 8) as u8
    }

    pub fn as_hex_string(&self) -> String {
        format!("<span font_desc=\"mono\">{:04X}</span>", self.val)
    }
}

// r8 (4 low of r16)
pub const AL: usize = 0;
pub const CL: usize = 1;
pub const DL: usize = 2;
pub const BL: usize = 3;
pub const AH: usize = 4;
pub const CH: usize = 5;
pub const DH: usize = 6;
pub const BH: usize = 7;

// r16
pub const AX: usize = 0;
pub const CX: usize = 1;
pub const DX: usize = 2;
pub const BX: usize = 3;
pub const SP: usize = 4;
pub const BP: usize = 5;
pub const SI: usize = 6;
pub const DI: usize = 7;

// sreg16
pub const ES: usize = 0;
pub const CS: usize = 1;
pub const SS: usize = 2;
pub const DS: usize = 3;
pub const FS: usize = 4;
pub const GS: usize = 5;
