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
    pal: Vec<DACPalette>,           // the palette in use
    pub pel_address: u8,            // set by write to 03c8
    pel_component: usize,           // color component for next out 03c9, 0 = red, 1 = green, 2 = blue
}

impl GPU {
    pub fn new() -> Self {
        GPU {
            scanline: 0,
            width: 320,
            height: 200,
            pal: default_vga_palette(), // XXX use array all the time
            pel_address: 0,
            pel_component: 0,
        }
    }

    pub fn progress_scanline(&mut self) {
        // HACK to have a source of info to toggle CGA status register
        self.scanline += 1;
        if self.scanline > self.width {
            self.scanline = 0;
        }
    }

    // (VGA,MCGA) PEL address register (0x03C8)
    // Sets DAC in write mode and assign start of color register
    // index (0..255) for following write accesses to 3C9h.
    // Next access to 03C8h will stop pending mode immediately.
    pub fn set_pel_address(&mut self, val: u8) {
        self.pel_address = val;
    }

    // (VGA,MCGA) PEL data register (0x03C9)
    // Three consecutive writes in the order: red, green, blue.
    // The internal DAC index is incremented on every 3rd write.
    pub fn set_pel_data(&mut self, val: u8) {
        // map 6-bit color into 8 bits
        let v = val << 2;

        match self.pel_component {
            0 => self.pal[self.pel_address as usize].r = v,
            1 => self.pal[self.pel_address as usize].g = v,
            2 => self.pal[self.pel_address as usize].b = v,
            _ => {}
        }

        self.pel_component += 1;

        if self.pel_component > 2 {
            self.pel_component = 0;
            self.pel_address = (Wrapping(self.pel_address) + Wrapping(1)).0;
        }
    }

    // sets the 0-255 intensity of the red color channel
    pub fn set_palette_r(&mut self, index: usize, val: u8) {
        self.pal[index].r = val;
    }

    // sets the 0-255 intensity of the green color channel
    pub fn set_palette_g(&mut self, index: usize, val: u8) {
        self.pal[index].g = val;
    }

    // sets the 0-255 intensity of the blue color channel
    pub fn set_palette_b(&mut self, index: usize, val: u8) {
        self.pal[index].b = val;
    }
}
