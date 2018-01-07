use memory::Memory;

pub struct MMU {
    memory: Memory
}

impl MMU {
    pub fn new() -> Self{
        MMU {
            memory: Memory::new()
        }
    }

    fn s_translate(&self, seg: u16, offset: u16) -> usize {
        let seg = seg as usize;
        let offset = offset as usize;

        seg * 16 + offset
    }

    pub fn read_u8(&self, seg: u16, offset: u16) -> u8 {
        self.memory.read_u8(self.s_translate(seg, offset))
    }

    pub fn read_u16(&self, seg: u16, offset: u16) -> u16 {
        self.memory.read_u16(self.s_translate(seg, offset))
    }

    pub fn write_u8(&mut self, seg: u16, offset: u16, data: u8) {
        let addr = self.s_translate(seg, offset);
        self.memory.write_u8(
            addr,
            data);
    }

    pub fn write_u16(&mut self, seg: u16, offset: u16, data: u16) {
        let addr = self.s_translate(seg, offset);
        self.memory.write_u16(
            addr,
            data);
    }

    pub fn read(&self, seg: u16, offset: u16, length: usize) -> &[u8] {

        let addr = self.s_translate(seg, offset);
        self.memory.read(addr, length)
    }

    pub fn write(&mut self, seg: u16, offset: u16, data: &[u8]) {
        let addr = self.s_translate(seg, offset);
        self.memory.write(addr, data);
    }
}
