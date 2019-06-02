use crate::cpu::{CPU, R};
use crate::machine::Machine;
use crate::memory::{MMU, MemoryAddress};
use crate::gpu::{VideoModeBlock, GFXMode, SpecialMode, ega_mode_block, vga_mode_block};
use crate::gpu::GFXMode::*;
use crate::bios::BIOS;

// video related interrupts
pub fn handle(machine: &mut Machine) {
    match machine.cpu.get_r8(R::AH) {
        0x00 => {
            // VIDEO - SET VIDEO MODE
            let al = machine.cpu.get_r8(R::AL);
            machine.gpu.set_mode(&mut machine.mmu, al);
        }
        0x01 => {
            // VIDEO - SET TEXT-MODE CURSOR SHAPE
            //
            // CH = cursor start and options (see #00013)
            // CL = bottom scan line containing cursor (bits 0-4)

            // Return:
            // Nothing
            println!("XXX set text-mode cursor shape, start_options={:02X}, bottom_line={:02X}",
                     machine.cpu.get_r8(R::CH),
                     machine.cpu.get_r8(R::CL));

        }
        0x02 => {
            // VIDEO - SET CURSOR POSITION
            let page = machine.cpu.get_r8(R::BH);
            let row = machine.cpu.get_r8(R::DH);
            let column = machine.cpu.get_r8(R::DL);
            machine.gpu.set_cursor_pos(&mut machine.mmu, row, column, page);
        }
        0x03 => {
            // VIDEO - GET CURSOR POSITION AND SIZE
            let page = machine.cpu.get_r8(R::BH);
            // Return:
            // AX = 0000h (Phoenix BIOS)
            // CH = start scan line
            // CL = end scan line
            // DH = row (00h is top)
            // DL = column (00h is left)
            println!("XXX GET CURSOR POSITION AND SIZE, page {}", page);
        }
        0x05 => {
            // VIDEO - SELECT ACTIVE DISPLAY PAGE
            // AL = new page number (0 to number of pages - 1)
            let al = machine.cpu.get_r8(R::AL);
            /*
            if (al & 0x80 != 0) && machine.gpu.card.is_tandy() {
                let crtcpu = machine.mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_CRTCPU_PAGE);
                match al {
                    0x80 => {
                        reg_bh = crtcpu & 7;
                        reg_bl = (crtcpu >> 3) & 0x7;
                    }
                    0x81 => {
                        crtcpu = (crtcpu & 0xc7) | ((reg_bl & 7) << 3);
                    }
                    0x82 => {
                        crtcpu = (crtcpu & 0xf8) | (reg_bh & 7);
                    }
                    0x83 => {
                        crtcpu = (crtcpu & 0xc0) | (reg_bh & 7) | ((reg_bl & 7) << 3);
                    }
                }
                if machine.gpu.card.is_pc_jr() {
                    // always return graphics mapping, even for invalid values of AL
                    reg_bh = crtcpu & 7;
                    reg_bl = (crtcpu >> 3) & 0x7;
                }
                IO_WriteB(0x3DF, crtcpu);
                machine.mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_CRTCPU_PAGE, crtcpu);
            } else {
            */
                machine.gpu.set_active_page(&mut machine.mmu, al);
            //}
        }
        0x06 => {
            // VIDEO - SCROLL UP WINDOW
            // AL = number of lines by which to scroll up (00h = clear entire window)
            // BH = attribute used to write blank lines at bottom of window
            // CH,CL = row,column of window's upper left corner
            // DH,DL = row,column of window's lower right corner
            let lines = machine.cpu.get_r8(R::AL);
            let attr = machine.cpu.get_r8(R::BH);
            let x1 = machine.cpu.get_r8(R::CL);
            let y1 = machine.cpu.get_r8(R::CH);
            let x2 = machine.cpu.get_r8(R::DL);
            let y2 = machine.cpu.get_r8(R::DH);
            println!("XXX int10 - SCROLL UP WINDOW, lines {}, attr {}, upper left {},{}, lower right {},{}", lines, attr, x1, y1, x2, y2);
        }
        0x08 => {
            // VIDEO - READ CHARACTER AND ATTRIBUTE AT CURSOR POSITION
            let page = machine.cpu.get_r8(R::BH);
            // Return:
            // AH = character's attribute (text mode only) (see #00014)
            // AH = character's color (Tandy 2000 graphics mode only)
            // AL = character
            println!("XXX int10 - READ CHARACTER AND ATTRIBUTE AT CURSOR POSITION, page {}", page);
        }
        0x09 => {
            // VIDEO - WRITE CHARACTER AND ATTRIBUTE AT CURSOR POSITION
            let chr = machine.cpu.get_r8(R::AL);
            let page = machine.cpu.get_r8(R::BH);
            let mut attrib = machine.cpu.get_r8(R::BL);
            let count = machine.cpu.get_r16(R::CX);
            if machine.mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_CURRENT_MODE) == 0x11 {
                attrib = (attrib & 0x80) | 0x3F;
            }
            machine.gpu.write_char(&mut machine.mmu, u16::from(chr), attrib, page, count, true);
        }
        0x0A => {
            // VIDEO - WRITE CHARACTER ONLY AT CURSOR POSITION
            let chr = machine.cpu.get_r8(R::AL);
            let page = machine.cpu.get_r8(R::BH);
            let attrib = machine.cpu.get_r8(R::BL);
            let count = machine.cpu.get_r16(R::CX);
            machine.gpu.write_char(&mut machine.mmu, u16::from(chr), attrib, page, count, false);
        }
        0x0B => {
            match machine.cpu.get_r8(R::BH) {
                0x00 => {
                    // VIDEO - SET BACKGROUND/BORDER COLOR
                    // BL = background/border color (border only in text modes)
                    // Return: Nothing
                    println!("XXX set bg/border color, bl={:02X}", machine.cpu.get_r8(R::BL));
                }
                0x01 => {
                    // VIDEO - SET PALETTE
                    // BL = palette ID
                    //    00h background, green, red, and brown/yellow
                    //    01h background, cyan, magenta, and white
                    // Return: Nothing
                    //
                    // Note: This call was only valid in 320x200 graphics on
                    // the CGA, but newer cards support it in many or all
                    // graphics modes
                    println!("XXX TODO set palette id, bl={:02X}", machine.cpu.get_r8(R::BL));
                }
                _ => {
                    println!("video error: unknown int 10, ah=0B, bh={:02X}", machine.cpu.get_r8(R::BH));
                }
            }
        }
        0x0C => {
            // VIDEO - WRITE GRAPHICS PIXEL
            let page = machine.cpu.get_r8(R::BH);
            let color = machine.cpu.get_r8(R::AL);
            let col = machine.cpu.get_r16(R::CX);
            let row = machine.cpu.get_r16(R::DX);
            machine.gpu.write_pixel(&mut machine.mmu, col, row, page, color);
        }
        0x0E => {
            // VIDEO - TELETYPE OUTPUT
            let chr = machine.cpu.get_r8(R::AL);
            let page = machine.cpu.get_r8(R::BH);
            let color = machine.cpu.get_r8(R::BL);
            machine.gpu.teletype_output(&mut machine.mmu, chr, page, color);
        }
        0x0F => {
            // VIDEO - GET CURRENT VIDEO MODE
            machine.cpu.set_r8(R::AH, machine.gpu.mode.twidth as u8);               // number of character columns
            machine.cpu.set_r8(R::AL, machine.gpu.mode.mode as u8);                 // display mode
            machine.cpu.set_r8(R::BH, machine.gpu.get_active_page(&mut machine.mmu));    // active page
        }
        0x10 => {
            match machine.cpu.get_r8(R::AL) {
                0x00 => {
                    // VIDEO - SET SINGLE PALETTE REGISTER (PCjr,Tandy,EGA,MCGA,VGA)
                    // BL = palette register number (00h-0Fh)
                    //    = attribute register number (undocumented) (see #00017)
                    // BH = color or attribute register value
                    panic!("XXX VIDEO - SET SINGLE PALETTE REGISTER, bl={:02X}, bh={:02X}",
                             machine.cpu.get_r8(R::BL),
                             machine.cpu.get_r8(R::BH));
                }
                0x07 => {
                    // VIDEO - GET INDIVIDUAL PALETTE REGISTER (VGA,UltraVision v2+)
                    let reg = machine.cpu.get_r8(R::BL);
                    machine.cpu.set_r8(R::BH, machine.gpu.get_individual_palette_register(reg));
                }
                0x10 => {
                    // VIDEO - SET INDIVIDUAL DAC REGISTER (VGA/MCGA)
                    let index = machine.cpu.get_r8(R::BL);
                    let r = machine.cpu.get_r8(R::DH);
                    let g = machine.cpu.get_r8(R::CH);
                    let b = machine.cpu.get_r8(R::CL);
                    machine.gpu.set_individual_dac_register(&mut machine.mmu, index, r, g, b);
                }
                0x12 => {
                    // VIDEO - SET BLOCK OF DAC REGISTERS (VGA/MCGA)
                    let start = machine.cpu.get_r16(R::BX);
                    let count = machine.cpu.get_r16(R::CX);
                    let seg = machine.cpu.get_r16(R::ES);
                    let off = machine.cpu.get_r16(R::DX);
                    machine.gpu.set_dac_block(&mut machine.mmu, start, count, seg, off);
                }
                0x15 => {
                    // VIDEO - READ INDIVIDUAL DAC REGISTER (VGA/MCGA)
                    let reg = machine.cpu.get_r8(R::BL);
                    let (r, g, b) = machine.gpu.get_individual_dac_register(reg);
                    machine.cpu.set_r8(R::DH, r);
                    machine.cpu.set_r8(R::CH, g);
                    machine.cpu.set_r8(R::CL, b);
                }
                0x17 => {
                    // VIDEO - READ BLOCK OF DAC REGISTERS (VGA/MCGA)
                    let index = machine.cpu.get_r16(R::BX);
                    let count = machine.cpu.get_r16(R::CX);
                    let seg = machine.cpu.get_r16(R::ES);
                    let off = machine.cpu.get_r16(R::DX);
                    machine.gpu.read_dac_block(&mut machine.mmu, index, count, seg, off);
                }
                _ => {
                    println!("int10 error: unknown AH 10, al={:02X}", machine.cpu.get_r8(R::AL));
                }
            }
        }
        0x11 => {
            match machine.cpu.get_r8(R::AL) {
                0x24 => {
                    // VIDEO - GRAPH-MODE CHARGEN - LOAD 8x16 GRAPHICS CHARS (VGA,MCGA)
                    let bl = machine.cpu.get_r8(R::BL);
                    let dl = machine.cpu.get_r8(R::DL);
                    machine.gpu.load_graphics_chars(&mut machine.mmu, bl, dl);
                }
                0x30 => {
                    // VIDEO - GET FONT INFORMATION (EGA, MCGA, VGA)
                    // return:
                    // ES:BP = specified pointer
                    // CX    = bytes/character of on-screen font (not the requested font!)
                    // DL    = highest character row on screen
                    let bh = machine.cpu.get_r8(R::BH);
                    match bh { // BH = pointer specifier
                        0x00 => { // INT 1Fh pointer
                            let (seg, off) = machine.mmu.read_vec(0x1F);
                            machine.cpu.set_r16(R::ES, seg);
                            machine.cpu.set_r16(R::BP, off);
                        }
                        // 01h INT 43h pointer
                        0x02 => {
                            // ROM 8x14 character font pointer
                            if let MemoryAddress::RealSegmentOffset(seg, off) = machine.gpu.font_14 {
                                machine.cpu.set_r16(R::ES, seg);
                                machine.cpu.set_r16(R::BP, off);
                            }
                        }
                        // 03h ROM 8x8 double dot font pointer
                        // 04h ROM 8x8 double dot font (high 128 characters)
                        // 05h ROM alpha alternate (9 by 14) pointer (EGA,VGA)
                        0x06 => {
                            // ROM 8x16 font (MCGA, VGA)
                            if machine.gpu.card.is_vga() {
                                if let MemoryAddress::RealSegmentOffset(seg, off) = machine.gpu.font_16 {
                                    machine.cpu.set_r16(R::ES, seg);
                                    machine.cpu.set_r16(R::BP, off);
                                }
                            }
                        }
                        // 07h ROM alternate 9x16 font (VGA only) (see #00021)
                        // 11h (UltraVision v2+) 8x20 font (VGA) or 8x19 font (autosync EGA)
                        // 12h (UltraVision v2+) 8x10 font (VGA) or 8x11 font (autosync EGA)
                        _ => panic!("VIDEO - GET FONT INFORMATION (EGA, MCGA, VGA): unhandled bh={:02X}", bh),
                    }
                }
                _ => println!("int10 error: unknown ah=11, al={:02X}", machine.cpu.get_r8(R::AL)),
            }
        }
        0x12 => {
            match machine.cpu.get_r8(R::BL) {
                0x10 => {
                    // VIDEO - ALTERNATE FUNCTION SELECT (PS, EGA, VGA, MCGA) - GET EGA INFO

                    // Return:
                    // BH = video state
                    //      00h color mode in effect (I/O port 3Dxh)
                    //      01h mono mode in effect (I/O port 3Bxh)
                    // BL = installed memory (00h = 64K, 01h = 128K, 02h = 192K, 03h = 256K)
                    // CH = feature connector bits (see #00022)
                    // CL = switch settings (see #00023,#00024)

                    // use return values as seen on win xp
                    machine.cpu.set_r8(R::BH, 0); // color mode in effect (I/O port 3Dxh)
                    machine.cpu.set_r8(R::BL, 3); // 256k
                    machine.cpu.set_r8(R::CH, 0);
                    machine.cpu.set_r8(R::CL, 9);
                }
                _ => println!("int10 error: unknown ah=12, bl={:02X}", machine.cpu.get_r8(R::BL)),
            }
        }
        0x13 => {
            // VIDEO - WRITE STRING (AT and later,EGA)
            let row = machine.cpu.get_r8(R::DH);
            let col = machine.cpu.get_r8(R::DL);
            let flag = machine.cpu.get_r8(R::AL);
            let attr = machine.cpu.get_r8(R::BL);
            let str_seg = machine.cpu.get_r16(R::ES);
            let str_offs = machine.cpu.get_r16(R::BP);
            let count = machine.cpu.get_r16(R::CX);
            let page = machine.cpu.get_r8(R::BH);
            machine.gpu.write_string(&mut machine.mmu, row, col, flag, attr, str_seg, str_offs, count, page);
        }
        0x1A => {
             match machine.cpu.get_r8(R::AL) {
                0x00 => {
                    // VIDEO - GET DISPLAY COMBINATION CODE (PS,VGA/MCGA)
                    // Return:
                    // AL = 1Ah if function was supported
                    // BL = active display code (see #00039)
                    // BH = alternate display code (see #00039)
                    machine.cpu.set_r8(R::AL, 0x1A);
                    machine.cpu.set_r8(R::BL, 0x08); // 08 = VGA w/ color analog display
                    machine.cpu.set_r8(R::BH, 0x00); // 00 = no display
                }
                 _ => {
                    println!("int10 error: unknown ah=1a, al={:02X}", machine.cpu.get_r8(R::AL));
                }
            }
        }
        0x4F => {
            // VESA
            match machine.cpu.get_r8(R::AL) {
                0x01 => {
                    // VESA SuperVGA BIOS - GET SuperVGA MODE INFORMATION
                    // CX = SuperVGA video mode (see #04082 for bitfields)
                    // ES:DI -> 256-byte buffer for mode information (see #00079)
                    // Return:
                    // AL = 4Fh if function supported
                    // AH = status:
                    //      00h successful, ES:DI buffer filled
                    //      01h failed
                    println!("XXX VESA SuperVGA BIOS - GET SuperVGA MODE INFORMATION. cx={:04X}", machine.cpu.get_r16(R::CX));
                }
                0x02 => {
                    // VESA SuperVGA BIOS - SET SuperVGA VIDEO MODE
                    // BX = new video mode (see #04082,#00083,#00084)
                    // ES:DI -> (VBE 3.0+) CRTC information block, bit mode bit 11 set
                    // Return:
                    // AL = 4Fh if function supported
                    // AH = status
                    //      00h successful
                    //      01h failed
                    println!("XXX VESA SuperVGA BIOS - SET SuperVGA VIDEO MODE. bx={:04X}", machine.cpu.get_r16(R::BX));
                }
                0x05 => {
                    // VESA SuperVGA BIOS - CPU VIDEO MEMORY CONTROL
                    // BH = subfunction
                    // 00h select video memory window
                    // DX = window address in video memory (in granularity units)
                    // 01h get video memory window
                    // Return:
                    // DX = window address in video memory (in gran. units).
                    // BL = window number
                    //      00h window A
                    //      01h window B.
                    // ES = selector for memory-mapped registers (VBE 2.0+, when called from 32-bit protected mode)
                    println!("XXX VESA SuperVGA BIOS - CPU VIDEO MEMORY CONTROL. bh={:02X}", machine.cpu.get_r8(R::BH));
                }
                 _ => {
                    println!("int10 error: unknown AH 4F (VESA), al={:02X}", machine.cpu.get_r8(R::AL));
                }
            }
        }
        _ => {
            println!("int10 (video) error: unknown ah={:02X}, ax={:04X}, bx={:04X}",
                     machine.cpu.get_r8(R::AH),
                     machine.cpu.get_r16(R::AX),
                     machine.cpu.get_r16(R::BX));
        }
    }
}

