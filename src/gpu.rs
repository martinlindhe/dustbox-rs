use piston_window::*;
use image::*;

use memory::Memory;

pub struct GPU {
    pub scanline: u16,
    pub width: u32,
    pub height: u32,
    window: PistonWindow,
}

impl GPU {
    pub fn new() -> GPU {
        let (width, height) = (320, 200);
        GPU {
            scanline: 0,
            width: width,
            height: height,
            window: WindowSettings::new("x86emu", (width, height))
                .exit_on_esc(true)
                .opengl(OpenGL::V3_2)
                .build()
                .unwrap(),
        }
    }
    pub fn progress_scanline(&mut self, memory: &mut Memory) {
        // HACK to have a source of info to toggle CGA status register
        self.scanline += 1;
        if self.scanline > 100 {
            self.redraw_window(memory);
            self.scanline = 0;
        }
    }

    fn redraw_window(&mut self, memory: &mut Memory) {

        println!("redraw_window");

        let mut canvas = ImageBuffer::new(self.width, self.height);

        let mut texture =
            Texture::from_image(&mut self.window.factory, &canvas, &TextureSettings::new())
                .unwrap();

        // XXX use palette

        for y in 0..self.height {
            for x in 0..self.width {
                let offset = 0xA0000 + ((y * self.width) + x) as usize;
                let byte = memory.memory[offset];
                let r = (byte & 7) * 36; // just hax. low 3 bits
                let g = ((byte >> 3) & 7) * 36; // mid 3 bits
                let b = ((byte >> 5) & 3) * 36; // hi 2 bits
                canvas.put_pixel(x, y, Rgba([r, g, b, 255]));
            }
        }

        texture
            .update(&mut self.window.encoder, &canvas)
            .unwrap();

        // HACK to redraw window without locking up
        for _ in 0..3 {
            match self.window.next() {
                Some(e) => {
                    self.window
                        .draw_2d(&e, |c, g| {
                            clear([1.0; 4], g);
                            image(&texture, c.transform, g);
                        });
                }
                None => {}
            }
        }
    }
}
