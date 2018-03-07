use hardware::Hardware;
use cpu::{CPU, R};
use memory::{MMU, MemoryAddress};
use gpu::{VideoModeBlock, GFXMode, SpecialMode, ega_mode_block, vga_mode_block};
use gpu::GFXMode::*;
use bios::BIOS;

// video related interrupts
pub fn handle(cpu: &mut CPU, hw: &mut Hardware) {
    match cpu.get_r8(&R::AH) {
        0x00 => {
            // VIDEO - SET VIDEO MODE
            let al = cpu.get_r8(&R::AL);
            hw.gpu.set_mode(&mut hw.mmu, &mut hw.bios, al);
        }
        0x01 => {
            // VIDEO - SET TEXT-MODE CURSOR SHAPE
            //
            // CH = cursor start and options (see #00013)
            // CL = bottom scan line containing cursor (bits 0-4)

            // Return:
            // Nothing
            println!("XXX set text-mode cursor shape, start_options={:02X}, bottom_line={:02X}",
                     cpu.get_r8(&R::CH),
                     cpu.get_r8(&R::CL));

        }
        0x02 => {
            // VIDEO - SET CURSOR POSITION
            let page = cpu.get_r8(&R::BH);
            let row = cpu.get_r8(&R::DH);
            let column = cpu.get_r8(&R::DL);
            hw.gpu.set_cursor_pos(&mut hw.mmu, row, column, page);
        }
        0x05 => {
            let al = cpu.get_r8(&R::AL);
            /*
            if (al & 0x80 != 0) && hw.gpu.card.is_tandy() {
                let crtcpu = hw.mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_CRTCPU_PAGE);
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
                if hw.gpu.card.is_pc_jr() {
                    // always return graphics mapping, even for invalid values of AL
                    reg_bh = crtcpu & 7;
                    reg_bl = (crtcpu >> 3) & 0x7;
                }
                IO_WriteB(0x3DF, crtcpu);
                hw.mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_CRTCPU_PAGE, crtcpu);
            } else {
            */
                hw.gpu.set_active_page(&mut hw.mmu, al);
            //}
        }
        0x09 => {
            // VIDEO - WRITE CHARACTER AND ATTRIBUTE AT CURSOR POSITION
            let chr = cpu.get_r8(&R::AL);
            let page = cpu.get_r8(&R::BH);
            let mut attrib = cpu.get_r8(&R::BL);
            let count = cpu.get_r16(&R::CX);
            if hw.mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_CURRENT_MODE) == 0x11 {
                attrib = (attrib & 0x80) | 0x3F;
            }
            hw.gpu.write_char(&mut hw.mmu, chr as u16, attrib, page, count, true);
        }
        0x0A => {
            // VIDEO - WRITE CHARACTER ONLY AT CURSOR POSITION
            let chr = cpu.get_r8(&R::AL);
            let page = cpu.get_r8(&R::BH);
            let attrib = cpu.get_r8(&R::BL);
            let count = cpu.get_r16(&R::CX);
            hw.gpu.write_char(&mut hw.mmu, chr as u16, attrib, page, count, false);
        }
        0x0B => {
            match cpu.get_r8(&R::BH) {
                0x00 => {
                    // VIDEO - SET BACKGROUND/BORDER COLOR
                    // BL = background/border color (border only in text modes)
                    // Return: Nothing
                    println!("XXX set bg/border color, bl={:02X}", cpu.get_r8(&R::BL));
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
                    println!("XXX TODO set palette id, bl={:02X}", cpu.get_r8(&R::BL));
                }
                _ => {
                    println!("video error: unknown int 10, ah=0B, bh={:02X}", cpu.get_r8(&R::BH));
                }
            }
        }
        0x0C => {
            // VIDEO - WRITE GRAPHICS PIXEL
            let page = cpu.get_r8(&R::BH);
            let color = cpu.get_r8(&R::AL);
            let col = cpu.get_r16(&R::CX);
            let row = cpu.get_r16(&R::DX);
            hw.gpu.write_pixel(&mut hw.mmu, col, row, page, color);
        }
        0x0E => {
            // VIDEO - TELETYPE OUTPUT
            let chr = cpu.get_r8(&R::AL);
            let page = cpu.get_r8(&R::BH);
            let color = cpu.get_r8(&R::BL);
            hw.gpu.teletype_output(&mut hw.mmu, chr, page, color);
        }
        0x0F => {
            // VIDEO - GET CURRENT VIDEO MODE
            //
            // Return:
            // AH = number of character columns
            // AL = display mode (see AH=00h)
            // BH = active page (see AH=05h)
            //
            // more info: http://www.ctyme.com/intr/rb-0108.htm
            println!("XXX int10,0F - get video mode impl");
        }
        0x10 => {
            match cpu.get_r8(&R::AL) {
                0x00 => {
                    // VIDEO - SET SINGLE PALETTE REGISTER (PCjr,Tandy,EGA,MCGA,VGA)
                    // BL = palette register number (00h-0Fh)
                    //    = attribute register number (undocumented) (see #00017)
                    // BH = color or attribute register value
                    panic!("XXX VIDEO - SET SINGLE PALETTE REGISTER, bl={:02X}, bh={:02X}",
                             cpu.get_r8(&R::BL),
                             cpu.get_r8(&R::BH));
                }
                0x07 => {
                    // VIDEO - GET INDIVIDUAL PALETTE REGISTER (VGA,UltraVision v2+)
                    let reg = cpu.get_r8(&R::BL);
                    cpu.set_r8(&R::BH, hw.gpu.get_individual_palette_register(reg));
                }
                0x10 => {
                    // VIDEO - SET INDIVIDUAL DAC REGISTER (VGA/MCGA)
                    let index = cpu.get_r8(&R::BL);
                    let r = cpu.get_r8(&R::DH);
                    let g = cpu.get_r8(&R::CH);
                    let b = cpu.get_r8(&R::CL);
                    hw.gpu.set_individual_dac_register(&mut hw.mmu, index, r, g, b);
                }
                0x12 => {
                    // VIDEO - SET BLOCK OF DAC REGISTERS (VGA/MCGA)
                    let start = cpu.get_r16(&R::BX);
                    let count = cpu.get_r16(&R::CX);
                    let seg = cpu.get_r16(&R::ES);
                    let off = cpu.get_r16(&R::DX);
                    hw.gpu.set_dac_block(&mut hw.mmu, start, count, seg, off);
                }
                0x15 => {
                    // VIDEO - READ INDIVIDUAL DAC REGISTER (VGA/MCGA)
                    let reg = cpu.get_r8(&R::BL);
                    let (r, g, b) = hw.gpu.get_individual_dac_register(reg);
                    cpu.set_r8(&R::DH, r);
                    cpu.set_r8(&R::CH, g);
                    cpu.set_r8(&R::CL, b);
                }
                0x17 => {
                    // VIDEO - READ BLOCK OF DAC REGISTERS (VGA/MCGA)
                    let index = cpu.get_r16(&R::BX);
                    let count = cpu.get_r16(&R::CX);
                    let seg = cpu.get_r16(&R::ES);
                    let off = cpu.get_r16(&R::DX);
                    hw.gpu.read_dac_block(&mut hw.mmu, index, count, seg, off);
                }
                _ => {
                    println!("int10 error: unknown AH 10, al={:02X}", cpu.get_r8(&R::AL));
                }
            }
        }
        0x11 => {
            match cpu.get_r8(&R::AL) {
                0x24 => {
                    // VIDEO - GRAPH-MODE CHARGEN - LOAD 8x16 GRAPHICS CHARS (VGA,MCGA)
                    let bl = cpu.get_r8(&R::BL);
                    let dl = cpu.get_r8(&R::DL);
                    hw.gpu.load_graphics_chars(&mut hw.mmu, bl, dl);
                }
                0x30 => {
                    // VIDEO - GET FONT INFORMATION (EGA, MCGA, VGA)
                    // return:
                    // ES:BP = specified pointer
                    // CX    = bytes/character of on-screen font (not the requested font!)
                    // DL    = highest character row on screen
                    let bh = cpu.get_r8(&R::BH);
                    match bh { // BH = pointer specifier
                        0x00 => { // INT 1Fh pointer
                            let (seg, off) = hw.mmu.read_vec(0x1F);
                            cpu.set_r16(&R::ES, seg);
                            cpu.set_r16(&R::BP, off);
                        }
                        // 01h INT 43h pointer
                        0x02 => {
                            // ROM 8x14 character font pointer
                            if let MemoryAddress::RealSegmentOffset(seg, off) = hw.gpu.font_14 {
                                cpu.set_r16(&R::ES, seg);
                                cpu.set_r16(&R::BP, off);
                            }
                        }
                        // 03h ROM 8x8 double dot font pointer
                        // 04h ROM 8x8 double dot font (high 128 characters)
                        // 05h ROM alpha alternate (9 by 14) pointer (EGA,VGA)
                        0x06 => {
                            // ROM 8x16 font (MCGA, VGA)
                            if hw.gpu.card.is_vga() {
                                if let MemoryAddress::RealSegmentOffset(seg, off) = hw.gpu.font_16 {
                                    cpu.set_r16(&R::ES, seg);
                                    cpu.set_r16(&R::BP, off);
                                }
                            }
                        }
                        // 07h ROM alternate 9x16 font (VGA only) (see #00021)
                        // 11h (UltraVision v2+) 8x20 font (VGA) or 8x19 font (autosync EGA)
                        // 12h (UltraVision v2+) 8x10 font (VGA) or 8x11 font (autosync EGA)
                        _ => panic!("VIDEO - GET FONT INFORMATION (EGA, MCGA, VGA): unhandled bh={:02X}", bh),
                    }
                }
                _ => {
                    println!("int10 error: unknown ah=11, al={:02X}", cpu.get_r8(&R::AL));
                }
            }
        }
        0x13 => {
            // VIDEO - WRITE STRING (AT and later,EGA)
            let row = cpu.get_r8(&R::DH);
            let col = cpu.get_r8(&R::DL);
            let flag = cpu.get_r8(&R::AL);
            let attr = cpu.get_r8(&R::BL);
            let str_seg = cpu.get_r16(&R::ES);
            let str_offs = cpu.get_r16(&R::BP);
            let count = cpu.get_r16(&R::CX);
            let page = cpu.get_r8(&R::BH);
            hw.gpu.write_string(&mut hw.mmu, row, col, flag, attr, str_seg, str_offs, count, page);
        }
        0x4F => {
            // VESA
            match cpu.get_r8(&R::AL) {
                0x01 => {
                    // VESA SuperVGA BIOS - GET SuperVGA MODE INFORMATION
                    // CX = SuperVGA video mode (see #04082 for bitfields)
                    // ES:DI -> 256-byte buffer for mode information (see #00079)
                    // Return:
                    // AL = 4Fh if function supported
                    // AH = status:
                    //      00h successful, ES:DI buffer filled
                    //      01h failed
                    println!("XXX VESA SuperVGA BIOS - GET SuperVGA MODE INFORMATION. cx={:04X}", cpu.get_r16(&R::CX));
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
                    println!("XXX VESA SuperVGA BIOS - SET SuperVGA VIDEO MODE. bx={:04X}", cpu.get_r16(&R::BX));
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
                    println!("XXX VESA SuperVGA BIOS - CPU VIDEO MEMORY CONTROL. bh={:02X}", cpu.get_r8(&R::BH));
                }
                 _ => {
                    println!("int10 error: unknown AH 4F (VESA), al={:02X}", cpu.get_r8(&R::AL));
                }
            }
        }
        _ => {
            println!("int10 error: unknown al={:02X}, ax={:04X}",
                     cpu.get_r8(&R::AL),
                     cpu.get_r16(&R::AX));
        }
    }
}

