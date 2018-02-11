use std::cell::RefCell;
use std::num::Wrapping;

use gpu::palette::{DACPalette, default_vga_palette};

#[cfg(test)]
#[path = "./gpu_test.rs"]
mod gpu_test;

#[derive(Clone, Default)]
pub struct GPU {
    pub scanline: u32,
    pub width: u32,
    pub height: u32,
    pub pal: Vec<DACPalette>,
    pub pel_address: u8,            // set by write to 03c8
    pel_component: usize,           // color component for next out 03c9, 0 = red, 1 = green, 2 = blue
    mode: u8,
}

impl GPU {
    pub fn new() -> Self {
        GPU {
            scanline: 0,
            width: 80,
            height: 25,
            pal: default_vga_palette(),
            pel_address: 0,
            pel_component: 0,
            mode: 0x03, // default mode is 80x25 text
        }
    }

    pub fn render_frame(&self, memory: &[u8]) -> Vec<u8> {
        match self.mode {
            // 00: 40x25 Black and White text (CGA,EGA,MCGA,VGA)
            // 01: 40x25 16 color text (CGA,EGA,MCGA,VGA)
            // 02: 80x25 16 shades of gray text (CGA,EGA,MCGA,VGA)
            //0x03 => self.render_mode03_frame(memory), // 80x25 16 color text (CGA,EGA,MCGA,VGA)
            0x04 => self.render_mode04_frame(memory), // 320x200 4 color graphics (CGA,EGA,MCGA,VGA)
            // 05: 320x200 4 color graphics (CGA,EGA,MCGA,VGA)
            //0x06 => self.render_mode06_frame(memory), // 640x200 B/W graphics (CGA,EGA,MCGA,VGA)
            // 07: 80x25 Monochrome text (MDA,HERC,EGA,VGA)
            // 08: 160x200 16 color graphics (PCjr)
            // 09: 320x200 16 color graphics (PCjr)
            // 0A: 640x200 4 color graphics (PCjr)
            // 0D: 320x200 16 color graphics (EGA,VGA)
            // 0E: 640x200 16 color graphics (EGA,VGA)
            // 0F: 640x350 Monochrome graphics (EGA,VGA)
            // 10: 640x350 16 color graphics (EGA or VGA with 128K)
            //     640x350 4 color graphics (64K EGA)
            //0x11 => self.render_mode11_frame(memory), // 640x480 B/W graphics (MCGA,VGA)
            //0x12 => self.render_mode12_frame(memory), // 640x480 16 color graphics (VGA)
            0x13 => self.render_mode13_frame(memory), // 320x200 256 color graphics (MCGA,VGA)
            _ => {
                println!("XXX fixme render_frame for mode {:02x}", self.mode);
                Vec::new()
            }
        }
    }
/*
    fn render_mode03_frame(&self, memory: &[u8]) -> Vec<u8> {
        // 03h = T  80x25  8x8   640x200   16       4   B800 CGA,PCjr,Tandy
        //     = T  80x25  8x14  640x350   16/64    8   B800 EGA
        //     = T  80x25  8x16  640x400   16       8   B800 MCGA
        //     = T  80x25  9x16  720x400   16       8   B800 VGA
        //     = T  80x43  8x8   640x350   16       4   B800 EGA,VGA [17]
        //     = T  80x50  8x8   640x400   16       4   B800 VGA [17]
        // XXX impl
        Vec::new()
    }
*/
    fn render_mode04_frame(&self, memory: &[u8]) -> Vec<u8> {
        // XXX palette selection is done by writes to cga registers
        // mappings to the cga palette
        let pal1_map: [usize; 4] = [0, 3, 5, 7];
        // let pal1_map: [u8; 3] = [11, 13, 15];
        // let pal0_map: [u8; 4] = [0, 2, 4, 6];
        // let pal0_map: [u8; 4] = [0, 10, 12, 14];

        // 04h = G  40x25  8x8   320x200    4       .   B800 CGA,PCjr,EGA,MCGA,VGA
        let mut buf = vec![0u8; (self.width * self.height * 3) as usize];
        println!("cga draw {}x{}", self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                // divide Y by 2
                // divide X by 4 (2 bits for each pixel)
                // 80 bytes per line (80 * 4 = 320), 4 pixels per byte
                let offset = (0xB_8000 + ((y%2) * 0x2000) + (80 * (y >> 1)) + (x >> 2)) as usize;
                let bits = (memory[offset] >> ((3 - (x & 3)) * 2)) & 3; // 2 bits: cga palette to use
                let pal = &self.pal[pal1_map[bits as usize]];

                let dst = (((y * self.width) + x) * 3) as usize;
                buf[dst] = pal.r;
                buf[dst+1] = pal.g;
                buf[dst+2] = pal.b;
            }
        }
        buf
    }
