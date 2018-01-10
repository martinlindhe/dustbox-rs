use std::cell::RefCell;

use gdk::prelude::*;
use gdk_pixbuf;
use cairo;
use raster;

#[cfg(test)]
#[path = "./gpu_test.rs"]
mod gpu_test;

#[derive(Clone, Default)]
pub struct GPU {
    pub scanline: i32,
    pub width: i32,
    pub height: i32,
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

    // render video frame to canvas `c`
    pub fn draw_canvas(&self, c: &cairo::Context, memory: &[u8]) {
        let mut buf = vec![0u8; (self.width * self.height * 3) as usize];
        for y in 0..self.height {
            for x in 0..self.width {
                let offset = 0xA_0000 + ((y * self.width) + x) as usize;
                let byte = memory[offset];
                let pal = &self.pal[byte as usize];
                let i = ((y * self.width + x) * 3) as usize;
                buf[i] = pal.r;
                buf[i+1] = pal.g;
                buf[i+2] = pal.b;
            }
        }

        let pixbuf = gdk_pixbuf::Pixbuf::new_from_vec(buf, 0, false, 8, self.width, self.height, self.width * 3);
        c.set_source_pixbuf(&pixbuf, 0., 0.);
    }

    // render video frame as a raster::Image, used for saving video frame to disk
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
