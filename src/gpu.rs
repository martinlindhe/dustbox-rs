use std::cell::RefCell;

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

#[derive(Clone, Default)]
pub struct DACPalette {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl GPU {
    pub fn new() -> Self {
        let mut pal = vec![DACPalette { r: 0, g: 0, b: 0 }; 256];
        // the standard EGA palette
        pal[0]  = DACPalette {r: 0, g: 0, b: 0};       // black
        pal[1]  = DACPalette {r: 0, g: 0, b: 170};     // blue
        pal[2]  = DACPalette {r: 0, g: 170, b: 0};     // green
        pal[3]  = DACPalette {r: 0, g: 170, b: 170};   // cyan
        pal[4]  = DACPalette {r: 170, g: 0, b: 0};     // red
        pal[5]  = DACPalette {r: 170, g: 0, b: 170};   // magenta
        pal[6]  = DACPalette {r: 170, g: 85, b: 0};    // brown
        pal[7]  = DACPalette {r: 170, g: 170, b: 170}; // gray
        pal[8]  = DACPalette {r: 85, g: 85, b: 85};    // dark gray
        pal[9]  = DACPalette {r: 85, g: 85, b: 255};   // bright blue
        pal[10] = DACPalette {r: 85, g: 255, b: 85};   // bright green
        pal[11] = DACPalette {r: 85, g: 255, b: 255};  // bright cyan
        pal[12] = DACPalette {r: 255, g: 85, b: 85};   // bright red
        pal[13] = DACPalette {r: 255, g: 85, b: 255};  // bright magenta
        pal[14] = DACPalette {r: 255, g: 255, b: 85};  // yellow
        pal[15] = DACPalette {r: 255, g: 255, b: 255}; // white
        GPU {
            scanline: 0,
            width: 320,
            height: 200,
            pal: pal,
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
