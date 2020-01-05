// TODO later: look into bitflags! macro

#[cfg(test)]
#[path = "./flag_test.rs"]
mod flag_test;

/// https://en.wikipedia.org/wiki/FLAGS_register
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Flags {
    // ____ O___ SZ_A _P_C
    pub carry: bool, // 0: carry flag
    reserved1: bool, // 1: reserved, always 1 in EFLAGS
    pub parity: bool, // 2: parity flag
    reserved3: bool,
    pub adjust: bool, // 4: adjust flag
    reserved5: bool,
    pub zero: bool, // 6: zero flag
    pub sign: bool, // 7: sign flag
    pub trap: bool, // 8: trap flag (single step)
    pub interrupt: bool, // 9: interrupt flag
    pub direction: bool, // 10: direction flag (control with cld, std)
    pub overflow: bool, // 11: overflow flag
    iopl12: bool, // 12: I/O privilege level (286+ only), always 1 on 8086 and 186
    iopl13: bool, // 13 --""---
    nested_task: bool, // 14: Nested task flag (286+ only), always 1 on 8086 and 186
    reserved15: bool, // 15: Reserved, always 1 on 8086 and 186, always 0 on later models
}

// XXX make use of flag mask
pub const FLAG_CF: u16 = 0x0000_0001;
pub const FLAG_PF: u16 = 0x0000_0004;
pub const FLAG_AF: u16 = 0x0000_0010;
pub const FLAG_ZF: u16 = 0x0000_0040;
pub const FLAG_SF: u16 = 0x0000_0080;
pub const FLAG_TF: u16 = 0x0000_0100;
pub const FLAG_IF: u16 = 0x0000_0200;
pub const FLAG_DF: u16 = 0x0000_0400;
pub const FLAG_OF: u16 = 0x0000_0800;

static PARITY_LOOKUP: [u16; 256] = [
    FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF,
    0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0,
    0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0,
    FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF,
    0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0,
    FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF,
    FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF,
    0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0,
    0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0,
    FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF,
    FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF,
    0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0,
    FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF,
    0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0,
    0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0,
    FLAG_PF, 0, 0, FLAG_PF, 0, FLAG_PF, FLAG_PF, 0, 0, FLAG_PF, FLAG_PF, 0, FLAG_PF, 0, 0, FLAG_PF
];

impl Flags {
    pub fn new() -> Self {
        Flags {
            carry: false, // bit 0
            reserved1: false,
            parity: false,
            reserved3: false,
            adjust: false,
            reserved5: false,
            zero: false,
            sign: false, // bit 7
            trap: false,
            interrupt: false,
            direction: false,
            overflow: false,
            iopl12: false,
            iopl13: false,
            nested_task: false,
            reserved15: false, // bit 15
        }
    }

    pub fn new_from_u16(val: u16) -> Flags {
        let mut f = Flags::new();
        f.set_u16(val);
        f
    }

    /// sets sign, zero, parity flags according to `b`
    pub fn set_szp(&mut self, b: bool) {
        self.sign = b;
        self.zero = b;
        self.parity = b;
    }

    /// Set equal to the most-significant bit of the result,
    /// which is the sign bit of a signed integer.
    /// (0 indicates a positive value and 1 indicates a negative value.)
    pub fn set_sign_u8(&mut self, v: usize) {
        self.sign = v & 0x80 != 0;
    }

    pub fn set_sign_u16(&mut self, v: usize) {
        self.sign = v & 0x8000 != 0;
    }

    pub fn set_sign_u32(&mut self, v: usize) {
        self.sign = v & 0x8000_0000 != 0;
    }

    /// Set if the least-significant byte of the result contains an
    /// even number of 1 bits; cleared otherwise.
    pub fn set_parity(&mut self, v: usize) {
        // TODO later: rework flag register to be a u16 directly, use FLAG_PF
        self.parity = PARITY_LOOKUP[v & 0xFF] != 0
    }

    /// Zero flag — Set if the result is zero; cleared otherwise.
    pub fn set_zero_u8(&mut self, v: usize) {
        self.zero = v.trailing_zeros() >= 8;
    }

    pub fn set_zero_u16(&mut self, v: usize) {
        self.zero = v.trailing_zeros() >= 16;
    }

    pub fn set_zero_u32(&mut self, v: usize) {
        self.zero = v.trailing_zeros() >= 32;
    }

