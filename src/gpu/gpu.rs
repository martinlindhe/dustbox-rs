use std::cell::RefCell;
use std::num::Wrapping;
use std::marker::PhantomData;

use cpu::CPU;
use memory::mmu::MMU;
use gpu::palette::{DACPalette, default_vga_palette};
use gpu::font;
use gpu::video_parameters;

#[cfg(test)]
#[path = "./gpu_test.rs"]
mod gpu_test;

pub static STATIC_FUNCTIONALITY: [u8; 0x10] = [
 /* 0 */ 0xff,  // All modes supported #1
 /* 1 */ 0xff,  // All modes supported #2
 /* 2 */ 0x0f,  // All modes supported #3
 /* 3 */ 0x00, 0x00, 0x00, 0x00,  // reserved
 /* 7 */ 0x07,  // 200, 350, 400 scan lines
 /* 8 */ 0x04,  // total number of character blocks available in text modes
 /* 9 */ 0x02,  // maximum number of active character blocks in text modes
 /* a */ 0xff,  // Misc Flags Everthing supported
 /* b */ 0x0e,  // Support for Display combination, intensity/blinking and video state saving/restoring
 /* c */ 0x00,  // reserved
 /* d */ 0x00,  // reserved
 /* e */ 0x00,  // Change to add new functions
 /* f */ 0x00,  // reserved
];


#[derive(Clone)]
pub struct GPU {
    pub scanline: u32,
    pub width: u32,
    pub height: u32,
    pub pal: Vec<DACPalette>,
    pub pel_address: u8,            // set by write to 03c8
    pel_component: usize,           // color component for next out 03c9, 0 = red, 1 = green, 2 = blue
    mode: u8,
    pub architecture: Architecture,
    font_8_first: u32,
    font_8_second: u32,
    pub font_14: u32,
    font_14_alternate: u32,
    pub font_16: u32,
    font_16_alternate: u32,
    static_config: u32,
    video_parameter_table: u32,
    video_dcc_table: u32,
    video_save_pointer_table: u32,
    video_save_pointers: u32,
}