/*
    fn render_mode06_frame(&self, memory: &[u8]) -> Vec<u8> {
        // 06h = G  80x25  8x8   640x200    2       .   B800 CGA,PCjr,EGA,MCGA,VGA
        //     = G  80x25   .       .     mono      .   B000 HERCULES.COM on HGC [14]
        // XXX impl
        Vec::new()
    }

    fn render_mode11_frame(&self, memory: &[u8]) -> Vec<u8> {
        // 11h = G  80x30  8x16  640x480  mono      .   A000 VGA,MCGA,ATI EGA,ATI VIP
        // XXX impl
        Vec::new()
    }

    fn render_mode12_frame(&self, memory: &[u8]) -> Vec<u8> {
        // 12h = G  80x30  8x16  640x480   16/256K  .   A000 VGA,ATI VIP
        //     = G  80x30  8x16  640x480   16/64    .   A000 ATI EGA Wonder
        //     = G    .     .    640x480   16       .     .  UltraVision+256K EGA
        // XXX impl, planar mode
        Vec::new()
    }
*/
    fn render_mode13_frame(&self, memory: &[u8]) -> Vec<u8> {
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
        buf
    }

    pub fn set_mode(&mut self, mode: u8) {
        self.mode = mode;
        // more info and video modes: http://www.ctyme.com/intr/rb-0069.htm
        match mode {
            0x01 => {
                // 01h = T  40x25  8x8   320x200   16       8   B800 CGA,PCjr,Tandy
                //     = T  40x25  8x14  320x350   16       8   B800 EGA
                //     = T  40x25  8x16  320x400   16       8   B800 MCGA
                //     = T  40x25  9x16  360x400   16       8   B800 VGA
                println!("XXX video: set video mode to 320x200, 16 colors (text)");
            }
            0x03 => {
                self.width = 640;
                self.height = 200;
            }
            0x04 => {
                self.width = 320;
                self.height = 200;
            }
            0x06 => {
                self.width = 640;
                self.height = 200;
            }
            0x11 => {
                self.width = 640;
                self.height = 480;
            }
            0x12 => {
                self.width = 640;
                self.height = 480;
            }
            0x13 => {
                self.width = 320;
                self.height = 200;
            }
            _ => {
                println!("video error: unknown video mode {:02X}", mode);
            }
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
        // XXX unsure how to handle color index > 0xFF. wrap or ignore?
        self.pal[index & 0xFF].r = val;
    }

    // sets the 0-255 intensity of the green color channel
    pub fn set_palette_g(&mut self, index: usize, val: u8) {
        self.pal[index & 0xFF].g = val;
    }

    // sets the 0-255 intensity of the blue color channel
    pub fn set_palette_b(&mut self, index: usize, val: u8) {
        self.pal[index & 0xFF].b = val;
    }

    // CGA status register (0x03DA)
    // color EGA/VGA: input status 1 register
    pub fn read_cga_status_register(&self) -> u8 {
        // Bitfields for CGA status register:
        // Bit(s)	Description	(Table P0818)
        // 7-6	not used
        // 7	(C&T Wingine) vertical sync in progress (if enabled by XR14)
        // 5-4	color EGA, color ET4000, C&T: diagnose video display feedback, select
        //      from color plane enable
        // 3	in vertical retrace
        //      (C&T Wingine) video active (retrace/video selected by XR14)
        // 2	(CGA,color EGA) light pen switch is off
        //      (MCGA,color ET4000) reserved (0)
        //      (VGA) reserved (1)
        // 1	(CGA,color EGA) positive edge from light pen has set trigger
        //      (VGA,MCGA,color ET4000) reserved (0)
        // 0	horizontal retrace in progress
        //    =0  do not use memory
        //    =1  memory access without interfering with display
        //        (VGA,Genoa SuperEGA) horizontal or vertical retrace
        //    (C&T Wingine) display enabled (retrace/DE selected by XR14)
        let mut flags = 0;

        // FIXME REMOVE THIS HACK: fake bit 0 and 3 (retrace in progress)
        if self.scanline == 0 {
            flags |= 0b0000_0001; // set bit 0
            flags |= 0b0000_1000; // set bit 3
        } else {
            flags &= 0b1111_1110; // clear bit 0
            flags &= 0b1111_0111; // clear bit 3
        }

        // println!("read_cga_status_register: returns {:02X}", flags);

        flags
    }
}
