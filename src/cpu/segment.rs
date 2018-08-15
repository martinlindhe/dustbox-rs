use std::fmt;

use cpu::register::R;

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
            Segment::Default | Segment::DS => "ds",
            Segment::CS => "cs",
            Segment::ES => "es",
            Segment::FS => "fs",
            Segment::GS => "gs",
            Segment::SS => "ss",
        }
    }

    pub fn as_register(self) -> R {
        match self {
            Segment::Default | Segment::DS => R::DS,
            Segment::CS => R::CS,
            Segment::ES => R::ES,
            Segment::FS => R::FS,
            Segment::GS => R::GS,
            Segment::SS => R::SS,
        }
    }
}
