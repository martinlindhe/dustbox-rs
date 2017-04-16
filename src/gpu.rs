//use piston_window::*;
//use image::*;

//use memory::Memory;

pub struct GPU {
    pub scanline: u32,
    pub width: u32,
    pub height: u32,
    pub palette: Vec<DACPalette>,
}

#[derive(Clone)]
pub struct DACPalette {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl GPU {
    pub fn new() -> Self {
        let (width, height) = (320, 200);
        GPU {
            scanline: 0,
            width: width,
            height: height,
            palette: vec![DACPalette { r: 0, g: 0, b: 0 }; 256],
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
