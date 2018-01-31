use std::fmt;

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
}

// translates a segment:offset address into a flat address
pub fn as_flat_address(segment: u16, offset: u16) -> usize {
    (segment as usize * 16) + offset as usize
}
