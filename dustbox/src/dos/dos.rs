use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use chrono::prelude::*;

use crate::cpu::R;
use crate::codepage::cp437;
use crate::cpu::CPU;
use crate::memory::MMU;
use crate::memory::MemoryAddress;
use crate::hex::hex_bytes;
use crate::string::bytes_to_ascii;
use crate::machine::Component;

#[derive(Clone)]
pub struct DOS {
    /// full path + filename to the currently loaded DOS program
    pub program_path: String,

    /// internal file handle map
    pub file_handles: HashMap<u16, PathBuf>,

    pub psp_segment: u16,
}

impl DOS {
    pub fn default() -> Self {
        Self {
            program_path: String::new(),
            file_handles: HashMap::new(),
            psp_segment: 0,
        }
    }

    /// returns a new file handle
    fn open_existing_file(&mut self, path: PathBuf) -> u16 {
        for n in 0x05..0x100 {
            match self.file_handles.get(&n) {
                None => {
                    self.file_handles.insert(n, path);
                    return n;
                }
                _ => {},
            }
        }
        unreachable!();
    }

    fn get_path_from_handle(&self, handle: u16) -> Option<&PathBuf> {
        self.file_handles.get(&handle)
    }
}

impl Component for DOS {
    /// handles DOS interrupts 0x20 and 0x21
    fn int(&mut self, int: u8, cpu: &mut CPU, mmu: &mut MMU) -> bool {
        if int == 0x20 {
            // DOS 1+ - TERMINATE PROGRAM
            // NOTE: Windows overloads INT 20
            println!("INT 20 - TERMINATE PROGRAM");
            cpu.fatal_error = true; // stops execution
            return true;
        }
        if int != 0x21 {
            return false;
        }
        match cpu.get_r8(R::AH) {
            0x00 => {
                // DOS 1+ - TERMINATE PROGRAM
                println!("DOS 1+ - TERMINATE PROGRAM");
                cpu.fatal_error = true; // XXX just to stop debugger.run() function
            }
            0x02 => {
                // DOS 1+ - WRITE CHARACTER TO STANDARD OUTPUT
                // DL = character to write
                let dl = cpu.get_r8(R::DL);

                // XXX set with video functions
                print!("{}", cp437::u8_as_char(dl));
                // Return:
                // AL = last character output (despite the official docs which state
                // nothing is returned) (at least DOS 2.1-7.0)
                cpu.set_r8(R::AL, dl);
            }
            0x06 => {
                // DOS 1+ - DIRECT CONSOLE OUTPUT
                // DL = character (except FFh)
                //
                // Notes: Does not check ^C/^Break. Writes to standard output,
                // which is always the screen under DOS 1.x, but may be redirected
                // under DOS 2+

                // XXX set with video functions
                let dl = cpu.get_r8(R::DL);
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
                cpu.set_r8(R::AL, dl);
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
                // Notes: ^C/^Break are checked, and INT 23 is called if either pressed.
                // Standard output is always the screen under DOS 1.x, but may be
                // redirected under DOS 2+. Under the FlashTek X-32 DOS extender,
                // the pointer is in DS:EDX
                //let s = mmu.read_asciid(cpu.get_r16(R::DS), cpu.get_r16(R::DX));
                
                let mut count = 0;
                loop {
                    let b = mmu.read_u8(cpu.get_r16(R::DS), cpu.get_r16(R::DX) + count);
                    count += 1;
                    if b as char == '$' {
                        break;
                    }
                    print!("{}", cp437::u8_as_char(b));
                    // machine.gpu_mut.write_char(&mut machine.mmu, b as u16, 0, 0, 1, false);
                }
                //cpu.set_r8(R::AL, b'$');
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

                let al = cpu.get_r8(R::AL);
                match al {
                    0x01 | 0x06 | 0x07 | 0x08 | 0x0A => {
                        // execute next function
                        let old_ah = cpu.get_r8(R::AH);
                        cpu.set_r8(R::AH, al);
                        cpu.execute_interrupt(mmu, 0x21);
                        cpu.set_r8(R::AH, old_ah);
                    }
                    _ => {},
                }
            }
            0x19 => {
                // DOS 1+ - GET CURRENT DEFAULT DRIVE
                // Return: AL = drive (00h = A:, 01h = B:, etc)
                println!("XXX DOS - GET CURRENT DEFAULT DRIVE");
                cpu.set_r8(R::AL, 0); // XXX drive = A:
            }
            0x1A => {
                // DOS 1+ - SET DISK TRANSFER AREA ADDRESS
                // DS:DX -> Disk Transfer Area (DTA)
                // Notes: The DTA is set to PSP:0080h when a program is started.
                let seg = cpu.get_r16(R::DS);
                let off = cpu.get_r16(R::DX);
                println!("XXX DOS - SET DISK TRANSFER AREA ADDRESS {:04X}:{:04X}", seg, off);
            }
            0x25 => {
                // DOS 1+ - SET INTERRUPT VECTOR
                let seg = cpu.get_r16(R::DS);
                let off = cpu.get_r16(R::DX);
                let int = cpu.get_r8(R::AL);
                mmu.write_vec(u16::from(int), MemoryAddress::LongSegmentOffset(seg, off));
            }
            0x2C => {
                // DOS 1+ - GET SYSTEM TIME
                if cpu.deterministic {
                    cpu.set_r16(R::CX, 0);
                    cpu.set_r16(R::DX, 0);
                } else {
                    let now = chrono::Local::now();
                    let centi_sec = now.nanosecond() / 1000_0000; // nanosecond to 1/100 sec
                    cpu.set_r8(R::CH, now.hour() as u8);    // hour
                    cpu.set_r8(R::CL, now.minute() as u8);  // minute
                    cpu.set_r8(R::DH, now.second() as u8);  // second
                    cpu.set_r8(R::DL, centi_sec as u8);     // 1/100 second
                }
            }
            0x2F => {
                // DOS 2+ - GET DISK TRANSFER AREA ADDRESS
                // Return: ES:BX -> current DTA
                println!("XXX DOS - GET DISK TRANSFER AREA ADDRESS");
            }
            0x30 => {
                // DOS 2+ - GET DOS VERSION
                cpu.set_r8(R::AL, 5); // major version number
                cpu.set_r8(R::AH, 0); // minor version number
                cpu.set_r8(R::BH, 0xFF); // indicates MS-DOS
            }
            0x31 => {
                // DOS 2+ - TERMINATE AND STAY RESIDENT
                // AL = return code
                // DX = number of paragraphs to keep resident
                // Return: Never
                let code = cpu.get_r8(R::AL);
                let paragraphs = cpu.get_r16(R::DX);
                println!("XXX DOS - TERMINATE AND STAY RESIDENT, code:{:02X}, paragraphs:{:04X}", code, paragraphs);
                cpu.fatal_error = true;
            }
            0x33 => {
                // DOS 2+ - EXTENDED BREAK CHECKING
                // AL = subfunction
                // 00h get current extended break state
                // Return:
                // DL = current state, 00h = off, 01h = on
                // 01h set state of extended ^C/^Break checking
                // DL = new state
                // 00h off, check only on character I/O functions
                // 01h on, check on all DOS functions
                let al = cpu.get_r8(R::AL);
                println!("XXX DOS - EXTENDED BREAK CHECKING, al:{:02X}", al);
            }
            0x35 => {
                // DOS 2+ - GET INTERRUPT VECTOR
                let int = cpu.get_r8(R::AL);
                let (seg, off) = mmu.read_vec(u16::from(int));
                cpu.set_r16(R::ES, seg);
                cpu.set_r16(R::BX, off);
            }
            0x3D => {
                // DOS 2+ - OPEN - OPEN EXISTING FILE
                let mode = cpu.get_r8(R::AL); // access and sharing modes (see #01402)
                let attr = cpu.get_r8(R::CL); // attribute mask of files to look for (server call only)
                // DS:DX -> ASCIZ filename
                let ds = cpu.get_r16(R::DS);
                let dx = cpu.get_r16(R::DX);
                let data = mmu.readz(ds, dx);
                let filename = cp437::to_utf8(&data);

                // XXX need to find file match with varying case
                let to_load = Path::new(&self.program_path).parent().unwrap().join(filename);
                if to_load.exists() {
                    println!("OPEN - OPEN EXISTING FILE {}, mode {:02X}, attr {:02X}", to_load.display(), mode, attr);
                    // CF clear if successful and AX = file handle
                    let handle = self.open_existing_file(to_load);
                    cpu.regs.flags.carry = false;
                    cpu.set_r16(R::AX, handle);
                } else {
                    // CF set on error and AX = error code (01h,02h,03h,04h,05h,0Ch,56h) (see #01680 at AH=59h)
                    println!("OPEN - OPEN EXISTING FILE {} - NOT FOUND", to_load.display());
                    cpu.regs.flags.carry = true;
                    cpu.set_r16(R::AX, 0x0002); // 2 = "file not found"
                }
            }
            0x3E => {
                // DOS 2+ - CLOSE - CLOSE FILE
                let handle = cpu.get_r16(R::BX); // file handle
                if let Some(_) = self.get_path_from_handle(handle) {
                    println!("CLOSE - CLOSE FILE, handle {:04X}", handle);
                    self.file_handles.remove(&handle);
                    // CF clear if successful and AX destroyed
                    cpu.regs.flags.carry = false;
                } else {
                    // CF set on error and AX = error code (06h) (see #01680 at AH=59h/BX=0000h)
                    cpu.regs.flags.carry = true;
                    println!("XXX - ignoring close unknown handle {}", handle);
                }
            }
            0x3F => {
                // DOS 2+ - READ - READ FROM FILE OR DEVICE
                let handle = cpu.get_r16(R::BX); // file handle
                let len = cpu.get_r16(R::CX) as usize; // number of bytes to read
                // DS:DX -> buffer for data
                let ds = cpu.get_r16(R::DS);
                let dx = cpu.get_r16(R::DX);
                println!("READ - READ FROM FILE OR DEVICE, handle {:04X}, len {}, buffer at {:04X}:{:04X}", handle, len, ds, dx);

                if let Some(path) = self.get_path_from_handle(handle) {
                    if let Ok(f) = File::open(path) {
                        // read up to `len` bytes
                        let mut buf = vec![0u8; len];
                        let mut handle = f.take(len as u64);
                        match handle.read(&mut buf) {
                            Ok(read_bytes) => {
                                // XXX 3. write N bytes to DS:DX
                                mmu.write(ds, dx, &buf);

                                // XXX set AX to number of bytes that was read
                                cpu.regs.flags.carry = false;
                                cpu.set_r16(R::AX, read_bytes as u16);
                                if read_bytes != len {
                                    println!("--- wanted {} bytes, read {} bytes", len, read_bytes);
                                }
                            }
                            Err(e) => panic!(e),
                        };
                    }
                }
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
                let ds = cpu.get_r16(R::DS);
                let dx = cpu.get_r16(R::DX);
                let count = cpu.get_r16(R::CX);
                println!("XXX DOS - WRITE TO FILE OR DEVICE, handle={:04X}, count={:04X}, data from {:04X}:{:04X}",
                        cpu.get_r16(R::BX),
                        count,
                        ds,
                        dx);

                let data = mmu.read(ds, dx, count as usize);
                println!("  -- DATA: {} {}", hex_bytes(&data), bytes_to_ascii(&data));
            }
            0x43 => {
                match cpu.get_r8(R::AL) {
                    0x00 => {
                        // EXTENDED MEMORY SPECIFICATION (XMS) v2+ - INSTALLATION CHECK
                        // Return:
                        // AL = 80h XMS driver installed
                        // AL <> 80h no driver
                        cpu.set_r8(R::AL, 0); // signals that XMS is not installed
                        println!("XXX DOS - XMS INSTALLATION CHECK");
                    }
                    _ => println!("int21 (dos) error: xms ah=43, al={:02X}",
                        cpu.get_r8(R::AL)),
                }
            }
            0x44 => {
                match cpu.get_r8(R::AL) {
                    0x00 => {
                        // DOS 2+ - IOCTL - GET DEVICE INFORMATION
                        // BX = handle
                        // Return:
                        // CF clear if successful
                        // DX = device information word (see #01423)
                        // CF set on error
                        // AX = error code (01h,05h,06h) (see #01680 at AH=59h/BX=0000h)
                        println!("XXX DOS - IOCTL - GET DEVICE INFORMATION, handle={:04X}",  cpu.get_r16(R::BX))
                    }
                    0x01 => {
                        // DOS 2+ - IOCTL - SET DEVICE INFORMATION
                        // BX = handle (must refer to character device)
                        // DX = device information word (see #01423)
                        // (DH must be zero for DOS version prior to 6.x)
                        // Return:
                        // CF clear if successful / set on error
                        // AX = error code (01h,05h,06h,0Dh) (see #01680 at AH=59h/BX=0000h)
                        println!("XXX DOS - IOCTL - SET DEVICE INFORMATION, handle={:04X}, device:{:04X}",  cpu.get_r16(R::BX),  cpu.get_r16(R::DX));
                    }
                    _ => println!("int21 (dos) error: ioctl ah=44, al={:02X}",
                        cpu.get_r8(R::AL)),
                }
            }
            0x47 => {
                // DOS 2+ - CWD - GET CURRENT DIRECTORY
                // DL = drive number (00h = default, 01h = A:, etc)
                // DS:SI -> 64-byte buffer for ASCIZ pathname

                // Return:
                // CF clear if successful
                // AX = 0100h (undocumented)
                // CF set on error
                // AX = error code (0Fh) (see #01680 at AH=59h/BX=0000h)
                let ds = cpu.get_r16(R::DS);
                let si = cpu.get_r16(R::SI);
                println!("XXX DOS - CWD - GET CURRENT DIRECTORY. dl={:02X}, DS:SI={:04X}:{:04X}",
                    cpu.get_r8(R::DL), ds, si);
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
                        cpu.get_r16(R::BX));

                // SIGNAL FAILURE
                cpu.set_r16(R::AX, 0x0008); // out of memory
                cpu.set_r16(R::BX, 0x0000);
                cpu.regs.flags.carry = true;
            }
            0x49 => {
                // DOS 2+ - FREE MEMORY
                // ES = segment of block to free
                // Return:
                // CF clear if successful
                // CF set on error
                // AX = error code (07h,09h) (see #01680 at AH=59h/BX=0000h)
                println!("XXX impl DOS 2+ - FREE MEMORY. es={:04X}",
                        cpu.get_r16(R::ES));
                cpu.regs.flags.carry = false; // fake success
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
                        cpu.get_r16(R::BX),
                        cpu.get_r16(R::ES));
                cpu.regs.flags.carry = false; // fake success
            }
            0x4B => {
                // DOS 2+ - EXEC - LOAD AND/OR EXECUTE PROGRAM
                // AL = type of load
                //  00h load and execute
                //  01h load but do not execute
                //  03h load overlay (see #01591)
                //  04h load and execute in background (European MS-DOS 4.0 only)
                // "Exec & Go" (see also AH=80h)
                // DS:DX -> ASCIZ program name (must include extension)
                // ES:BX -> parameter block (see #01590,#01591,#01592)
                // CX = mode (subfunction 04h only)
                //  0000h child placed in zombie mode after termination
                //  0001h child's return code discarded on termination
                // Return:
                // CF clear if successful
                // BX,DX destroyed
                // if subfunction 01h, process ID set to new program's PSP; get with
                // INT 21/AH=62h
                // CF set on error
                // AX = error code (01h,02h,05h,08h,0Ah,0Bh) (see #01680 at AH=59h)

                let mode = cpu.get_r8(R::AL);
                let name = mmu.read_asciiz(cpu.get_r16(R::DS), cpu.get_r16(R::DX));
                println!("XXX DOS - EXEC - LOAD AND/OR EXECUTE PROGRAM {}, mode {:02X}", name, mode);
            }
            0x4C => {
                // DOS 2+ - EXIT - TERMINATE WITH RETURN CODE
                // AL = return code

                // Notes: Unless the process is its own parent (see #01378 [offset 16h] at AH=26h),
                // all open files are closed and all memory belonging to the process is freed. All
                // network file locks should be removed before calling this function
                let al = cpu.get_r8(R::AL);
                println!("DOS - TERMINATE WITH RETURN CODE {:02X}", al);
                cpu.fatal_error = true; // XXX just to stop debugger.run() function
            }
            0x4D => {
                // DOS 2+ - GET RETURN CODE (ERRORLEVEL)
                // Return:
                // AH = termination type
                // 00h normal (INT 20,INT 21/AH=00h, or INT 21/AH=4Ch)
                // 01h control-C abort
                // 02h critical error abort
                // 03h terminate and stay resident (INT 21/AH=31h or INT 27)
                // AL = return code
                // CF clear
                println!("XXX DOS 2+ - GET RETURN CODE");
            }
            0x50 => {
                // DOS 2+ internal - SET CURRENT PROCESS ID (SET PSP ADDRESS)
                // BX = segment of PSP for new process
                let bx = cpu.get_r16(R::BX);
                println!("XXX DOS 2+ - SET CURRENT PROCESS ID, bx={:04X}", bx);
            }
            0x51 => {
                // DOS 2+ internal - GET CURRENT PROCESS ID (GET PSP ADDRESS)
                // Return: BX = segment of PSP for current process
                println!("XXX DOS - GET CURRENT PROCESS ID");
            }
            0x59 => {
                match cpu.get_r16(R::BX) {
                    0x0000 => {
                        // DOS 3.0+ - GET EXTENDED ERROR INFORMATION
                        // Return:
                        // AX = extended error code (see #01680)
                        // BH = error class (see #01682)
                        // BL = recommended action (see #01683)
                        // CH = error locus (see #01684)
                        // ES:DI may be pointer (see #01681, #01680)
                        // CL, DX, SI, BP, and DS destroyed
                        println!("XXX DOS - GET EXTENDED ERROR INFORMATION");
                    }
                    _ => println!("int21 (dos) error: unknown ah=59, bx={:04X}",
                        cpu.get_r16(R::BX)),
                }
            }
            _ => {
                println!("int21 (dos) error: unknown ah={:02X}, ax={:04X}",
                        cpu.get_r8(R::AH),
                        cpu.get_r16(R::AX));
                return false;
            }
        }
        true
    }
}
