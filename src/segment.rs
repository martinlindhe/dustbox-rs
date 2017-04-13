use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum Segment {
    CS(),
    DS(),
    ES(),
    SS(),
    GS(),
    Default(), // is treated as CS
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Segment::CS() => write!(f, "cs:"),
            Segment::DS() => write!(f, "ds:"),
            Segment::ES() => write!(f, "es:"),
            Segment::SS() => write!(f, "ss:"),
            Segment::GS() => write!(f, "gs:"),
            Segment::Default() => write!(f, ""),
        }
    }
}
