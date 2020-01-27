use std::cmp;
use std::fmt;

/// represents a memory address inside the vm
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryAddress {
    /// a real mode segment:offset pair (0x0_0000 - 0xF_FFFF)
    RealSegmentOffset(u16, u32),

    /// a long segment:offset pair (0x0000_0000 - 0xFFFF_FFFF)
    LongSegmentOffset(u16, u32),

    /// a unknown value
    Unset,
}


impl fmt::Display for MemoryAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MemoryAddress::RealSegmentOffset(seg, off) => {
                write!(f, "{:04X}:{:04X}", seg, off)
            }
            MemoryAddress::LongSegmentOffset(seg, off) => {
                write!(f, "{:08X}:{:08X}", seg, off)
            }
            _ => unreachable!(),
        }
    }
}

impl PartialOrd for MemoryAddress {
    fn partial_cmp(&self, other: &MemoryAddress) -> Option<cmp::Ordering> {
        Some(other.cmp(self))
    }
}

impl Ord for MemoryAddress {
    fn cmp(&self, other: &MemoryAddress) -> cmp::Ordering {
        other.value().cmp(&self.value())
    }
}

impl MemoryAddress {
    pub fn default_real() -> MemoryAddress {
        MemoryAddress::RealSegmentOffset(0, 0)
    }

    /// translates a segment:offset pair to a physical (flat) address
    pub fn value(self) -> u32 {
        match self {
            MemoryAddress::RealSegmentOffset(seg, imm) => ((seg as u32) << 4).wrapping_add(imm),
            MemoryAddress::LongSegmentOffset(seg, imm) => ((seg as u32) << 16).wrapping_add(imm),
            _ => unreachable!(),
        }
    }

    pub fn segment(self) -> u16 {
        match self {
            MemoryAddress::RealSegmentOffset(seg, _) |
            MemoryAddress::LongSegmentOffset(seg, _) => seg,
            _ => unreachable!(),
        }
    }

    pub fn offset(self) -> u32 {
        match self {
            MemoryAddress::RealSegmentOffset(_, off) => off as u32,
            MemoryAddress::LongSegmentOffset(_, off) => off,
            _ => unreachable!(),
        }
    }

    /// set offset to `n`
    pub fn set_offset(&mut self, n: u32) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) => *off = n,
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off = n,
            _ => unreachable!(),
        }
    }

    /// add `n` to offset
    pub fn add_offset(&mut self, n: u16) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) => *off += n as u32,
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += n as u32,
            _ => unreachable!(),
        }
    }

    /// increase offset by 1
    pub fn inc_u8(&mut self) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) => *off += 1,
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += 1,
            _ => unreachable!(),
        }
    }

    /// increase offset by 2
    pub fn inc_u16(&mut self) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) => *off += 2,
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += 2,
            _ => unreachable!(),
        }
    }

    /// increase offset by 4
    pub fn inc_u32(&mut self) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) => *off += 4,
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += 4,
            _ => unreachable!(),
        }
    }

    /// increase offset by n
    pub fn inc_n(&mut self, n: u16) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) => *off += n as u32,
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += n as u32,
            _ => unreachable!(),
        }
    }
}
