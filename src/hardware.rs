use gpu::GPU;
use memory::mmu::MMU;
use pit::PIT;
use pic::PIC;
use bios::BIOS;

const DEBUG_IO: bool = false;

pub struct Hardware {
    pub gpu: GPU,
    pub mmu: MMU,
    pub bios: BIOS,
    pub pit: PIT,
    pub pic: PIC,
    pub pic2: PIC, // secondary pic
}

impl Hardware {
    pub fn new() -> Self {
        let mut mmu = MMU::new();
        let mut gpu = GPU::new();
        let mut bios = BIOS::new();
        bios.init(&mut mmu);
        gpu.init(&mut mmu);
        Hardware {
            mmu: mmu,
            gpu: gpu,
            bios: bios,
            pit: PIT::new(),
            pic: PIC::new(),
            pic2: PIC::new(),
        }
    }

    // read byte from I/O port
    pub fn in_u8(&mut self, port: u16) -> u8 {
        if DEBUG_IO {
            println!("in_u8: read from {:04X}", port);
        }
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
            0x03DA => self.gpu.read_cga_status_register(),
            _ => {
                println!("in_u8: unhandled port {:04X}", port);
                0
            }
        }
    }

    // read word from I/O port
    pub fn in_u16(&mut self, port: u16) -> u16 {
        if DEBUG_IO {
            println!("in_u16: read from {:04X}", port);
        }
        match port {
            _ => {
                println!("in_u16: unhandled port {:04X}", port);
                0
            }
        }
    }

    // write byte to I/O port
    pub fn out_u8(&mut self, port: u16, data: u8) {
        if DEBUG_IO {
            println!("out_u8: write to {:04X} = {:02X}", port, data);
        }
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
            0x03B4 => self.gpu.crtc.set_index(data),           // NOTE: mirroring 3d4 is what dosbox does too
            0x03B5 => self.gpu.crtc.write_current(data),
            0x03C7 => self.gpu.set_pel_address(data),   // XXX unsure if understood correctly
            0x03C8 => self.gpu.set_pel_address(data),
            0x03C9 => self.gpu.set_pel_data(data),
            0x03D4 => self.gpu.crtc.set_index(data),
            0x03D5 => self.gpu.crtc.write_current(data),
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
            _ => println!("out_u8: unhandled port {:04X} = {:02X}", port, data),
        }
    }

    // write word to I/O port
    pub fn out_u16(&mut self, port: u16, data: u16) {
        if DEBUG_IO {
            println!("out_u16: write to {:04X} = {:04X}", port, data);
        }
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
            0x03D4 => self.gpu.crtc.set_index(data as u8),
            0x03D5 => self.gpu.crtc.write_current(data as u8),
            _ => println!("out_u16: unhandled port {:04X} = {:04X}", port, data),
        }
    }
}
