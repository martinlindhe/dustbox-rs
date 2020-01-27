use crate::memory::{FlatMemory, MemoryAddress};
use crate::codepage::cp437;

#[cfg(test)]
#[path = "./mmu_test.rs"]
mod mmu_test;

const DEBUG_MMU: bool = false;
const DEBUG_VEC: bool = false;

#[derive(Clone)]
pub struct MMU {
    pub memory: FlatMemory,

    /// the FLAGS register offset on stack while in interrupt
    pub flags_address: MemoryAddress,
}

impl MMU {
    pub fn default() -> Self{
        MMU {
            memory: FlatMemory::new(),
            flags_address: MemoryAddress::Unset,
        }
    }

    /// manipulates the FLAGS register on stack while in a interrupt
    pub fn set_flag(&mut self, flag_mask: u16, flag_value: bool) {
        if self.flags_address == MemoryAddress::Unset {
            panic!("bios: set_flag with 0 flags_address");
        }
        let mut flags = self.memory.read_u16(self.flags_address.value());
        if flag_value {
            flags |= flag_mask;
        } else {
            flags &= !flag_mask;
        }
        self.memory.write_u16(self.flags_address.value(), flags);
    }

    /// reads a sequence of data from memory
    pub fn read(&self, seg: u16, imm: u32, length: usize) -> Vec<u8> {
        let addr = MemoryAddress::RealSegmentOffset(seg, imm).value();
        Vec::from(self.memory.read(addr, length))
    }

    /// reads a sequence of data until a NULL byte is found
    pub fn readz(&self, seg: u16, imm: u32) -> Vec<u8> {
        let mut res = Vec::new();
        let mut addr = MemoryAddress::RealSegmentOffset(seg, imm);
        loop {
            let b = self.memory.read_u8(addr.value());
            if b == 0 {
                break;
            }
            res.push(b);
            addr.inc_u8();
        }
        res
    }

    /// reads a sequence of text until a NULL byte is found
    pub fn read_asciiz(&self, seg: u16, imm: u32) -> String {
        let mut res = String::new();
        let mut addr = MemoryAddress::RealSegmentOffset(seg, imm);
        loop {
            let b = self.memory.read_u8(addr.value());
            if b == 0 {
                break;
            }
            res.push(cp437::u8_as_char(b));
            addr.inc_u8();
        }
        res
    }

    /// reads a sequence of text until a $ terminator is found
    pub fn read_asciid(&self, seg: u16, imm: u32) -> String {
        let mut res = String::new();
        let mut addr = MemoryAddress::RealSegmentOffset(seg, imm);
        loop {
            let b = self.memory.read_u8(addr.value());
            if b == b'$' {
                break;
            }
            res.push(cp437::u8_as_char(b));
            addr.inc_u8();
        }
        res
    }

    pub fn read_u8_addr(&self, addr: MemoryAddress) -> u8 {
        let v = self.memory.read_u8(addr.value());
        if DEBUG_MMU {
            println!("mmu.read_u8_addr from {} = {:02X}", addr, v);
        }
        v
    }

    pub fn read_u8(&self, seg: u16, imm: u32) -> u8 {
        let addr = MemoryAddress::RealSegmentOffset(seg, imm).value();
        if addr > self.memory.data.len() as u32 {
            panic!("read_u8 FATAL out of bounds read from {:04X}:{:04X} == {:06X}", seg, imm, addr);
            return 0;
        }
        let v = self.memory.read_u8(addr);
        if DEBUG_MMU {
            println!("mmu.read_u8 from ({:04X}:{:04X} == {:06X}) = {:02X}", seg, imm, addr, v);
        }
        v
    }

    pub fn read_u16(&self, seg: u16, imm: u32) -> u16 {
        let addr = MemoryAddress::RealSegmentOffset(seg, imm).value();
        if addr > self.memory.data.len() as u32 {
            panic!("read_u8 FATAL out of bounds read from {:04X}:{:04X} == {:06X}", seg, imm, addr);
            return 0;
        }
        let v = self.memory.read_u16(addr);
        if DEBUG_MMU {
            println!("mmu.read_u16 from ({:04X}:{:04X} == {:06X}) = {:04X}", seg, imm, addr, v);
        }
        v
    }

