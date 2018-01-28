use std::fmt;

use register;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Segment {
    Default,
    CS,
    DS,
    ES,
    FS,
    GS,
    SS,
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Segment {
    pub fn as_str(&self) -> &str {
        match *self {
            Segment::Default => "ds",
            Segment::CS => "cs",
            Segment::DS => "ds",
            Segment::ES => "es",
            Segment::FS => "fs",
            Segment::GS => "gs",
            Segment::SS => "ss",
        }
    }
}
