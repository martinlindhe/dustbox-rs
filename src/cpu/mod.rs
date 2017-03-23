const BASE_OFFSET: u16 = 0x100;

pub struct CPU {
    pub pc: u16,
    pub memory: [u8; 0x10000],
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            pc: BASE_OFFSET,
            memory: [0; 0x10000],
        }
    }
    pub fn reset(&mut self) {
        self.pc = BASE_OFFSET;
    }
    pub fn load_rom(&mut self, data: &Vec<u8>) {
        // XXX bail if data is > 64k
        for i in 0..data.len() {
            self.memory[(BASE_OFFSET as usize) + i] = data[i];
        }
    }
    pub fn disasm_instruction(&mut self) -> String {
        let b = self.memory[self.pc as usize];
        self.pc += 1;
        match b {
            0x48...0x4F => format!("dec {}", r16(b & 7)),
            0xB0...0xB7 => format!("mov {}, {:02X}", r8(b & 7), self.read_u8()),
            0xB8...0xBF => format!("mov {}, {:04X}", r16(b & 7), self.read_u16()),
            0xCD => format!("int {:02X}", self.read_u8()),
            _ => format!("UNHANDLED OP {:X}", b),
        }
    }
    fn read_u8(&mut self) -> u8 {
        let b = self.memory[self.pc as usize];
        self.pc += 1;
        b
    }
    fn read_u16(&mut self) -> u16 {
        let lo = self.read_u8();
        let hi = self.read_u8();
        (hi as u16) << 8 | lo as u16
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
