use std::cmp;
use std::fmt;

/// represents a memory address inside the vm
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryAddress {
    /// a real mode segment:offset pair (0_0000 - F_FFFF)
    RealSegmentOffset(u16, u16),
    /// a long segment:offset pair (0000_0000 - FFFF_FFFF)
    LongSegmentOffset(u16, u16),
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
                panic!("XXX")
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
    /// translates a segment:offset pair to a physical (flat) address
    pub fn value(&self) -> u32 {
        match *self {
            MemoryAddress::RealSegmentOffset(seg, off) => (u32::from(seg) << 4) + u32::from(off),
            MemoryAddress::LongSegmentOffset(seg, off) => (u32::from(seg) << 16) + u32::from(off),
            _ => unreachable!(),
        }
    }

    pub fn segment(&self) -> u16 {
        match *self {
            MemoryAddress::RealSegmentOffset(seg, _) |
            MemoryAddress::LongSegmentOffset(seg, _) => seg,
            _ => unreachable!(),
        }
    }

    pub fn offset(&self) -> u16 {
        match *self {
            MemoryAddress::RealSegmentOffset(_, off) |
            MemoryAddress::LongSegmentOffset(_, off) => off,
            _ => unreachable!(),
        }
    }

    pub fn set_offset(&mut self, val: u16) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) |
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off = val,
            _ => unreachable!(),
        }
    }

    /// increase offset by 1
    pub fn inc_u8(&mut self) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) |
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += 1,
            _ => unreachable!(),
        }
    }

    /// increase offset by 2
    pub fn inc_u16(&mut self) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) |
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += 2,
            _ => unreachable!(),
        }
    }

    /// increase offset by 4
    pub fn inc_u32(&mut self) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) |
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += 4,
            _ => unreachable!(),
        }
    }

    /// increase offset by n
    pub fn inc_n(&mut self, n: u16) {
        match *self {
            MemoryAddress::RealSegmentOffset(_, ref mut off) |
            MemoryAddress::LongSegmentOffset(_, ref mut off) => *off += n,
            _ => unreachable!(),
        }
    }
}