    /// reads a 16-bit value from a 32-bit offset
    pub fn read_u16_32(&self, seg: u16, imm: u32) -> u16 {
        let addr = MemoryAddress::LongSegmentOffset(seg, imm).value();
        let v = self.memory.read_u16(addr);
        if DEBUG_MMU {
            println!("mmu.read_u16_32 from ({:04X}:{:04X} == {:06X}) = {:04X}", seg, imm, addr, v);
        }
        v
    }

    pub fn write_u8(&mut self, seg: u16, imm: u32, data: u8) {
        let addr = MemoryAddress::RealSegmentOffset(seg, imm).value();
        if DEBUG_MMU {
            println!("mmu.write_u8 to ({:04X}:{:04X} == {:06X}) = {:02X}", seg, imm, addr, data);
        }
        self.memory.write_u8(addr, data);
    }

    /// write data and increase addr
    pub fn write_u8_inc(&mut self, addr: &mut MemoryAddress, data: u8) {
        self.memory.write_u8(addr.value(), data);
        if DEBUG_MMU {
            println!("mmu.write_u8_inc to {:06X} = {:02X}", addr.value(), data);
        }
        addr.inc_u8();
    }

    /// writes a sequence of data to memory
    pub fn write(&mut self, seg: u16, imm: u32, data: &[u8]) {
        let addr = MemoryAddress::RealSegmentOffset(seg, imm).value();
        self.memory.write(addr, data);
    }

    pub fn write_u16(&mut self, seg: u16, imm: u32, data: u16) {
        let addr = MemoryAddress::RealSegmentOffset(seg, imm).value();
        if DEBUG_MMU {
            println!("mmu.write_u16 to ({:04X}:{:04X} == {:06X}) = {:02X}", seg, imm, addr, data);
        }
        self.memory.write_u16(addr, data);
    }

    /// write data and increase addr
    pub fn write_u16_inc(&mut self, addr: &mut MemoryAddress, data: u16) {
        self.memory.write_u16(addr.value(), data);
        if DEBUG_MMU {
            println!("mmu.write_u16_inc to {:06X} = {:08X}", addr.value(), data);
        }
        addr.inc_u16();
    }

    pub fn read_u32(&self, seg: u16, imm: u32) -> u32 {
        let addr = MemoryAddress::RealSegmentOffset(seg, imm).value();
        let v = self.memory.read_u32(addr);
        if DEBUG_MMU {
            println!("mmu.read_u32 from {:06X} = {:04X}", addr, v);
        }
        v
    }

    pub fn write_u32(&mut self, seg: u16, imm: u32, data: u32) {
        // TODO take MemoryAddress parameter directly
        let addr = MemoryAddress::RealSegmentOffset(seg, imm).value();
        if DEBUG_MMU {
            println!("mmu.write_u32 to {:06X} = {:08X}", addr, data);
        }
        self.memory.write_u32(addr, data);
    }

    /// write data and increase addr
    pub fn write_u32_inc(&mut self, addr: &mut MemoryAddress, data: u32) {
        self.memory.write_u32(addr.value(), data);
        if DEBUG_MMU {
            println!("mmu.write_u32_inc to {:06X} = {:08X}", addr.value(), data);
        }
        addr.inc_u32();
    }

    /// read interrupt vector, returns segment, offset
    pub fn read_vec(&self, v: u16) -> (u16, u32) {
        // XXX better naming
        let v_abs = (v as u32) << 2;
        let seg = self.memory.read_u16(v_abs);
        let off = self.memory.read_u16(v_abs + 2) as u32;
        if DEBUG_VEC {
            println!("mmu.read_vec: {:04X} = {:04X}:{:04X}", v, seg, off);
        }
        (seg, off)
    }

    /// write interrupt vector
    pub fn write_vec(&mut self, v: u16, data: MemoryAddress) {
        let v_abs = u32::from(v) << 2;
        self.memory.write_u16(v_abs, data.segment());
        self.memory.write_u16(v_abs + 2, data.offset() as u16);
        if DEBUG_VEC {
            println!("mmu.write_vec: {:04X} = {:04X}:{:04X}", v, data.segment(), data.offset());
        }
    }
}
