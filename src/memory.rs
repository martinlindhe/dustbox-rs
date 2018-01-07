#[derive(Clone)]
pub struct Memory {
    pub memory: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Memory { memory: vec![0u8; 0x1_0000 * 64] }
    }

    pub fn read_u8(&self, addr: usize) -> u8 {
        self.memory[addr]
    }

    pub fn read_u16(&self, addr: usize) -> u16 {
        u16::from(self.read_u8(addr)) << 8 |
            u16::from(self.read_u8(addr+1))
    }

    pub fn write_u8(&mut self, addr: usize, data: u8) {
        self.memory[addr] = data;
    }

    pub fn write_u16(&mut self, addr: usize, data: u16) {
        self.write_u8(addr, (data & 0xff) as u8);
        self.write_u8(addr+1, (data >> 8) as u8);
    }

    pub fn read(&self, addr: usize, length: usize) -> &[u8] {
        &self.memory[addr..addr+length]
    }

    pub fn write(&mut self, addr: usize, data: &[u8]) {
       self.memory[addr..addr+data.len()].copy_from_slice(data);
    }
}
