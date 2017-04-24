use cpu::CPU;
use register::{AX, BX, CX, DX, AL, ES};

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
                0x13 => {
                    // 13h = G  40x25  8x8   320x200  256/256K  .   A000 VGA,MCGA,ATI VIP
                    println!("XXX video: set video mode to 320x200, 256 colors (VGA)");
                }
                _ => {
                    println!("video error: unknown video mode {:02X}",
                             cpu.r16[AX].lo_u8());
                }
            }
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

            println!("XXX write character at pos: char={}, page={}, attrib={}, count={}",
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
                    println!("XXX VIDEO - SET SINGLE PALETTE REGISTER, palette register number {:02X}, color = {:02X}",
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
                    let count = cpu.r16[CX].val as usize;
                    let reg = cpu.r16[BX].val as usize;
                    let mut offset = (cpu.sreg16[ES] as usize * 16) + cpu.r16[DX].val as usize;
                    println!("VIDEO - SET BLOCK OF DAC REGISTERS (VGA/MCGA) start={}, count={}",
                             reg,
                             count);

                    for i in reg..count {
                        let r = cpu.peek_u8_at(offset) as usize;
                        let g = cpu.peek_u8_at(offset + 1) as usize;
                        let b = cpu.peek_u8_at(offset + 2) as usize;

                        // XXX each value is 6 bits (0-63), scale it to 8 bits

                        cpu.gpu.palette[i].r = ((r << 2) & 0xFF) as u8;
                        cpu.gpu.palette[i].g = ((g << 2) & 0xFF) as u8;
                        cpu.gpu.palette[i].b = ((b << 2) & 0xFF) as u8;
                        offset += 3;
                    }
                }
                _ => {
                    println!("int10 error: unknown AH 10, AL={:02X}",
                             cpu.r16[AX].lo_u8());
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
