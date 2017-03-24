pub struct CPU {
    pc: u16,
    memory: [u8; 0x10000],
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            pc: 0,
            memory: [0; 0x10000],
        }
    }
    pub fn reset(&mut self) {
        self.pc = BASE_OFFSET;
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
