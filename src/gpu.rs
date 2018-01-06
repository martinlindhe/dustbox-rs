use std::cell::RefCell;

use gdk::prelude::*;
use gdk_pixbuf;
use cairo;
use raster;

#[cfg(test)]
#[path = "./gpu_test.rs"]
mod gpu_test;

#[derive(Clone)]
pub struct GPU {
    pub scanline: i32,
    pub width: i32,
    pub height: i32,
    pub pal: Vec<DACPalette>,     // the palette in use
    pub dac_color: usize,         // for out 03c9, 0 = red, 1 = green, 2 = blue
    pub dac_index: u8,            // for out 03c9
    pub dac_current_pal: Vec<u8>, // for out 03c9
}

#[derive(Clone)]
pub struct DACPalette {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl GPU {
    pub fn new() -> Self {
        GPU {
            scanline: 0,
            width: 320,
            height: 200,
            pal: vec![DACPalette { r: 0, g: 0, b: 0 }; 256],
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

    // render current video to canvas `c`
    pub fn draw_canvas(&self, c: &cairo::Context, memory: &[u8]) {
        let colorspace = 0; // XXX: gdk_pixbuf_sys::GDK_COLORSPACE_RGB = 0

        // XXX FIXME +1 hack because of off-by-1 bug accessing the pixbuf
        let buf = unsafe { gdk_pixbuf::Pixbuf::new(colorspace, false, 8, self.width, self.height + 1) }.unwrap();
        // println!("draw_canvas: buf w {}, h {}. video w {}, h {}", buf.get_width(), buf.get_height(), self.width, self.height);

        for y in 0..self.height {
            for x in 0..self.width {
                let offset = 0xA_0000 + ((y * self.width) + x) as usize;
                let byte = memory[offset];
                let pal = &self.pal[byte as usize];
                buf.put_pixel(x, y, pal.r, pal.g, pal.b, 255);
            }
        }

        c.set_source_pixbuf(&buf, 0., 0.);
        c.paint();
    }

    pub fn draw_image(&self, memory: &[u8]) -> raster::Image {
        let mut canvas = raster::Image::blank(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let offset = 0xA_0000 + ((y * self.width) + x) as usize;
                let byte = memory[offset];
                let pal = &self.pal[byte as usize];
                canvas
                    .set_pixel(x, y, raster::Color::rgba(pal.r, pal.g, pal.b, 255))
                    .unwrap();
            }
        }
        canvas
    }

    // used by gpu tests
    pub fn test_render_frame(&self, memory: &[u8], pngfile: &str) {
        let img = self.draw_image(memory);
        match raster::open(pngfile) {
            Ok(v) => {
                // alert if output has changed. NOTE: output change is not nessecary a bug
                if !raster::compare::equal(&v, &img).unwrap() {
                    println!("WARNING: Writing changed gpu test result to {}", pngfile);
                    if let Err(why) = raster::save(&img, pngfile) {
                        println!("save err: {:?}", why);
                    }
                }
            }
            Err(_) => {
                println!("Writing initial gpu test result to {}", pngfile);
                if let Err(why) = raster::save(&img, pngfile) {
                    println!("save err: {:?}", why);
                }
            }
        };
    }
}
