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

fn default_vga_palette() -> Vec<DACPalette> {
    let pal = [
        // 0-15: EGA palette
        DACPalette {r: 0x00, g: 0x00, b: 0x00}, // 0. black
        DACPalette {r: 0x00, g: 0x00, b: 0xAA}, // 1. blue
        DACPalette {r: 0x00, g: 0xAA, b: 0x00}, // 2. green
        DACPalette {r: 0x00, g: 0xAA, b: 0xAA}, // 3. cyan
        DACPalette {r: 0xAA, g: 0x00, b: 0x00}, // 4. red
        DACPalette {r: 0xAA, g: 0x00, b: 0xAA}, // 5. magenta
        DACPalette {r: 0xAA, g: 0x55, b: 0x00}, // 6. brown
        DACPalette {r: 0xAA, g: 0xAA, b: 0xAA}, // 7. gray
        DACPalette {r: 0x55, g: 0x55, b: 0x55}, // 8. dark gray
        DACPalette {r: 0x55, g: 0x55, b: 0xFF}, // 9. bright blue
        DACPalette {r: 0x55, g: 0xFF, b: 0x55}, // 10. bright green
        DACPalette {r: 0x55, g: 0xFF, b: 0xFF}, // 11. bright cyan
        DACPalette {r: 0xFF, g: 0x55, b: 0x55}, // 12. bright red
        DACPalette {r: 0xFF, g: 0x55, b: 0xFF}, // 13. bright magenta
        DACPalette {r: 0xFF, g: 0xFF, b: 0x55}, // 14. yellow
        DACPalette {r: 0xFF, g: 0xFF, b: 0xFF}, // 15. white

        // 16-31: gray scale
        DACPalette {r: 0x00, g: 0x00, b: 0x00},
        DACPalette {r: 0x14, g: 0x14, b: 0x14},
        DACPalette {r: 0x20, g: 0x20, b: 0x20},
        DACPalette {r: 0x2C, g: 0x2C, b: 0x2C},
        DACPalette {r: 0x38, g: 0x38, b: 0x38},
        DACPalette {r: 0x45, g: 0x45, b: 0x45},
        DACPalette {r: 0x51, g: 0x51, b: 0x51},
        DACPalette {r: 0x61, g: 0x61, b: 0x61},
        DACPalette {r: 0x71, g: 0x71, b: 0x71},
        DACPalette {r: 0x82, g: 0x82, b: 0x82},
        DACPalette {r: 0x92, g: 0x92, b: 0x92},
        DACPalette {r: 0xA2, g: 0xA2, b: 0xA2},
        DACPalette {r: 0xB6, g: 0xB6, b: 0xB6},
        DACPalette {r: 0xCB, g: 0xCB, b: 0xCB},
        DACPalette {r: 0xE3, g: 0xE3, b: 0xE3},
        DACPalette {r: 0xFF, g: 0xFF, b: 0xFF},

        // 32-55: rainbow
        DACPalette {r: 0x00, g: 0x00, b: 0xFF}, // blue
        DACPalette {r: 0x41, g: 0x00, b: 0xFF},
        DACPalette {r: 0x7D, g: 0x00, b: 0xFF},
        DACPalette {r: 0xBE, g: 0x00, b: 0xFF},
        DACPalette {r: 0xFF, g: 0x00, b: 0xFF}, // magenta
        DACPalette {r: 0xFF, g: 0x00, b: 0xBE},
        DACPalette {r: 0xFF, g: 0x00, b: 0x7D},
        DACPalette {r: 0xFF, g: 0x00, b: 0x41},
        DACPalette {r: 0xFF, g: 0x00, b: 0x00}, // red
        DACPalette {r: 0xFF, g: 0x41, b: 0x00},
        DACPalette {r: 0xFF, g: 0x7D, b: 0x00},
        DACPalette {r: 0xFF, g: 0xBE, b: 0x00},
        DACPalette {r: 0xFF, g: 0xFF, b: 0x00}, // yellow
        DACPalette {r: 0xBE, g: 0xFF, b: 0x00},
        DACPalette {r: 0x7D, g: 0xFF, b: 0x00},
        DACPalette {r: 0x41, g: 0xFF, b: 0x00},
        DACPalette {r: 0x00, g: 0xFF, b: 0x00}, // green
        DACPalette {r: 0x00, g: 0xFF, b: 0x41},
        DACPalette {r: 0x00, g: 0xFF, b: 0x7D},
        DACPalette {r: 0x00, g: 0xFF, b: 0xBE},
        DACPalette {r: 0x00, g: 0xFF, b: 0xFF}, // cyan
        DACPalette {r: 0x00, g: 0xBE, b: 0xFF},
        DACPalette {r: 0x00, g: 0x7D, b: 0xFF},
        DACPalette {r: 0x00, g: 0x41, b: 0xFF},
    ];

    let mut out = vec![DACPalette { r: 0, g: 0, b: 0 }; 256];
    for (i, el) in pal.iter().enumerate() {
        out[i] = el.clone();
    }

    for i in 56..80 {
        // XXX 56-79: 49% whitemix of 32-55   (video 01:38)
        let el = out[i-24].clone();
        out[i] = el;
    }

    for i in 80..104 {
        // XXX 80-103: 72% whitemix of 32-55
        let el = out[i-48].clone();
        out[i] = el;
    }

    for i in 104..176 {
        // XXX 104-175: 56% blackmix of 32-103
        let el = out[i-72].clone();
        out[i] = el;
    }
    
    for i in 176..248 {
        // XXX 176-247: 75% blackmix of 32-103
        let el = out[i-144].clone();
        out[i] = el;
    }

    // 248-255: all black

    out
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
