use memory::FlatMemory;
use std::cell::RefCell;
use std::rc::Rc;

use memory::MemoryAddress;

#[cfg(test)]
#[path = "./mmu_test.rs"]
mod mmu_test;

const DEBUG_MMU: bool = false;
const DEBUG_VEC: bool = false;

#[derive(Clone, Default)]
pub struct MMU {
    pub memory: Rc<RefCell<FlatMemory>>
}

impl MMU {
    pub fn default() -> Self{
        MMU {
            memory: Rc::new(RefCell::new(FlatMemory::new()))
        }
    }

    // reads a sequence of data from memory
    pub fn read(&self, seg: u16, offset: u16, length: usize) -> Vec<u8> {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        Vec::from(self.memory.borrow().read(addr, length))
    }

    // reads a sequence of data until a NULL byte is found
    pub fn readz(&self, seg: u16, offset: u16) -> Vec<u8> {
        let mut res = Vec::new();
        let mut addr = MemoryAddress::RealSegmentOffset(seg, offset);
        loop {
            let b = self.memory.borrow().read_u8(addr.value());
            if b == 0 {
                break;
            }
            res.push(b);
            addr.inc_u8();
        }
        res
    }

    pub fn read_u8(&self, seg: u16, offset: u16) -> u8 {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        let v = self.memory.borrow().read_u8(addr);
        if DEBUG_MMU {
            println!("mmu.read_u8 from {:06X} = {:02X}", addr, v);
        }
        v
    }

    pub fn read_u16(&self, seg: u16, offset: u16) -> u16 {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        let v = self.memory.borrow().read_u16(addr);
        if DEBUG_MMU {
            println!("mmu.read_u16 from {:06X} = {:04X}", addr, v);
        }
        v
    }

    pub fn write_u8(&mut self, seg: u16, offset: u16, data: u8) {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        if DEBUG_MMU {
            println!("mmu.write_u8 to {:06X} = {:02X}", addr, data);
        }
        self.memory.borrow_mut().write_u8(addr, data);
    }

    /// writes and increments offset
    pub fn write_u8_inc(&mut self, addr: &mut MemoryAddress, data: u8) {
        self.memory.borrow_mut().write_u8(addr.value(), data);
        if DEBUG_MMU {
            println!("mmu.write_u8_inc to {:06X} = {:02X}", addr.value(), data);
        }
        addr.inc_u8();
    }

    // writes a sequence of data to memory
    pub fn write(&mut self, seg: u16, offset: u16, data: &[u8]) {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        self.memory.borrow_mut().write(addr, data);
    }

    pub fn write_u16(&mut self, seg: u16, offset: u16, data: u16) {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        if DEBUG_MMU {
            println!("mmu.write_u16 to {:06X} = {:04X}", addr, data);
        }
        self.memory.borrow_mut().write_u16(addr, data);
    }

    pub fn write_u16_inc(&mut self, addr: &mut MemoryAddress, data: u16) {
        self.memory.borrow_mut().write_u16(addr.value(), data);
        if DEBUG_MMU {
            println!("mmu.write_u16_inc to {:06X} = {:08X}", addr.value(), data);
        }
        addr.inc_u16();
    }

    pub fn read_u32(&self, seg: u16, offset: u16) -> u32 {
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        let v = self.memory.borrow().read_u32(addr);
        if DEBUG_MMU {
            println!("mmu.read_u32 from {:06X} = {:04X}", addr, v);
        }
        v
    }

    pub fn write_u32(&mut self, seg: u16, offset: u16, data: u32) {
        // TODO take MemoryAddress parameter directly
        let addr = MemoryAddress::RealSegmentOffset(seg, offset).value();
        if DEBUG_MMU {
            println!("mmu.write_u32 to {:06X} = {:08X}", addr, data);
        }
        self.memory.borrow_mut().write_u32(addr, data);
    }

    pub fn write_u32_inc(&mut self, addr: &mut MemoryAddress, data: u32) {
        self.memory.borrow_mut().write_u32(addr.value(), data);
        if DEBUG_MMU {
            println!("mmu.write_u32_inc to {:06X} = {:08X}", addr.value(), data);
        }
        addr.inc_u32();
    }

    /// read interrupt vector, returns segment, offset
    pub fn read_vec(&self, v: u16) -> (u16, u16) {
        let v_abs = u32::from(v) << 2;
        let seg = self.memory.borrow().read_u16(v_abs);
        let off = self.memory.borrow().read_u16(v_abs + 2);
        if DEBUG_VEC {
            println!("mmu.read_vec: {:04X} = {:04X}:{:04X}", v, seg, off);
        }
        (seg, off)
    }

    /// write interrupt vector
    pub fn write_vec(&mut self, v: u16, data: &MemoryAddress) {
        let v_abs = u32::from(v) << 2;
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
