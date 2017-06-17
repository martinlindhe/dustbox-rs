use std::cell::RefCell;

use gdk::prelude::*;
use cairo;
use gdk_pixbuf;

#[derive(Clone)]
pub struct GPU {
    pub scanline: i32,
    pub width: i32,
    pub height: i32,
    pub palette: Vec<DACPalette>,
    pub dac_color: usize, // for out 03c9, 0 = red, 1 = green, 2 = blue
    pub dac_index: u8, // for out 03c9
    pub dac_current_palette: Vec<u8>, // for out 03c9
    pixbuf: RefCell<gdk_pixbuf::Pixbuf>,
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
        let colorspace = 0; // XXX: gdk_pixbuf_sys::GDK_COLORSPACE_RGB = 0

        GPU {
            scanline: 0,
            width: width,
            height: height,
            palette: vec![DACPalette { r: 0, g: 0, b: 0 }; 256],
            dac_color: 0,
            dac_index: 0,
            dac_current_palette: vec![0u8; 3],
            pixbuf: RefCell::new(unsafe {
                gdk_pixbuf::Pixbuf::new(colorspace, false, 8, width, height)
            }.unwrap()),
        }
    }
    pub fn progress_scanline(&mut self) {
        // HACK to have a source of info to toggle CGA status register
        self.scanline += 1;
        if self.scanline > self.width {
            self.scanline = 0;
        }
    }

    // render current video to canvas `c`
    pub fn draw_canvas(&self, c: &cairo::Context, memory: &Vec<u8>) {

        println!("draw canvas");

        let buf = self.pixbuf.borrow();

        for y in 0..self.height {
            for x in 0..self.width {
                let offset = 0xA0000 + ((y * self.width) + x) as usize;
                let byte = memory[offset];
                let pal = &self.palette[byte as usize];
                buf.put_pixel(x as i32, y as i32, pal.r, pal.g, pal.b, 255);
            }
        }

        c.set_source_pixbuf(&buf, self.width as f64, self.height as f64);
    }
}

