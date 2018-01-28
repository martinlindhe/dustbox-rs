use std::fmt;

use register;

#[derive(Debug, Copy, Clone)]
pub enum Segment {
    Default(),
    CS(),
    DS(),
    ES(),
    FS(),
    GS(),
    SS(),
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Segment::Default() => write!(f, ""),
            Segment::CS() => write!(f, "cs:"),
            Segment::DS() => write!(f, "ds:"),
            Segment::ES() => write!(f, "es:"),
            Segment::FS() => write!(f, "fs:"),
            Segment::GS() => write!(f, "gs:"),
            Segment::SS() => write!(f, "ss:"),
        }
    }
}

impl Segment {
    pub fn get_segment_register_index(&self) -> usize {
        match *self {
            Segment::Default() => register::DS,
            Segment::CS() => register::CS,
            Segment::DS() => register::DS,
            Segment::ES() => register::ES,
            Segment::FS() => register::FS,
            Segment::GS() => register::GS,
            Segment::SS() => register::SS,
        }
    }
}
