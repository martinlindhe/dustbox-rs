use time;

use cpu::CPU;
use register::{AX, BX, CX, DX, DS, ES};

// dos related interrupts
pub fn handle(cpu: &mut CPU, deterministic: bool) {
    match cpu.r16[AX].hi_u8() {
        0x06 => {
            // DOS 1+ - DIRECT CONSOLE OUTPUT
            //
            // DL = character (except FFh)
            //
            // Notes: Does not check ^C/^Break. Writes to standard output,
            // which is always the screen under DOS 1.x, but may be redirected
            // under DOS 2+
            let b = cpu.r16[DX].lo_u8();
            if b != 0xFF {
                print!("{}", b as char);
            } else {
                println!("XXX character out: {:02X}", b);
            }
            // Return:
            // AL = character output (despite official docs which
            // state nothing is returned) (at least DOS 2.1-7.0)
            cpu.r16[AX].set_lo(b);
        }
        0x09 => {
            // DOS 1+ - WRITE STRING TO STANDARD OUTPUT
            //
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
                let b = cpu.mmu.read_u8(
                    cpu.sreg16[DS],
                    cpu.r16[DX].val+count
                ) as char;

                count += 1;
                if b == '$' {
                    break;
                }
                print!("{}", b as char);
            }
            cpu.r16[AX].set_lo(b'$');
        }
        0x0C => {
            // DOS 1+ - FLUSH BUFFER AND READ STANDARD INPUT
            // AL = STDIN input function to execute after flushing buffer
            // other registers as appropriate for the input function
            // Return: As appropriate for the specified input function
            //
            // Note: If AL is not one of 01h,06h,07h,08h, or 0Ah, the
            // buffer is flushed but no input is attempted
            println!("XXX int21, 0x0c - read stdin");
        }
        0x2C => {
            // DOS 1+ - GET SYSTEM TIME
            if deterministic {
                cpu.r16[CX].set_hi(0);
                cpu.r16[CX].set_lo(0);
                cpu.r16[DX].set_hi(0);
                cpu.r16[DX].set_lo(0);
            } else {
                let now = time::now();
                let centi_sec = now.tm_nsec / 1000_0000; // nanosecond to 1/100 sec
                cpu.r16[CX].set_hi(now.tm_hour as u8); // CH = hour
                cpu.r16[CX].set_lo(now.tm_min as u8); // CL = minute
                cpu.r16[DX].set_hi(now.tm_sec as u8); // DH = second
                cpu.r16[DX].set_lo(centi_sec as u8); // DL = 1/100 second
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
            cpu.r16[AX].set_lo(3); // AL = major version number (00h if DOS 1.x)
            cpu.r16[AX].set_hi(10); // AH = minor version number
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
                     cpu.r16[BX].val,
                     cpu.r16[CX].val,
                     cpu.sreg16[DS],
                     cpu.r16[DX].val);
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
            println!("XXX impl DOS 2+ - ALLOCATE MEMORY. BX={:04X}",
                     cpu.r16[BX].val);
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
            println!("XXX impl DOS 2+ - RESIZE MEMORY BLOCK. BX={:04X}, ES={:04X}",
                     cpu.r16[BX].val,
                     cpu.sreg16[ES]);
        }
        0x4C => {
            // DOS 2+ - EXIT - TERMINATE WITH RETURN CODE
            // AL = return code

            // Notes: Unless the process is its own parent (see #01378 [offset 16h] at AH=26h),
            // all open files are closed and all memory belonging to the process is freed. All
            // network file locks should be removed before calling this function
            let code = cpu.r16[AX].lo_u8();
            println!("DOS - TERMINATE WITH RETURN CODE {:02X}", code);
            cpu.fatal_error = true; // XXX just to stop debugger.run() function
        }
        _ => {
            println!("int21 error: unknown AH={:02X}, AX={:04X}",
                     cpu.r16[AX].hi_u8(),
                     cpu.r16[AX].val);
        }
    }
}
