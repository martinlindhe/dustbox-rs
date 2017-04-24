use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum Segment {
    CS(),
    DS(),
    ES(),
    FS(),
    GS(),
    SS(),
    Default(), // is treated as CS
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Segment::CS() => write!(f, "cs:"),
            Segment::DS() => write!(f, "ds:"),
            Segment::ES() => write!(f, "es:"),
            Segment::FS() => write!(f, "fs:"),
            Segment::GS() => write!(f, "gs:"),
            Segment::SS() => write!(f, "ss:"),
            Segment::Default() => write!(f, ""),
        }
    }
}
