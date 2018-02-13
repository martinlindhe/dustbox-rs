use memory::FlatMemory;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Default)]
pub struct MMU {
    pub memory: Rc<RefCell<FlatMemory>>
}

impl MMU {
    pub fn new() -> Self{
        MMU {
            memory: Rc::new(RefCell::new(FlatMemory::new()))
        }
    }

    // translates a segment:offset pair to a physical address
    pub fn to_flat(seg: u16, offset: u16) -> u32 {
        (seg as u32) * 16 + (offset as u32)
    }

    pub fn read_u8(&self, seg: u16, offset: u16) -> u8 {
        self.memory.borrow().read_u8(MMU::to_flat(seg, offset))
    }

    pub fn read_u16(&self, seg: u16, offset: u16) -> u16 {
        self.memory.borrow().read_u16(MMU::to_flat(seg, offset))
    }

    pub fn write_u8(&mut self, seg: u16, offset: u16, data: u8) {
        let addr = MMU::to_flat(seg, offset);
        self.memory.borrow_mut().write_u8(addr, data);
    }

    pub fn write(&mut self, seg: u16, offset: u16, data: &[u8]) {
        let addr = MMU::to_flat(seg, offset);
        self.memory.borrow_mut().write(addr, data);
    }

    pub fn write_u16(&mut self, seg: u16, offset: u16, data: u16) {
        let addr = MMU::to_flat(seg, offset);
        self.memory.borrow_mut().write_u16(addr, data);
    }

    pub fn write_u32(&mut self, seg: u16, offset: u16, data: u32) {
        let addr = MMU::to_flat(seg, offset);
        self.memory.borrow_mut().write_u32(addr, data);
    }

    pub fn read(&self, seg: u16, offset: u16, length: usize) -> Vec<u8> {
        let addr = MMU::to_flat(seg, offset);
        Vec::from(self.memory.borrow().read(addr, length))
    }

    pub fn set_vec(&mut self, v: u32, data: u32) {
        self.memory.borrow_mut().write_u32(v << 2, data);
    }

    pub fn dump_mem(&self) -> Vec<u8> {
        self.memory.borrow().memory.clone()
    }
}