    /// Set if an arithmetic operation generates a carry or a borrow out
    /// of bit 3 of the result; cleared otherwise. This flag is used in
    /// binary-coded decimal (BCD) arithmetic.
    pub fn set_adjust(&mut self, res: usize, v1: usize, v2: usize) {
        self.adjust = (res ^ (v1 ^ v2)) & 0x10 != 0;
    }

    /// Set if the integer result is too large a positive number or too
    /// small a negative number (excluding the sign-bit) to fit in the
    /// destination operand; cleared otherwise. This flag indicates an
    /// overflow condition for signed-integer (two’s complement) arithmetic.
    pub fn set_overflow_add_u8(&mut self, res: usize, v1: usize, v2: usize) {
        self.overflow = (res ^ v1) & (res ^ v2) & 0x80 != 0;
    }

    pub fn set_overflow_add_u16(&mut self, res: usize, v1: usize, v2: usize) {
        self.overflow = (res ^ v1) & (res ^ v2) & 0x8000 != 0;
    }

    pub fn set_overflow_add_u32(&mut self, res: usize, v1: usize, v2: usize) {
        self.overflow = (res ^ v1) & (res ^ v2) & 0x8000_0000 != 0;
    }

    pub fn set_overflow_sub_u8(&mut self, res: usize, v1: usize, v2: usize) {
        self.overflow = (v2 ^ v1) & (v2 ^ res) & 0x80 != 0;
    }

    pub fn set_overflow_sub_u16(&mut self, res: usize, v1: usize, v2: usize) {
        self.overflow = (v2 ^ v1) & (v2 ^ res) & 0x8000 != 0;
    }

    pub fn set_overflow_sub_u32(&mut self, res: usize, v1: usize, v2: usize) {
        self.overflow = (v2 ^ v1) & (v2 ^ res) & 0x8000_0000 != 0;
    }

    /// Set if an arithmetic operation generates a carry or a borrow out of
    /// the most-significant bit of the result; cleared otherwise. This flag
    /// indicates an overflow condition for unsigned-integer arithmetic.
    pub fn set_carry_u8(&mut self, res: usize) {
        self.carry = res & 0x100 != 0;
    }

    pub fn set_carry_u16(&mut self, res: usize) {
        self.carry = res & 0x1_0000 != 0;
    }

    pub fn set_carry_u32(&mut self, res: usize) {
        self.carry = res & 0x1_0000_0000 != 0;
    }

    /// initializes the flags with a packed u16
    pub fn set_u16(&mut self, val: u16) {
        self.carry       = val & 0x1 != 0;
        //self.reserved1   = val & 0x2 != 0;
        self.parity      = val & 0x4 != 0;
        self.adjust      = val & 0x10 != 0;
        self.zero        = val & 0x40 != 0;
        self.sign        = val & 0x80 != 0;
        self.trap        = val & 0x100 != 0;
        //self.interrupt   = val & 0x200 != 0;
        self.direction   = val & 0x400 != 0;
        self.overflow    = val & 0x800 != 0;
        //self.iopl12      = val & 0x1000 != 0;
        //self.iopl13      = val & 0x2000 != 0;
        //self.nested_task = val & 0x4000 != 0;
    }

    pub fn carry_val(&self) -> usize {
        if self.carry {
            1
        } else {
            0
        }
    }

    pub fn carry_numeric(&self) -> String {
        format!("{}", if self.carry {
            1
        } else {
            0
        })
    }

    pub fn zero_numeric(&self) -> String {
        format!("{}", if self.zero {
            1
        } else {
            0
        })
    }

    pub fn sign_numeric(&self) -> String {
        format!("{}", if self.sign { 1 } else { 0 })
    }

    pub fn overflow_numeric(&self) -> String {
        format!("{}", if self.overflow {
            1
        } else {
            0
        })
    }

    pub fn adjust_numeric(&self) -> String {
        format!("{}", if self.adjust {
            1
        } else {
            0
        })
    }

    pub fn parity_numeric(&self) -> String {
        format!("{}", if self.parity {
            1
        } else {
            0
        })
    }

    pub fn direction_numeric(&self) -> String {
        format!("{}", if self.direction {
            1
        } else {
            0
        })
    }

    pub fn interrupt_numeric(&self) -> String {
        format!("{}", if self.interrupt {
            1
        } else {
            0
        })
    }

    /// returns the FLAGS register
    pub fn u16(&self) -> u16 {
        let mut val = 0 as u16;
        if self.carry {
            val |= 1;
        }
        if self.reserved1 {
            val |= 1 << 1;
        }
        if self.parity {
            val |= 1 << 2;
        }
        if self.adjust {
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
        val
    }
}

