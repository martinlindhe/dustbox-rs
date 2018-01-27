use std::cell::RefCell;

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
    pub dac_index: u8,            // for out 03c9
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
            dac_index: 0,
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
}