impl GPU {
    pub fn new() -> Self {
        GPU {
            scanline: 0,
            width: 300,
            height: 200,
            pal: default_vga_palette(),
            pel_address: 0,
            pel_component: 0,
            mode: 0x03, // default mode is 80x25 text
            architecture: Architecture::VGA,
            font_8_first: 0,
            font_8_second: 0,
            font_14: 0,
            font_14_alternate: 0,
            font_16: 0,
            font_16_alternate: 0,
            static_config: 0,
            video_parameter_table: 0,
            video_dcc_table: 0,
            video_save_pointer_table: 0,
            video_save_pointers: 0,
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

    fn setup_video_parameter_table(&mut self, mmu: &mut MMU, segment: u16, offset: u16) -> u16 {
        if self.architecture.is_vga() {
            for (i, b) in video_parameters::TABLE_VGA.iter().enumerate() {
                mmu.write_u8(segment, offset + i as u16, *b);
            }
            return video_parameters::TABLE_VGA.len() as u16;
        }
        for (i, b) in video_parameters::TABLE_EGA.iter().enumerate() {
            mmu.write_u8(segment, offset + i as u16, *b);
        }
        return video_parameters::TABLE_EGA.len() as u16;
    }

    fn video_bios_size(&self) -> u16 {
        // XXX more details in Init_VGABIOS in dosbox-x
        0x8000
    }

    pub fn init(&mut self, mut mmu: &mut MMU) {
        let rom_base = 0xC000;

        let video_bios_size = self.video_bios_size();

        let mut pos = 3;
        if self.architecture.is_ega_vga() {
            // ROM signature
            mmu.write_u16(rom_base, 0, 0xAA55);
            mmu.write_u8(rom_base, 2, (video_bios_size >> 9) as u8);
            /*
            // entry point
            mmu.write_u8(rom_base, 3, 0xFE); // Callback instruction
            mmu.write_u8(rom_base, 4, 0x38);
            mmu.write_u16(rom_base, 5, VGA_ROM_BIOS_ENTRY_cb);
            mmu.write_u8(rom_base, 7, 0xCB); // RETF
            */

            // VGA BIOS copyright
            if self.architecture.is_vga() {
                mmu.write(rom_base, 0x1e, b"IBM compatible VGA BIOS\0");
            } else {
                mmu.write(rom_base, 0x1e, b"IBM compatible EGA BIOS\0");
            }

            pos = 0x100;
        }

        // cga font
        self.font_8_first = MMU::to_long_pair(rom_base, pos);
        for i in 0..(128 * 8) {
            mmu.write_u8(rom_base, pos, font::FONT_08[i]);
            pos += 1;
        }

        if self.architecture.is_ega_vga() {
            // cga second half
            self.font_8_second = MMU::to_long_pair(rom_base, pos);
            for i in 0..(128 * 8) {
                mmu.write_u8(rom_base, pos, font::FONT_08[i + (128 * 8)]);
                pos += 1;
            }
        }

        if self.architecture.is_ega_vga() {
            // ega font
            self.font_14 = MMU::to_long_pair(rom_base, pos);
            for i in 0..(256 * 14) {
                mmu.write_u8(rom_base, pos, font::FONT_14[i]);
                pos += 1;
            }
        }

        if self.architecture.is_vga() {
            // vga font
            self.font_16 = MMU::to_long_pair(rom_base, pos);
            for i in 0..(256 * 16) {
                mmu.write_u8(rom_base, pos, font::FONT_16[i]);
                pos += 1;
            }

            self.static_config = MMU::to_long_pair(rom_base, pos);
            for i in 0..0x10 {
                mmu.write_u8(rom_base, pos, STATIC_FUNCTIONALITY[i]);
                pos += 1;
            }
        }

        mmu.write_vec(0x1F, self.font_8_second);
        self.font_14_alternate = MMU::to_long_pair(rom_base, pos);
        self.font_16_alternate = MMU::to_long_pair(rom_base, pos);

        mmu.write_u8(rom_base, pos, 0x00); // end of table (empty)
        pos += 1;

        if self.architecture.is_ega_vga() {
            self.video_parameter_table = MMU::to_long_pair(rom_base, pos);
            pos += self.setup_video_parameter_table(&mut mmu, rom_base, pos);

            if self.architecture.is_vga() {
                self.video_dcc_table = MMU::to_long_pair(rom_base, pos);
                mmu.write_u8(rom_base, pos, 0x10); pos += 1; // number of entries
                mmu.write_u8(rom_base, pos, 1); pos += 1;    // version number
                mmu.write_u8(rom_base, pos, 8); pos += 1;    // maximum display code
                mmu.write_u8(rom_base, pos, 0); pos += 1;    // reserved

                // display combination codes
                mmu.write_u16(rom_base, pos, 0x0000); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0100); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0200); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0102); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0400); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0104); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0500); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0502); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0600); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0601); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0605); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0800); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0801); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0700); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0702); pos += 2;
                mmu.write_u16(rom_base, pos, 0x0706); pos += 2;

                self.video_save_pointer_table = MMU::to_long_pair(rom_base, pos);
                mmu.write_u16(rom_base, pos, 0x1a); pos += 2; // length of table

                mmu.write_u32(rom_base, pos, self.video_dcc_table as u32); pos += 4;
                mmu.write_u32(rom_base, pos, 0); pos += 4; // alphanumeric charset override
                mmu.write_u32(rom_base, pos, 0); pos += 4; // user palette table
                mmu.write_u32(rom_base, pos, 0); pos += 4;
                mmu.write_u32(rom_base, pos, 0); pos += 4;
                mmu.write_u32(rom_base, pos, 0); pos += 4;
            }

            self.video_save_pointers = MMU::to_long_pair(rom_base, pos);
            mmu.write_u32(rom_base, pos, self.video_parameter_table as u32); pos += 4;
            mmu.write_u32(rom_base, pos,0); pos += 4; // dynamic save area pointer
            mmu.write_u32(rom_base, pos,0); pos += 4; // alphanumeric character set override
            mmu.write_u32(rom_base, pos, 0); pos += 4; // graphics character set override
            mmu.write_u32(rom_base, pos, self.video_save_pointer_table as u32); pos += 4; // will be 0 if not vga
            mmu.write_u32(rom_base, pos, 0); pos += 4;
            mmu.write_u32(rom_base, pos, 0); pos += 4;
        }

        if self.architecture.is_tandy() {
            mmu.write_vec(0x44, self.font_8_first);
        }
    }
}

// Architecture indicates the current gpu mode of operation
#[derive(Clone, PartialEq)]
pub enum Architecture {
    Tandy, CGA, EGA, VGA,
}

impl Architecture {
    pub fn is_ega_vga(&self) -> bool {
        match *self {
            Architecture::EGA | Architecture::VGA => true,
            _ => false,
        }
    }
    pub fn is_tandy(&self) -> bool {
        match *self {
            Architecture::Tandy => true,
            _ => false,
        }
    }
    pub fn is_cga(&self) -> bool {
        match *self {
            Architecture::CGA => true,
            _ => false,
        }
    }
    pub fn is_ega(&self) -> bool {
        match *self {
            Architecture::EGA => true,
            _ => false,
        }
    }
    pub fn is_vga(&self) -> bool {
        match *self {
            Architecture::VGA => true,
            _ => false,
        }
    }
}
