use memory::FlatMemory;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(test)]
#[path = "./mmu_test.rs"]
mod mmu_test;

const DEBUG_VEC: bool = false;

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

    // reads a sequence of data from memory
    pub fn read(&self, seg: u16, offset: u16, length: usize) -> Vec<u8> {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        Vec::from(self.memory.borrow().read(addr, length))
    }

    pub fn read_u8(&self, seg: u16, offset: u16) -> u8 {
        self.memory.borrow().read_u8(MemoryAddress::RealSegmentOffset(seg, offset).value())
    }

    pub fn read_u16(&self, seg: u16, offset: u16) -> u16 {
        self.memory.borrow().read_u16(MemoryAddress::RealSegmentOffset(seg, offset).value())
    }

    pub fn write_u8(&mut self, seg: u16, offset: u16, data: u8) {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        self.memory.borrow_mut().write_u8(addr, data);
    }

    // writes a sequence of data to memory
    pub fn write(&mut self, seg: u16, offset: u16, data: &[u8]) {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        self.memory.borrow_mut().write(addr, data);
    }

    pub fn write_u16(&mut self, seg: u16, offset: u16, data: u16) {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        self.memory.borrow_mut().write_u16(addr, data);
    }

    pub fn write_u32(&mut self, seg: u16, offset: u16, data: u32) {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        self.memory.borrow_mut().write_u32(addr, data);
    }

    /// read interrupt vector, returns segment, offset
    pub fn read_vec(&self, v: u16) -> (u16, u16) {
        let v_abs = (v as u32) << 2;
        let seg = self.memory.borrow().read_u16(v_abs);
        let off = self.memory.borrow().read_u16(v_abs + 2);
        if DEBUG_VEC {
            println!("mmu.read_vec: {:04X} = {:04X}:{:04X}", v, seg, off);
        }
        (seg, off)
    }

    /// write interrupt vector
    pub fn write_vec(&mut self, v: u16, data: &MemoryAddress) {
        let v_abs = (v as u32) << 2;
        //self.memory.borrow_mut().write_u32((v as u32) << 2, data.value());
        self.memory.borrow_mut().write_u16(v_abs, data.segment());
        self.memory.borrow_mut().write_u16(v_abs + 2, data.offset());
        if DEBUG_VEC {
            println!("mmu.write_vec: {:04X} = {:04X}:{:04X}", v, data.segment(), data.offset());
        }
    }

    pub fn dump_mem(&self) -> Vec<u8> {
        self.memory.borrow().memory.clone()
    }
}

// represents a memory address inside the vm
#[derive(Clone, Debug, PartialEq)]
pub enum MemoryAddress {
    /// a real mode segment:offset pair (0_0000 - F_FFFF)
    RealSegmentOffset(u16, u16),
    /// a long segment:offset pair (0000_0000 - FFFF_FFFF)
    LongSegmentOffset(u16, u16),
    /// a unknown value
    Unset,
}

impl MemoryAddress {
    /// translates a segment:offset pair to a physical (flat) address
    pub fn value(&self) -> u32 {
        match *self {
            MemoryAddress::RealSegmentOffset(seg, off) => ((seg as u32) << 4) + off as u32,
            MemoryAddress::LongSegmentOffset(seg, off) => ((seg as u32) << 16) + (off as u32),
            _ => panic!("unhandled type {:?}", self),
        }
    }

    pub fn segment(&self) -> u16 {
         match *self {
            MemoryAddress::RealSegmentOffset(seg, _) |
            MemoryAddress::LongSegmentOffset(seg, _) => seg,
            _ => panic!("unhandled type {:?}", self),
        }
    }

    pub fn offset(&self) -> u16 {
         match *self {
            MemoryAddress::RealSegmentOffset(_, off) |
            MemoryAddress::LongSegmentOffset(_, off) => off,
            _ => panic!("unhandled type {:?}", self),
        }
    }
}
