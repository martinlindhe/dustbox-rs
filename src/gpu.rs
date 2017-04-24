use orbtk::{Image, Color};
use std::sync::Arc;

pub struct GPU {
    pub scanline: u32,
    pub width: u32,
    pub height: u32,
    pub palette: Vec<DACPalette>,
    pub dac_color: usize, // for out 03c9, 0 = red, 1 = green, 2 = blue
    pub dac_index: u8, // for out 03c9
    pub dac_current_palette: Vec<u8>, // for out 03c9
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
            dac_color: 0,
            dac_index: 0,
            dac_current_palette: vec![0u8; 3],
        }
    }
    pub fn progress_scanline(&mut self) {
        // HACK to have a source of info to toggle CGA status register
        self.scanline += 1;
        if self.scanline > self.width {
            self.scanline = 0;
        }
    }

    pub fn render_frame(&mut self) -> Arc<Image> {

        let canvas = Image::from_color(320, 200, Color::rgb(0, 0, 0));
        /*
            let height = dbg.cpu.gpu.height;
            let width = dbg.cpu.gpu.width;

            for y in 0..height {
                for x in 0..width {
                    let offset = 0xA0000 + ((y * width) + x) as usize;
                    let byte = dbg.cpu.memory.memory[offset];
                    let pal = &dbg.cpu.gpu.palette[byte as usize];
                    image.pixel(x as i32, y as i32, Color::rgb(pal.r, pal.g, pal.b));
                }
            }
        */
        canvas
    }
}
