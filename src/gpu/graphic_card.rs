// GraphicCard indicates the gfx card generation to emulate
#[derive(Clone, Debug, PartialEq)]
pub enum GraphicCard {
    CGA, EGA, VGA, Tandy,
}

impl GraphicCard {
     pub fn is_ega_vga(&self) -> bool {
        match *self {
            GraphicCard::EGA | GraphicCard::VGA => true,
            _ => false,
        }
    }
    pub fn is_tandy(&self) -> bool {
        match *self {
            GraphicCard::Tandy => true,
            _ => false,
        }
    }
    pub fn is_cga(&self) -> bool {
        match *self {
            GraphicCard::CGA => true,
            _ => false,
        }
    }
    pub fn is_ega(&self) -> bool {
        match *self {
            GraphicCard::EGA => true,
            _ => false,
        }
    }
    pub fn is_vga(&self) -> bool {
        match *self {
            GraphicCard::VGA => true,
            _ => false,
        }
    }
}
