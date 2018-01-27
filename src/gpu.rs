use std::cell::RefCell;
use std::num::Wrapping;

use palette::{DACPalette, default_vga_palette};

#[cfg(test)]
#[path = "./gpu_test.rs"]
mod gpu_test;

#[derive(Clone, Default)]
pub struct GPU {
    pub scanline: u32,
    pub width: u32,
    pub height: u32,
    pub pal: Vec<DACPalette>,     // the palette in use
    pub dac_color: usize,         // for out 03c9, 0 = red, 1 = green, 2 = blue
    pub pel_address: u8,          // set by write to 03c8
    pub dac_current_pal: Vec<u8>, // for out 03c9
}

impl GPU {
    pub fn new() -> Self {
        GPU {
            scanline: 0,
            width: 320,
            height: 200,
            pal: default_vga_palette(), // XXX use array all the time
            dac_color: 0,
            pel_address: 0,
            dac_current_pal: vec![0u8; 3],
        }
    }

    pub fn progress_scanline(&mut self) {
        // HACK to have a source of info to toggle CGA status register
        self.scanline += 1;
        if self.scanline > self.width {
            self.scanline = 0;
        }
    }

    // (VGA,MCGA) PEL address register
    // Sets DAC in write mode and assign start of color register
    // index (0..255) for following write accesses to 3C9h.
    // Next access to 03C8h will stop pending mode immediately.
    pub fn set_pel_address(&mut self, val: u8) {
        self.pel_address = val;
    }

    // (VGA,MCGA) PEL data register
    // Three consecutive writes in the order: red, green, blue.
    // The internal DAC index is incremented on every 3rd write.
    pub fn set_pel_data(&mut self, val: u8) {
        if self.dac_color > 2 {
            let i = self.pel_address as usize;
            self.pal[i].r = self.dac_current_pal[0];
            self.pal[i].g = self.dac_current_pal[1];
            self.pal[i].b = self.dac_current_pal[2];

            if self.pel_address == 0 {
                println!("DAC palette {} = {}, {}, {}",
                        self.pel_address,
                        self.pal[i].r,
                        self.pal[i].g,
                        self.pal[i].b);
            }

            self.dac_color = 0;
            self.pel_address = (Wrapping(self.pel_address) + Wrapping(1)).0;
        }

        // map 6-bit color into 8 bits
        self.dac_current_pal[self.dac_color] = val << 2;

        self.dac_color += 1;
    }
}
