use crate::gpu::palette::{ColorSpace, text_palette};
use crate::gpu::palette::ColorSpace::RGB;

const DEBUG_DAC: bool = false;

#[derive(Clone)]
pub struct DAC {
    /// DAC bits, usually 6 or 8
    bits: u8,

    pub pel_mask: u8,

    /// color component for next out 03c9, 0 = red, 1 = green, 2 = blue
    pub pel_index: u8,

    pub state: State,

    /// set by io write to 03c7
    pub read_index: u8,

    /// set by io write to 03c8
    pub write_index: u8,

    first_changed: usize,

    pub combine: [u8; 16],

    pub pal: Vec<ColorSpace>,

    pub hidac_counter: u8,

    reg02: u8,
}

impl Default for DAC {
    fn default() -> Self {
        DAC {
            bits: 0,
            pel_mask: 0xFF,
            pel_index: 0,
            state: State::Read,
            read_index: 0,
            write_index: 0,
            first_changed: 0,
            combine: [0; 16],
            pal: text_palette().to_vec(),
            hidac_counter: 0,
            reg02: 0,
        }
    }
}

impl DAC {
    /// (VGA) DAC state register (0x03C7)
    pub fn get_state(&mut self) -> u8 {
        self.hidac_counter = 0;
        let res = self.state.register();
        if DEBUG_DAC {
            println!("read port 03C7: get_state = {:02X}", res);
        }
        res
    }

    /// (VGA, MCGA) PEL mask register (0x03C6)
    pub fn set_pel_mask(&mut self, val: u8) {
        self.pel_mask = val;
    }

    /// (VGA,MCGA,CEG-VGA) PEL address register (read mode) (0x03C7)
    /// Sets DAC in read mode and assign start of color register
    /// index (0..255) for following read accesses to 3C9h.
    /// Don't write to 3C9h while in read mode. Next access to
    /// 03C8h will stop pending mode immediatly.
    pub fn set_pel_read_index(&mut self, val: u8) {
        self.state = State::Read;
        self.read_index = val;
        self.write_index = val + 1;
        self.pel_index = 0;
        self.hidac_counter = 0;
        if DEBUG_DAC {
            println!("write port 03C7: set_pel_read_index = {:02X}", val);
        }
    }

    /// (VGA,MCGA) PEL address register (0x03C8)
    pub fn get_pel_write_index(&mut self) -> u8 {
        self.hidac_counter = 0;
        if DEBUG_DAC {
            println!("read port 03C8: get_pel_write_index = {:02X}", self.write_index);
        }
        self.write_index
    }

    /// (VGA,MCGA) PEL address register (write mode) (0x03C8)
    /// Sets DAC in write mode and assign start of color register
    /// index (0..255) for following write accesses to 3C9h.
    /// Next access to 03C8h will stop pending mode immediately.
    pub fn set_pel_write_index(&mut self, val: u8) {
        self.state = State::Write;
        self.write_index = val;
        self.pel_index = 0;
        self.hidac_counter = 0;
        if DEBUG_DAC {
            println!("write port 03C8: set_pel_write_index = {:02X}", val);
        }
    }

    /// (VGA,MCGA) PEL data register (0x03C9)
    /// Three consequtive reads (in read mode) in the order: red, green, blue.
    /// The internal DAC index is incremented each 3rd access.
    pub fn get_pel_data(&mut self) -> u8 {
        self.hidac_counter = 0;
        let ret = match self.pal[self.read_index as usize] {
            RGB(r, g, b) => {
                match self.pel_index {
                    0 => {
                        self.pel_index = 1;
                        r >> 2
                    }
                    1 => {
                        self.pel_index = 2;
                        g >> 2
                    }
                    2 => {
                        self.pel_index = 0;
                        self.read_index += 1;
                        b >> 2
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };
        if DEBUG_DAC {
            println!("read port 03C9: get_pel_data = {:02X}", ret);
        }
        ret
    }

    /// (VGA,MCGA) PEL data register (0x03C9)
    /// Three consecutive writes (in write mode) in the order: red, green, blue.
    /// The internal DAC index is incremented on every 3rd access.
    pub fn set_pel_data(&mut self, mut val: u8) {
        val &= 0x3F;
        if DEBUG_DAC {
            println!("write port 03C9: set_pel_data = write index {:02X}, pel index {:02X} = {:02X}", self.write_index, self.pel_index, val);
        }
        // scale 6-bit color into 8 bits
        val <<= 2;

        self.hidac_counter = 0;
        if let RGB(ref mut r, ref mut g, ref mut b) = self.pal[self.write_index as usize] {
            match self.pel_index {
                0 => *r = val,
                1 => *g = val,
                2 => *b = val,
                _ => unreachable!(),
            }
        }

        self.pel_index += 1;
        if self.pel_index > 2 {
            // println!("self.write_index as usize  {}     len  {}", self.write_index as usize,self.pal.len() );
            if self.write_index as usize >= self.pal.len() - 1 {
                // println!("XXX dac write_index wrapped to 0 at {}", self.pal.len());
                self.write_index = 0;
            } else {
                self.write_index += 1;
            }
            self.pel_index = 0;
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum State {
    Read, Write,
}

impl State {
    /// encodes state for the DAC state register (0x03C7)
    pub fn register(&self) -> u8 {
        match *self {
            State::Read  => 0b11,
            State::Write => 0b00,
        }
    }
}
