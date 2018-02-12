use super::cpu::*;
use cpu::register::R16;
use interrupt;
use hardware::Hardware;

impl CPU {
    // read byte from I/O port
    pub fn in_u8(&mut self, hw: &mut Hardware, port: u16) -> u8 {
        // println!("in_port: read from {:04X} at {:06X}", port, self.get_offset());
        match port {
            // PORT 0000-001F - DMA 1 - FIRST DIRECT MEMORY ACCESS CONTROLLER (8237)
            0x0002 => {
                // DMA channel 1	current address		byte  0, then byte 1
                println!("XXX fixme in_port read DMA channel 1 current address");
                0
            }
            0x0020 => self.pic.get_register(),
            0x0021 => self.pic.get_ocw1(),
            0x0040 => self.pit.counter0.get_next_u8(),
            0x0041 => self.pit.counter1.get_next_u8(),
            0x0042 => self.pit.counter2.get_next_u8(),
            0x0060 => {
                // keyboard controller data output buffer
                0 // XXX
            },
            0x0061 => {
                // keyboard controller port b control register
                0 // XXX
            }
            0x00A0 => self.pic2.get_register(),
            0x00A1 => self.pic2.get_ocw1(),
            0x0201 => {
                // read joystick position and status
                // Bit(s)	Description	(Table P0542)
                //  7	status B joystick button 2 / D paddle button
                //  6	status B joystick button 1 / C paddle button
                //  5	status A joystick button 2 / B paddle button
                //  4	status A joystick button 1 / A paddle button
                //  3	B joystick Y coordinate	   / D paddle coordinate
                //  2	B joystick X coordinate	   / C paddle coordinate
                //  1	A joystick Y coordinate	   / B paddle coordinate
                //  0	A joystick X coordinate	   / A paddle coordinate
                0 // XXX
            }
            0x03DA => hw.gpu.read_cga_status_register(),
            _ => {
                println!("in_u8: unhandled in8 {:04X}", port);
                0
            }
        }
    }

    // write byte to I/O port
    pub fn out_u8(&mut self, hw: &mut Hardware, port: u16, data: u8) {
        match port {
            0x0020 => self.pic.set_command(data),
            0x0021 => self.pic.set_data(data),
            0x0040 => self.pit.counter0.write_reload_part(data),
            0x0041 => self.pit.counter1.write_reload_part(data),
            0x0042 => self.pit.counter2.write_reload_part(data),
            0x0043 => self.pit.set_mode_command(data),
            0x0061 => {
                // keyboard controller port b OR ppi programmable perihpial interface (XT only) - which mode are we in?
            },
            0x00A0 => self.pic2.set_command(data),
            0x00A1 => self.pic2.set_data(data),
            0x0201 => {
                // W  fire joystick's four one-shots
            }
            0x03C7 => hw.gpu.set_pel_address(data), // XXX unsure if understood correctly
            0x03C8 => hw.gpu.set_pel_address(data),
            0x03C9 => hw.gpu.set_pel_data(data),
            0x03D4 => {
                // CRT (6845) register index XXX
            }
            0x03D5 => {
                // CRT (6845) data register XXX
            }
            0x03D8 => {
                // RW  CGA mode control register  (except PCjr) (see #P0817)
	            // cannot be found on native color EGA, color VGA, but on most clones
            }
            0x03D9 => {
                // XXX CGA palette register!!!
            }
            0x03DA => {
                // 03DA  -W  color EGA/color VGA feature control register (see #P0820)
	            //  (at PORT 03BAh w in mono mode, VGA: 3CAh r)
                // 03DA  -W  HZ309 (MDA/HGC/CGA clone) card from in Heath/Zenith HZ150 PC
                //  bit7-1=0: unknown, zero is default and known to function
                //            properly at least in CGA modes.
                //  bit 0 = 1 override 3x8h bit3 control register that switches
                //            CRT beam off if bit3 is cleared. So screens always
                //            stays on.
                //  bit 0 = 0 3x8h bit3 indicates if CRT beam is on or off.
                //            No more info available. Might conflict with EGA/VGA.
            }
            _ => println!("ERROR: unhandled out_u8 to port {:04X}, data {:02X}", port, data),
        }
    }

    // write word to I/O port
    pub fn out_u16(&mut self, hw: &mut Hardware, port: u16, data: u16) {
        match port {
            0x03C4 => {
                // XXX
                /*
                03C4  -W  EGA  TS index register
                        bit7-3 : reserved (VGA only)
                        bit2-0 : current TS index
                03C4  RW  VGA  sequencer register index (see #P0670)
                */
            }
            /*
            0x03C5 => {
                03C5  -W  EGA  TS data register
                03C5  RW  VGA  sequencer register data
            }
            PORT 03D4-03D5 - COLOR VIDEO - CRT CONTROL REGISTERS
            */
            0x03D4 => {
                // 03D4  rW  CRT (6845) register index   (CGA/MCGA/color EGA/color VGA)
              // selects which register (0-11h) is to be accessed through 03D5
               // this port is r/w on some VGA, e.g. ET4000
                //        bit 7-6 =0: (VGA) reserved
                //        bit 5   =0: (VGA) reserved for testage
               //        bit 4-0   : selects which register is to be accessed through 03D5
            }
            /*
                03D5  -W  CRT (6845) data register   (CGA/MCGA/color EGA/color VGA) (see #P0708)
                    selected by PORT 03D4h. registers 0C-0F may be read
                    (see also PORT 03B5h)
                    MCGA, native EGA and VGA use very different defaults from those
                    mentioned for the other adapters; for additional notes and
                    registers 00h-0Fh and EGA/VGA registers 10h-18h and ET4000
                    registers 32h-37h see PORT 03B5h (see #P0654)
                    registers 10h-11h on CGA, EGA, VGA and 12h-14h on EGA, VGA are
                    conflictive with MCGA (see #P0710)
            */
            _ => println!("XXX unhandled out_u16 to {:04X}, data {:02X}", port, data),
        }
    }

    // execute interrupt
    pub fn int(&mut self, mut hw: &mut Hardware, int: u8) {
        match int {
            0x03 => {
                // debugger interrupt
                // http://www.ctyme.com/intr/int-03.htm
                println!("INT 3 - debugger interrupt. AX={:04X}", self.get_r16(&R16::AX));
                self.fatal_error = true; // stops execution
            }
            0x10 => interrupt::int10::handle(self, &mut hw),
            0x16 => interrupt::int16::handle(self, &mut hw),
            0x1A => interrupt::int1a::handle(self, &mut hw),
            0x20 => {
                // DOS 1+ - TERMINATE PROGRAM
                // NOTE: Windows overloads INT 20
                println!("INT 20 - Terminating program");
                self.fatal_error = true; // stops execution
            }
            0x21 => interrupt::int21::handle(self, &mut hw),
            0x33 => interrupt::int33::handle(self, &mut hw),
            _ => {
                println!("int error: unknown interrupt {:02X}, AX={:04X}, BX={:04X}",
                         int,
                         self.get_r16(&R16::AX),
                         self.get_r16(&R16::BX));
            }
        }
    }
}
