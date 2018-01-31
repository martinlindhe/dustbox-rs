use cpu::CPU;
use cpu::register::{AX, BX, CX, DX, AL, ES};

// video related interrupts
pub fn handle(cpu: &mut CPU) {
    match cpu.r16[AX].hi_u8() {
        0x00 => {
            // VIDEO - SET VIDEO MODE
            //
            // AL = desired video mode
            //
            // Return:
            // AL = video mode flag (Phoenix, AMI BIOS)
            // 20h mode > 7
            // 30h modes 0-5 and 7
            // 3Fh mode 6
            // AL = CRT controller mode byte (Phoenix 386 BIOS v1.10)
            //
            // Desc: Specify the display mode for the currently
            // active display adapter
            //
            // more info and video modes: http://www.ctyme.com/intr/rb-0069.htm
            match cpu.r16[AX].lo_u8() {
                0x01 => {
                    // 01h = T  40x25  8x8   320x200   16       8   B800 CGA,PCjr,Tandy
                    //     = T  40x25  8x14  320x350   16       8   B800 EGA
                    //     = T  40x25  8x16  320x400   16       8   B800 MCGA
                    //     = T  40x25  9x16  360x400   16       8   B800 VGA
                    println!("XXX video: set video mode to 320x200, 16 colors (text)");
                }
                0x03 => {
                    // 03h = T  80x25  8x8   640x200   16       4   B800 CGA,PCjr,Tandy
                    //     = T  80x25  8x14  640x350   16/64    8   B800 EGA
                    //     = T  80x25  8x16  640x400   16       8   B800 MCGA
                    //     = T  80x25  9x16  720x400   16       8   B800 VGA
                    //     = T  80x43  8x8   640x350   16       4   B800 EGA,VGA [17]
                    //     = T  80x50  8x8   640x400   16       4   B800 VGA [17]
                    println!("XXX video: set video mode to 640x200, 16 colors (text)");
                }
                0x04 => {
                    // 04h = G  40x25  8x8   320x200    4       .   B800 CGA,PCjr,EGA,MCGA,VGA
                    println!("XXX video: set video mode to 320x200, 4 colors");
                }
                0x06 => {
                    // 06h = G  80x25  8x8   640x200    2       .   B800 CGA,PCjr,EGA,MCGA,VGA
                    //     = G  80x25   .       .     mono      .   B000 HERCULES.COM on HGC [14]
                    println!("XXX video: set video mode to 640x200, 2 colors");
                }
                0x11 => {
                    // 11h = G  80x30  8x16  640x480  mono      .   A000 VGA,MCGA,ATI EGA,ATI VIP
                    println!("XXX video: set video mode to 640x480, mono");
                }
                0x12 => {
                    // 12h = G  80x30  8x16  640x480   16/256K  .   A000 VGA,ATI VIP
                    //     = G  80x30  8x16  640x480   16/64    .   A000 ATI EGA Wonder
                    //     = G    .     .    640x480   16       .     .  UltraVision+256K EGA
                    println!("XXX video: set video mode to 640x480, 16 colors");
                }
                0x13 => {
                    // 13h = G  40x25  8x8   320x200  256/256K  .   A000 VGA,MCGA,ATI VIP
                    println!("XXX video: set video mode to 320x200, 256 colors (VGA) after {} instr", cpu.instruction_count);
                }
                _ => {
                    println!("video error: unknown video mode {:02X}",
                             cpu.r16[AX].lo_u8());
                }
            }
        }
        0x01 => {
            // VIDEO - SET TEXT-MODE CURSOR SHAPE
            //
            // CH = cursor start and options (see #00013)
            // CL = bottom scan line containing cursor (bits 0-4)

            // Return:
            // Nothing
            println!("XXX set text-mode cursor shape, start_options={:02X}, bottom_line={:02X}",
                     cpu.r16[CX].hi_u8(),
                     cpu.r16[CX].lo_u8());

        }
        0x02 => {
            // VIDEO - SET CURSOR POSITION
            //
            // BH = page number
            // 0-3 in modes 2&3
            // 0-7 in modes 0&1
            // 0 in graphics modes
            // DH = row (00h is top)
            // DL = column (00h is left)
            // Return: Nothing
            println!("XXX set cursor position, page={}, row={}, column={}",
                     cpu.r16[BX].hi_u8(),
                     cpu.r16[DX].hi_u8(),
                     cpu.r16[DX].lo_u8());
        }
        0x06 => {
            // VIDEO - SCROLL UP WINDOW
            //
            // AL = number of lines by which to scroll up (00h = clear entire window)
            // BH = attribute used to write blank lines at bottom of window
            // CH,CL = row,column of window's upper left corner
            // DH,DL = row,column of window's lower right corner
            // Return: Nothing
            //
            // Note: Affects only the currently active page (see AH=05h)
            println!("XXX scroll window up: lines={},attrib={},topleft={},{},btmright={},{}",
                     cpu.r16[AL].lo_u8(),
                     cpu.r16[BX].hi_u8(),
                     cpu.r16[CX].hi_u8(),
                     cpu.r16[CX].lo_u8(),
                     cpu.r16[DX].hi_u8(),
                     cpu.r16[DX].lo_u8());
        }
        0x09 => {
            // VIDEO - WRITE CHARACTER AND ATTRIBUTE AT CURSOR POSITION
            //
            // AL = character to display
            // BH = page number (00h to number of pages - 1) (see #00010)
            //      background color in 256-color graphics modes (ET4000)
            // BL = attribute (text mode) or color (graphics mode)
            //      if bit 7 set in <256-color graphics mode, character
            //      is XOR'ed onto screen
            // CX = number of times to write character
            // Return: Nothing
            //
            // Notes: All characters are displayed, including CR, LF, and BS.
            // Replication count in CX may produce an unpredictable result
            // in graphics modes if it is greater than the number of positions
            // remaining in the current row. With PhysTechSoft's PTS ROM-DOS
            // the BH, BL, and CX values are ignored on entry.
            println!("XXX impl VIDEO - WRITE CHARACTER AND ATTRIBUTE AT CURSOR POSITION. char={}, page={}, attrib={}, count={}",
                     cpu.r16[AX].lo_u8() as char,
                     cpu.r16[BX].hi_u8(),
                     cpu.r16[BX].lo_u8(),
                     cpu.r16[CX].val);
        }
        0x0A => {
            // VIDEO - WRITE CHARACTER ONLY AT CURSOR POSITION
            //
            // AL = character to display
            // BH = page number (00h to number of pages - 1) (see #00010)
            //      background color in 256-color graphics modes (ET4000)
            // BL = attribute (PCjr, Tandy 1000 only) or color (graphics mode)
            //      if bit 7 set in <256-color graphics mode, character is XOR'ed
            //      onto screen
            // CX = number of times to write character
            // Return: Nothing
             println!("XXX impl VIDEO - WRITE CHARACTER ONLY AT CURSOR POSITION. char={}, page={}, attrib={}, count={}",
                     cpu.r16[AX].lo_u8() as char,
                     cpu.r16[BX].hi_u8(),
                     cpu.r16[BX].lo_u8(),
                     cpu.r16[CX].val);
        }
        0x0B => {
            match cpu.r16[BX].hi_u8() {
                0x00 => {
                    // VIDEO - SET BACKGROUND/BORDER COLOR
                    // BL = background/border color (border only in text modes)
                    // Return: Nothing
                    println!("XXX set bg/border color to {:02X}", cpu.r16[BX].lo_u8());
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
                    println!("XXX TODO set palette id to {:02X}", cpu.r16[BX].lo_u8());
                }
                _ => {
                    println!("video error: unknown int 10, ah=0B, bh={:02X}",
                             cpu.r16[BX].hi_u8());
                }
            }
        }
        0x0E => {
            // VIDEO - TELETYPE OUTPUT
            // Display a character on the screen, advancing the cursor
            // and scrolling the screen as necessary
            //
            // AL = character to write
            // BH = page number
            // BL = foreground color (graphics modes only)
            // Return: Nothing
            //
            // Notes: Characters 07h (BEL), 08h (BS), 0Ah (LF),
            // and 0Dh (CR) are interpreted and do the expected things.
            // IBM PC ROMs dated 1981/4/24 and 1981/10/19 require
            // that BH be the same as the current active page
            //
            // BUG: If the write causes the screen to scroll, BP is destroyed
            // by BIOSes for which AH=06h destroys BP
            print!("{}", cpu.r16[AX].lo_u8() as char);
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
            match cpu.r16[AX].lo_u8() {
                0x00 => {
                    // VIDEO - SET SINGLE PALETTE REGISTER (PCjr,Tandy,EGA,MCGA,VGA)
                    // BL = palette register number (00h-0Fh)
                    //    = attribute register number (undocumented) (see #00017)
                    // BH = color or attribute register value
                    panic!("XXX VIDEO - SET SINGLE PALETTE REGISTER {:02X}, color = {:02X}",
                             cpu.r16[BX].lo_u8(),
                             cpu.r16[BX].hi_u8());
                }
                0x12 => {
                    // VIDEO - SET BLOCK OF DAC REGISTERS (VGA/MCGA)
                    //
                    // BX = starting color register
                    // CX = number of registers to set
                    // ES:DX -> table of 3*CX bytes where each 3 byte group represents one
                    // byte each of red, green and blue (0-63)
                    let start = cpu.r16[BX].val as usize;
                    let count = cpu.r16[CX].val as usize;
                    println!("VIDEO - SET BLOCK OF DAC REGISTERS (VGA/MCGA) start={}, count={}",
                             start,
                             count);

                    for i in start..(start+count) {
                        let next = (i*3) as u16;
                        let r = cpu.mmu.read_u8(cpu.sreg16[ES],
                                                cpu.r16[DX].val + next) as usize;
                        let g = cpu.mmu.read_u8(cpu.sreg16[ES],
                                                cpu.r16[DX].val + next + 1) as usize;
                        let b = cpu.mmu.read_u8(cpu.sreg16[ES],
                                                cpu.r16[DX].val + next + 2) as usize;

                        // each value is 6 bits (0-63), scale them to 8 bits
                        let r = ((r << 2) & 0xFF) as u8;
                        let g = ((g << 2) & 0xFF) as u8;
                        let b = ((b << 2) & 0xFF) as u8;

                        cpu.gpu.set_palette_r(i, r);
                        cpu.gpu.set_palette_g(i, g);
                        cpu.gpu.set_palette_b(i, b);
                        // println!("set color {}: {}, {}, {}", i, r, g, b);
                    }
                }
                0x17 => {
                    // VIDEO - READ BLOCK OF DAC REGISTERS (VGA/MCGA)
                    // BX = starting palette register
                    // CX = number of palette registers to read
                    // ES:DX -> buffer (3 * CX bytes in size) (see also AX=1012h)
                    // Return:
                    // Buffer filled with CX red, green and blue triples
                    println!("XXX VIDEO - READ BLOCK OF DAC REGISTERS (VGA/MCGA)");
                }
                _ => {
                    println!("int10 error: unknown AH 10, AL={:02X}", cpu.r16[AX].lo_u8());
                }
            }
        }
        0x11 => {
            match cpu.r16[AX].lo_u8() {
                0x24 => {
                    // VIDEO - GRAPH-MODE CHARGEN - LOAD 8x16 GRAPHICS CHARS (VGA,MCGA)
                    // BL = row specifier (see AX=1121h)
                    // Return: Nothing
                    println!("XXX VIDEO - GRAPH-MODE CHARGEN - LOAD 8x16 GRAPHICS CHARS (VGA,MCGA)");
                }
                0x30 => {
                    // VIDEO - GET FONT INFORMATION (EGA, MCGA, VGA)
                    // in:
                    // BH = pointer specifier
                    //    00h INT 1Fh pointer
                    //    01h INT 43h pointer
                    //    02h ROM 8x14 character font pointer
                    //    03h ROM 8x8 double dot font pointer
                    //    04h ROM 8x8 double dot font (high 128 characters)
                    //    05h ROM alpha alternate (9 by 14) pointer (EGA,VGA)
                    //    06h ROM 8x16 font (MCGA, VGA)
                    //    07h ROM alternate 9x16 font (VGA only) (see #00021)
                    //    11h (UltraVision v2+) 8x20 font (VGA) or 8x19 font (autosync EGA)
                    //    12h (UltraVision v2+) 8x10 font (VGA) or 8x11 font (autosync EGA)
                    // return:
                    // ES:BP = specified pointer
                    // CX    = bytes/character of on-screen font (not the requested font!)
                    // DL    = highest character row on screen
                    println!("stub int10 - VIDEO - GET FONT INFORMATION (EGA, MCGA, VGA)");
                }
                _ => {
                    println!("int10 error: unknown AH 11, AL={:02X}", cpu.r16[AX].lo_u8());
                }
            }
        }
        0x13 => {
            // VIDEO - WRITE STRING (AT and later,EGA)
            //
            // AH = 13h
            // AL = write mode:
            //      bit 0: Update cursor after writing
            //      bit 1: String contains alternating characters and attributes
            //      bits 2-7: Reserved (0).
            // BH = page number.
            // BL = attribute if string contains only characters.
            // CX = number of characters in string.
            // DH,DL = row,column at which to start writing.
            // ES:BP -> string to write
            println!("XXX int10: VIDEO - WRITE STRING unhandled");
        }
        0x4F => {
            // VESA
            match cpu.r16[AX].lo_u8() {
                0x01 => {
                    // VESA SuperVGA BIOS - GET SuperVGA MODE INFORMATION
                    // CX = SuperVGA video mode (see #04082 for bitfields)
                    // ES:DI -> 256-byte buffer for mode information (see #00079)
                    // Return:
                    // AL = 4Fh if function supported
                    // AH = status:
                    //      00h successful, ES:DI buffer filled
                    //      01h failed
                    println!("XXX VESA SuperVGA BIOS - GET SuperVGA MODE INFORMATION. cx={:04X}", cpu.r16[CX].val);
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
                    println!("XXX VESA SuperVGA BIOS - SET SuperVGA VIDEO MODE. bx={:04X}", cpu.r16[BX].val);
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
                    println!("XXX VESA SuperVGA BIOS - CPU VIDEO MEMORY CONTROL. bh={:02X}", cpu.r16[BX].hi_u8());
                }
                 _ => {
                    println!("int10 error: unknown AH 4F (VESA), AL={:02X}", cpu.r16[AX].lo_u8());
                }
            }
        }
        _ => {
            println!("int10 error: unknown AH={:02X}, AX={:04X}",
                     cpu.r16[AX].hi_u8(),
                     cpu.r16[AX].val);
        }
    }
}

