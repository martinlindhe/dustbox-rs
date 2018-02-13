use hardware::Hardware;
use cpu::CPU;
use cpu::register::{R8, R16, SR};
use memory::mmu::MMU;

// video related interrupts
pub fn handle(cpu: &mut CPU, mut hw: &mut Hardware) {
    match cpu.get_r8(&R8::AH) {
        0x00 => {
            // VIDEO - SET VIDEO MODE
            // AL = desired video mode
            //
            // Return:
            // AL = video mode flag (Phoenix, AMI BIOS)
            // 20h mode > 7
            // 30h modes 0-5 and 7
            // 3Fh mode 6
            // AL = CRT controller mode byte (Phoenix 386 BIOS v1.10)
            let al = cpu.get_r8(&R8::AL);
            hw.gpu.set_mode(al);
        }
        0x01 => {
            // VIDEO - SET TEXT-MODE CURSOR SHAPE
            //
            // CH = cursor start and options (see #00013)
            // CL = bottom scan line containing cursor (bits 0-4)

            // Return:
            // Nothing
            println!("XXX set text-mode cursor shape, start_options={:02X}, bottom_line={:02X}",
                     cpu.get_r8(&R8::CH),
                     cpu.get_r8(&R8::CL));

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
            let page = cpu.get_r8(&R8::BH);
            let row = cpu.get_r8(&R8::DH);
            let column = cpu.get_r8(&R8::DL);
            println!("XXX set cursor position, page={}, row={}, column={}", page, row, column);
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
                     cpu.get_r8(&R8::AL),
                     cpu.get_r8(&R8::BH),
                     cpu.get_r8(&R8::CH),
                     cpu.get_r8(&R8::CL),
                     cpu.get_r8(&R8::DH),
                     cpu.get_r8(&R8::DL));
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
                     cpu.get_r8(&R8::AL) as char,
                     cpu.get_r8(&R8::BH),
                     cpu.get_r8(&R8::BL),
                     cpu.get_r16(&R16::CX));
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
                     cpu.get_r8(&R8::AL) as char,
                     cpu.get_r8(&R8::BH),
                     cpu.get_r8(&R8::BL),
                     cpu.get_r16(&R16::CX));
        }
        0x0B => {
            match cpu.get_r8(&R8::BH) {
                0x00 => {
                    // VIDEO - SET BACKGROUND/BORDER COLOR
                    // BL = background/border color (border only in text modes)
                    // Return: Nothing
                    println!("XXX set bg/border color, bl={:02X}", cpu.get_r8(&R8::BL));
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
                    println!("XXX TODO set palette id, bl={:02X}", cpu.get_r8(&R8::BL));
                }
                _ => {
                    println!("video error: unknown int 10, ah=0B, bh={:02X}", cpu.get_r8(&R8::BH));
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
            print!("{}", cpu.get_r8(&R8::AL) as char);
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
            match cpu.get_r8(&R8::AL) {
                0x00 => {
                    // VIDEO - SET SINGLE PALETTE REGISTER (PCjr,Tandy,EGA,MCGA,VGA)
                    // BL = palette register number (00h-0Fh)
                    //    = attribute register number (undocumented) (see #00017)
                    // BH = color or attribute register value
                    panic!("XXX VIDEO - SET SINGLE PALETTE REGISTER, bl={:02X}, bh={:02X}",
                             cpu.get_r8(&R8::BL),
                             cpu.get_r8(&R8::BH));
                }
                0x12 => {
                    // VIDEO - SET BLOCK OF DAC REGISTERS (VGA/MCGA)
                    //
                    // BX = starting color register
                    // CX = number of registers to set
                    // ES:DX -> table of 3*CX bytes where each 3 byte group represents one
                    // byte each of red, green and blue (0-63)
                    let start = cpu.get_r16(&R16::BX) as usize;
                    let count = cpu.get_r16(&R16::CX) as usize;

                    // #define VGAREG_DAC_WRITE_ADDRESS       0x3c8
                    cpu.out_u8(&mut hw, 0x3C8, start as u8);

                    let es = cpu.get_sr(&SR::ES);
                    let dx = cpu.get_r16(&R16::DX);

                    for i in start..(start+count) {
                        let next = (i * 3) as u16;
                        let r = hw.mmu.read_u8(es, dx + next) ;
                        let g = hw.mmu.read_u8(es, dx + next + 1);
                        let b = hw.mmu.read_u8(es, dx + next + 2);

                        // #define VGAREG_DAC_DATA                0x3c9
                        cpu.out_u8(&mut hw, 0x3C9, r);
                        cpu.out_u8(&mut hw, 0x3C9, g);
                        cpu.out_u8(&mut hw, 0x3C9, b);
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
                    println!("int10 error: unknown AH 10, al={:02X}", cpu.get_r8(&R8::AL));
                }
            }
        }
        0x11 => {
            match cpu.get_r8(&R8::AL) {
                0x24 => {
                    // VIDEO - GRAPH-MODE CHARGEN - LOAD 8x16 GRAPHICS CHARS (VGA,MCGA)
                    // BL = row specifier (see AX=1121h)
                    // Return: Nothing
                    println!("XXX VIDEO - GRAPH-MODE CHARGEN - LOAD 8x16 GRAPHICS CHARS (VGA,MCGA)");
                }
                0x30 => {
                    // VIDEO - GET FONT INFORMATION (EGA, MCGA, VGA)
                    // return:
                    // ES:BP = specified pointer
                    // CX    = bytes/character of on-screen font (not the requested font!)
                    // DL    = highest character row on screen
                    let bh = cpu.get_r8(&R8::BH);
                    match bh { // BH = pointer specifier
                        // 00h INT 1Fh pointer
                        // 01h INT 43h pointer
                        0x02 => {
                            // 02h ROM 8x14 character font pointer
                            cpu.set_sr(&SR::ES, MMU::segment_from_long_pair(hw.gpu.font_14));
                            cpu.set_r16(&R16::BP, MMU::offset_from_long_pair(hw.gpu.font_14));
                        }
                        // 03h ROM 8x8 double dot font pointer
                        // 04h ROM 8x8 double dot font (high 128 characters)
                        // 05h ROM alpha alternate (9 by 14) pointer (EGA,VGA)
                        0x06 => {
                            // ROM 8x16 font (MCGA, VGA)
                            if hw.gpu.architecture.is_vga() {
                                cpu.set_sr(&SR::ES, MMU::segment_from_long_pair(hw.gpu.font_16));
                                cpu.set_r16(&R16::BP, MMU::offset_from_long_pair(hw.gpu.font_16));
                            }
                        }
                        // 07h ROM alternate 9x16 font (VGA only) (see #00021)
                        // 11h (UltraVision v2+) 8x20 font (VGA) or 8x19 font (autosync EGA)
                        // 12h (UltraVision v2+) 8x10 font (VGA) or 8x11 font (autosync EGA)
                        _ => panic!("VIDEO - GET FONT INFORMATION (EGA, MCGA, VGA): unhandled bh={:02X}", bh),
                    }
                }
                _ => {
                    println!("int10 error: unknown ah=11, al={:02X}", cpu.get_r8(&R8::AL));
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
            match cpu.get_r8(&R8::AL) {
                0x01 => {
                    // VESA SuperVGA BIOS - GET SuperVGA MODE INFORMATION
                    // CX = SuperVGA video mode (see #04082 for bitfields)
                    // ES:DI -> 256-byte buffer for mode information (see #00079)
                    // Return:
                    // AL = 4Fh if function supported
                    // AH = status:
                    //      00h successful, ES:DI buffer filled
                    //      01h failed
                    println!("XXX VESA SuperVGA BIOS - GET SuperVGA MODE INFORMATION. cx={:04X}", cpu.get_r16(&R16::CX));
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
                    println!("XXX VESA SuperVGA BIOS - SET SuperVGA VIDEO MODE. bx={:04X}", cpu.get_r16(&R16::BX));
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
                    println!("XXX VESA SuperVGA BIOS - CPU VIDEO MEMORY CONTROL. bh={:02X}", cpu.get_r8(&R8::BH));
                }
                 _ => {
                    println!("int10 error: unknown AH 4F (VESA), al={:02X}", cpu.get_r8(&R8::AL));
                }
            }
        }
        _ => {
            println!("int10 error: unknown al={:02X}, ax={:04X}",
                     cpu.get_r8(&R8::AL),
                     cpu.get_r16(&R16::AX));
        }
    }
}

