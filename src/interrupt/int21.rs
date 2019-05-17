use chrono::prelude::*;
use std::str;

use crate::cpu::{CPU, R};
use crate::codepage::cp437;
use crate::machine::Machine;
use crate::memory::MemoryAddress;

// dos related interrupts
pub fn handle(machine: &mut Machine) {
    match machine.cpu.get_r8(R::AH) {
        0x00 => {
            // DOS 1+ - TERMINATE PROGRAM
            println!("DOS 1+ - TERMINATE PROGRAM");
            machine.cpu.fatal_error = true; // XXX just to stop debugger.run() function
        }
        0x02 => {
            // DOS 1+ - WRITE CHARACTER TO STANDARD OUTPUT
            // DL = character to write
            let dl = machine.cpu.get_r8(R::DL);

            // XXX set with video functions
            print!("{}", cp437::u8_as_char(dl));
            // Return:
            // AL = last character output (despite the official docs which state
            // nothing is returned) (at least DOS 2.1-7.0)
            machine.cpu.set_r8(R::AL, dl);
        }
        0x06 => {
            // DOS 1+ - DIRECT CONSOLE OUTPUT
            // DL = character (except FFh)
            //
            // Notes: Does not check ^C/^Break. Writes to standard output,
            // which is always the screen under DOS 1.x, but may be redirected
            // under DOS 2+

            // XXX set with video functions
            let dl = machine.cpu.get_r8(R::DL);
            if dl != 0xFF {
                print!("{}", cp437::u8_as_char(dl));

                // XXX instead, we should WRITE to a "dos_stdout" stream
            } else {
                // see dosbox-x/src/dos/dos.cpp:484
                // happens in ../dos-software-decoding/games-com-commercial/Blort\ \(1987\)\(Hennsoft\)/blort.com
                // println!("XXX dl is 0xFF, TODO read input?");
            }
            // Return:
            // AL = character output (despite official docs which
            // state nothing is returned) (at least DOS 2.1-7.0)
            machine.cpu.set_r8(R::AL, dl);
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
                let b = machine.mmu.read_u8(machine.cpu.get_r16(R::DS), machine.cpu.get_r16(R::DX) + count);
                count += 1;
                if b as char == '$' {
                    break;
                }
                print!("{}", cp437::u8_as_char(b));
            }
            machine.cpu.set_r8(R::AL, b'$');
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

            // println!("XXX flush text buffer");

            let al = machine.cpu.get_r8(R::AL);
            match al {
                0x01 | 0x06 | 0x07 | 0x08 | 0x0A => {
                    // execute next function
                    let old_ah = machine.cpu.get_r8(R::AH);
                    machine.cpu.set_r8(R::AH, al);
                    machine.int(0x21);
                    machine.cpu.set_r8(R::AH, old_ah);
                }
                _ => {},
            }
        }
        0x25 => {
            // DOS 1+ - SET INTERRUPT VECTOR
            let seg = machine.cpu.get_r16(R::DS);
            let off = machine.cpu.get_r16(R::DX);
            let int = machine.cpu.get_r8(R::AL);
            machine.mmu.write_vec(u16::from(int), MemoryAddress::LongSegmentOffset(seg, off));
        }
        0x2C => {
            // DOS 1+ - GET SYSTEM TIME
            if machine.cpu.deterministic {
                machine.cpu.set_r16(R::CX, 0);
                machine.cpu.set_r16(R::DX, 0);
            } else {
                let now = chrono::Local::now();
                let centi_sec = now.nanosecond() / 1000_0000; // nanosecond to 1/100 sec
                machine.cpu.set_r8(R::CH, now.hour() as u8);    // hour
                machine.cpu.set_r8(R::CL, now.minute() as u8);  // minute
                machine.cpu.set_r8(R::DH, now.second() as u8);  // second
                machine.cpu.set_r8(R::DL, centi_sec as u8);     // 1/100 second
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
            machine.cpu.set_r8(R::AL, 3); // AL = major version number (00h if DOS 1.x)
            machine.cpu.set_r8(R::AH, 10); // AH = minor version number
        }
        0x35 => {
            // DOS 2+ - GET INTERRUPT VECTOR
            let int = machine.cpu.get_r8(R::AL);
            let (seg, off) = machine.mmu.read_vec(u16::from(int));
            machine.cpu.set_r16(R::ES, seg);
            machine.cpu.set_r16(R::BX, off);
        }
        0x3D => {
            // DOS 2+ - OPEN - OPEN EXISTING FILE
            let mode = machine.cpu.get_r8(R::AL); // access and sharing modes (see #01402)
            let attr = machine.cpu.get_r8(R::CL); // attribute mask of files to look for (server call only)
            // DS:DX -> ASCIZ filename
            let ds = machine.cpu.get_r16(R::DS);
            let dx = machine.cpu.get_r16(R::DX);
            let data = machine.mmu.readz(ds, dx);
            let filename = str::from_utf8(&data).unwrap();
            // Return:
            // CF clear if successful and AX = file handle
            // CF set on error and AX = error code (01h,02h,03h,04h,05h,0Ch,56h) (see #01680 at AH=59h)
            println!("int21 XXX DOS 2+ - OPEN - OPEN EXISTING FILE, name {}, mode {:02X}, attr {:02X}", filename, mode, attr);

            machine.cpu.regs.flags.carry = true; // XXX fake failure
            machine.cpu.set_r16(R::AX, 0x0002); // XXX 2 = "file not found"
        }
        0x3E => {
            // DOS 2+ - CLOSE - CLOSE FILE
            let handle = machine.cpu.get_r16(R::BX); // file handle
            // Return:
            // CF clear if successful and AX destroyed
            // CF set on error and AX = error code (06h) (see #01680 at AH=59h/BX=0000h)
            println!("int21 XXX DOS 2+ - CLOSE - CLOSE FILE, handle {:04X}", handle);
            machine.cpu.regs.flags.carry = false; // XXX fake success
        }
        0x3F => {
            // DOS 2+ - READ - READ FROM FILE OR DEVICE
            let handle = machine.cpu.get_r16(R::BX); // file handle
            let len = machine.cpu.get_r16(R::CX); // number of bytes to read
            // DS:DX -> buffer for data
            let ds = machine.cpu.get_r16(R::DS);
            let dx = machine.cpu.get_r16(R::DX);
            println!("int21 XXX DOS 2+ - READ - READ FROM FILE OR DEVICE, handle {:04X}, len {}, buffer at {:04X}:{:04X}", handle, len, ds, dx);
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
                     machine.cpu.get_r16(R::BX),
                     machine.cpu.get_r16(R::CX),
                     machine.cpu.get_r16(R::DS),
                     machine.cpu.get_r16(R::DX));
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
                     machine.cpu.get_r16(R::BX));
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
                     machine.cpu.get_r16(R::BX),
                     machine.cpu.get_r16(R::ES));
        }
        0x4C => {
            // DOS 2+ - EXIT - TERMINATE WITH RETURN CODE
            // AL = return code

            // Notes: Unless the process is its own parent (see #01378 [offset 16h] at AH=26h),
            // all open files are closed and all memory belonging to the process is freed. All
            // network file locks should be removed before calling this function
            let al = machine.cpu.get_r8(R::AL);
            println!("DOS - TERMINATE WITH RETURN CODE {:02X}", al);
            machine.cpu.fatal_error = true; // XXX just to stop debugger.run() function
        }
        _ => {
            println!("int21 (dos) error: unknown ah={:02X}, ax={:04X}",
                     machine.cpu.get_r8(R::AH),
                     machine.cpu.get_r16(R::AX));
        }
    }
}
