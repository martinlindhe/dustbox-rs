
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

    pub fn set_offset(&mut self, val: u16) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) |
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off = val,
            _ => panic!("unhandled type {:?}", self),
        }
    }

    pub fn inc_u8(&mut self) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) |
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += 1,
            _ => panic!("unhandled type {:?}", self),
        }
    }

    pub fn inc_u16(&mut self) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) |
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += 2,
            _ => panic!("unhandled type {:?}", self),
        }
    }

    pub fn inc_u32(&mut self) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) |
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += 4,
            _ => panic!("unhandled type {:?}", self),
        }
    }
}
