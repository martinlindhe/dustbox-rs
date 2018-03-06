use time;

use hardware::Hardware;
use cpu::CPU;
use cpu::register::R;
use codepage::cp437;
use memory::MemoryAddress;

// dos related interrupts
pub fn handle(cpu: &mut CPU, hw: &mut Hardware) {
    match cpu.get_r8(&R::AH) {
        0x02 => {
            // DOS 1+ - WRITE CHARACTER TO STANDARD OUTPUT
            // DL = character to write
            let dl = cpu.get_r8(&R::DL);
            print!("{}", cp437::u8_as_char(dl));
            // Return:
            // AL = last character output (despite the official docs which state
            // nothing is returned) (at least DOS 2.1-7.0)
            cpu.set_r8(&R::AL, dl);
        }
        0x06 => {
            // DOS 1+ - DIRECT CONSOLE OUTPUT
            // DL = character (except FFh)
            //
            // Notes: Does not check ^C/^Break. Writes to standard output,
            // which is always the screen under DOS 1.x, but may be redirected
            // under DOS 2+
            let dl = cpu.get_r8(&R::DL);
            if dl != 0xFF {
                print!("{}", cp437::u8_as_char(dl));
            }
            // Return:
            // AL = character output (despite official docs which
            // state nothing is returned) (at least DOS 2.1-7.0)
            cpu.set_r8(&R::AL, dl);
        }
        0x07 => {
            // DOS 1+ - DIRECT CHARACTER INPUT, WITHOUT ECHO
            // Return:
            // AL = character read from standard input
            println!("XXX DOS 1+ - DIRECT CHARACTER INPUT, WITHOUT ECHO");
        }
        0x09 => {
            // DOS 1+ - WRITE STRING TO STANDARD OUTPUT
            // DS:DX -> '$'-terminated string
            //
            // Return:
            // AL = 24h (the '$' terminating the string, despite official docs which
            // state that nothing is returned) (at least DOS 2.1-7.0 and NWDOS)
            //
            // Notes: ^C/^Break are checked, and INT 23 is called if either pressed.
            // Standard output is always the screen under DOS 1.x, but may be
            // redirected under DOS 2+. Under the FlashTek X-32 DOS extender,
            // the pointer is in DS:EDX
            let mut count = 0;
            loop {
                let b = hw.mmu.read_u8(cpu.get_r16(&R::DS), cpu.get_r16(&R::DX) + count);
                count += 1;
                if b as char == '$' {
                    break;
                }
                print!("{}", cp437::u8_as_char(b));
            }
            cpu.set_r8(&R::AL, b'$');
        }
        0x0B => {
            // DOS 1+ - GET STDIN STATUS
            // Return:
            // AL = status
            // 00h if no character available
            // FFh if character is available
            println!("XXX DOS 1+ - GET STDIN STATUS");
        }
        0x0C => {
            // DOS 1+ - FLUSH BUFFER AND READ STANDARD INPUT
            // AL = STDIN input function to execute after flushing buffer
            // other registers as appropriate for the input function
            // Return: As appropriate for the specified input function
            //
            // Note: If AL is not one of 01h,06h,07h,08h, or 0Ah, the
            // buffer is flushed but no input is attempted
            // println!("XXX int21, 0x0c - read stdin");
        }
        0x25 => {
            // DOS 1+ - SET INTERRUPT VECTOR
            let seg = cpu.get_r16(&R::DS);
            let off = cpu.get_r16(&R::DX);
            let int = cpu.get_r8(&R::AL);
            hw.mmu.write_vec(int as u16, &MemoryAddress::LongSegmentOffset(seg, off));
        }
        0x2C => {
            // DOS 1+ - GET SYSTEM TIME
            if cpu.deterministic {
                cpu.set_r16(&R::CX, 0);
                cpu.set_r16(&R::DX, 0);
            } else {
                let now = time::now();
                let centi_sec = now.tm_nsec / 1000_0000; // nanosecond to 1/100 sec
                cpu.set_r8(&R::CH, now.tm_hour as u8); // hour
                cpu.set_r8(&R::CL, now.tm_min as u8);  // minute
                cpu.set_r8(&R::DH, now.tm_sec as u8);  // second
                cpu.set_r8(&R::DL, centi_sec as u8);   // 1/100 second
            }
        }
        0x30 => {
            // DOS 2+ - GET DOS VERSION
            // ---DOS 5+ ---
            // AL = what to return in BH
            // 00h OEM number (see #01394)
            // 01h version flag
            //
            // Return:
            // AL = major version number (00h if DOS 1.x)
            // AH = minor version number
            // BL:CX = 24-bit user serial number (most versions do not use this)
            // ---if DOS <5 or AL=00h---
            // BH = MS-DOS OEM number (see #01394)
            // ---if DOS 5+ and AL=01h---
            // BH = version flag
            //
            // bit 3: DOS is in ROM

            // (Table 01394)
            // Values for DOS OEM number:
            // 00h *  IBM
            // -  (Novell DOS, Caldera OpenDOS, DR-OpenDOS, and DR-DOS 7.02+ report IBM
            // as their OEM)
            // 01h *  Compaq
            // 02h *  MS Packaged Product
            // 04h *  AT&T
            // 05h *  ZDS (Zenith Electronics, Zenith Electronics).

            // fake MS-DOS 3.10, as needed by msdos32/APPEND.COM
            cpu.set_r8(&R::AL, 3); // AL = major version number (00h if DOS 1.x)
            cpu.set_r8(&R::AH, 10); // AH = minor version number
        }
        0x35 => {
            // DOS 2+ - GET INTERRUPT VECTOR
            let int = cpu.get_r8(&R::AL);
            let (seg, off) = hw.mmu.read_vec(int as u16);
            cpu.set_r16(&R::ES, seg);
            cpu.set_r16(&R::BX, off);
        }
        0x40 => {
            // DOS 2+ - WRITE - WRITE TO FILE OR DEVICE

            // BX = file handle
            // CX = number of bytes to write
            // DS:DX -> data to write
            //
            // Return:
            // CF clear if successful
            // AX = number of bytes actually written
            // CF set on error
            // AX = error code (05h,06h) (see #01680 at AH=59h/BX=0000h)

            // Notes: If CX is zero, no data is written, and the file is truncated or extended
            // to the current position. Data is written beginning at the current file position,
            // and the file position is updated after a successful write. For FAT32 drives, the
            // file must have been opened with AX=6C00h with the "extended size" flag in order
            // to expand the file beyond 2GB; otherwise the write will fail with error code
            // 0005h (access denied). The usual cause for AX < CX on return is a full disk
            println!("XXX DOS - WRITE TO FILE OR DEVICE, handle={:04X}, count={:04X}, data from {:04X}:{:04X}",
                     cpu.get_r16(&R::BX),
                     cpu.get_r16(&R::CX),
                     cpu.get_r16(&R::DS),
                     cpu.get_r16(&R::DX));
        }
        0x48 => {
            // DOS 2+ - ALLOCATE MEMORY
            // BX = number of paragraphs to allocate
            // Return:
            // CF clear if successful
            // AX = segment of allocated block
            // CF set on error
            // AX = error code (07h,08h) (see #01680 at AH=59h/BX=0000h)
            // BX = size of largest available block
            println!("XXX impl DOS 2+ - ALLOCATE MEMORY. bx={:04X}",
                     cpu.get_r16(&R::BX));
        }
        0x4A => {
            // DOS 2+ - RESIZE MEMORY BLOCK
            // BX = new size in paragraphs
            // ES = segment of block to resize
            // Return:
            // CF clear if successful
            // CF set on error
            // AX = error code (07h,08h,09h) (see #01680 at AH=59h/BX=0000h)
            // BX = maximum paragraphs available for specified memory block
            println!("XXX impl DOS 2+ - RESIZE MEMORY BLOCK. bx={:04X}, es={:04X}",
                     cpu.get_r16(&R::BX),
                     cpu.get_r16(&R::ES));
        }
        0x4C => {
            // DOS 2+ - EXIT - TERMINATE WITH RETURN CODE
            // AL = return code

            // Notes: Unless the process is its own parent (see #01378 [offset 16h] at AH=26h),
            // all open files are closed and all memory belonging to the process is freed. All
            // network file locks should be removed before calling this function
            let al = cpu.get_r8(&R::AL);
            println!("DOS - TERMINATE WITH RETURN CODE {:02X}", al);
            cpu.fatal_error = true; // XXX just to stop debugger.run() function
        }
        _ => {
            println!("int21 error: unknown ah={:02X}, ax={:04X}",
                     cpu.get_r8(&R::AH),
                     cpu.get_r16(&R::AX));
        }
    }
}
