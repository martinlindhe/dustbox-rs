use gpu::GPU;
use memory::MMU;
use pit::PIT;
use pic::PIC;
use bios::BIOS;
use keyboard::Keyboard;

const DEBUG_IO: bool = false;

pub struct Hardware {
    pub gpu: GPU,
    pub mmu: MMU,
    pub bios: BIOS,
    pub pit: PIT,
    pub pic: PIC,
    pub pic2: PIC, // secondary pic
    pub keyboard: Keyboard,
}

impl Hardware {
    pub fn default() -> Self {
        let mut res = Self::deterministic();

        let midnight = chrono::Local::now().date().and_hms(0, 0, 0);
        let duration = chrono::Local::now().signed_duration_since(midnight).to_std().unwrap();

        // there are approximately 18.2 clock ticks per second, 0x18_00B0 per 24 hrs. one tick is generated every 54.9254ms
        res.pit.timer0.count = (((duration.as_secs() as f64 * 1000.) + (duration.subsec_nanos() as f64 / 1_000_000.)) / 54.9254) as u32;

        res
    }

    pub fn deterministic() -> Self {
        let mut mmu = MMU::default();
        let mut gpu = GPU::default();
        let mut bios = BIOS::default();
        bios.init(&mut mmu);
        gpu.init(&mut mmu);
        gpu.set_mode(&mut mmu, &mut bios, 0x03); // inits gpu to text mode 80x25

        Hardware {
            mmu,
            gpu,
            bios,
            pit: PIT::default(),
            pic: PIC::default(),
            pic2: PIC::default(),
            keyboard: Keyboard::default(),
        }
    }

    /// read byte from I/O port
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

            // PORT 0040-005F - PIT - PROGRAMMABLE INTERVAL TIMER (8253, 8254)
            0x0040 => self.pit.timer0.get_next_u8(),
            0x0041 => self.pit.timer1.get_next_u8(),
            0x0042 => self.pit.timer2.get_next_u8(),

            // PORT 0060-006F - KEYBOARD CONTROLLER 804x (8041, 8042) (or PPI (8255) on PC,XT)
            // Note: XT uses ports 60h-63h, AT uses ports 60h-64h
            0x0060 => {
                // keyboard controller data output buffer
                let (scancode, _, keypress) = self.keyboard.peek_dos_standard_scancode_and_ascii();
                if let Some(keypress) = keypress {
                    self.keyboard.consume(&keypress);
                }
                scancode
            },
            0x0061 => {
                // keyboard controller port b control register
                let val = 0 as u8; // XXX
                println!("XXX impl -- keyboard: read keyboard controller port b control register (current {:02X})", val);
                val
            }
            0x0064 => {
                // keyboard controller read status
                let val = self.keyboard.get_status_register_byte();
                val
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
            0x03C7 => self.gpu.dac.get_state(),
            0x03C8 => self.gpu.dac.get_pel_write_index(),
            0x03C9 => self.gpu.dac.get_pel_data(),
            0x03D5 => {
                // RW  CRT control register value
                // XXX
                0
            },
            0x03DA => self.gpu.read_cga_status_register(),
            _ => {
                println!("in_u8: unhandled port {:04X}", port);
                0
            }
        }
    }

    /// read word from I/O port
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

    /// write byte to I/O port
    pub fn out_u8(&mut self, port: u16, data: u8) {
        if DEBUG_IO {
            println!("out_u8: write to {:04X} = {:02X}", port, data);
        }
        match port {
            0x0020 => self.pic.set_command(data),
            0x0021 => self.pic.set_data(data),
            0x0040 => self.pit.timer0.write_reload_part(data),
            0x0041 => self.pit.timer1.write_reload_part(data),
            0x0042 => self.pit.timer2.write_reload_part(data),
            0x0043 => self.pit.set_mode_command(data),
            0x0061 => {
                // keyboard controller port b OR ppi programmable perihpial interface (XT only) - which mode are we in?
                println!("XXX impl -- keyboard: write keyboard controller port b {:02X}", data);
            },
            0x00A0 => self.pic2.set_command(data),
            0x00A1 => self.pic2.set_data(data),
            0x0201 => {
                // W  fire joystick's four one-shots
            }
            // 02C6-02C9 - VGA/MCGA - DAC REGISTERS (alternate address)
            0x02C9 => self.gpu.dac.set_pel_data(data),

            0x03B4 => self.gpu.crtc.set_index(data),           // NOTE: mirror of 03D4
            0x03B5 => self.gpu.crtc.write_current(data),

            // PORT 03C2-03CF - EGA/VGA - MISCELLANEOUS REGISTERS
            0x03C2 => {
                // -W  miscellaneous output register (see #P0669)
                // XXX impl
            },

            // PORT 03C6-03C9 - EGA/VGA/MCGA - DAC REGISTERS
            0x03C6 => self.gpu.dac.set_pel_mask(data),
            0x03C7 => self.gpu.dac.set_pel_read_index(data),
            0x03C8 => self.gpu.dac.set_pel_write_index(data),
            0x03C9 => self.gpu.dac.set_pel_data(data),

            // PORT 03D4-03D5 - COLOR VIDEO - CRT CONTROL REGISTERS
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

            // PORT 03F0-03F7 - FDC 1	(1st Floppy Disk Controller)	second FDC at 0370
            0x03F2 => {
                // 03F2  -W  diskette controller DOR (Digital Output Register) (see #P0862)

                // ../dos-software-decoding/games-com/Galaxian (1983)(Atari Inc)/galaxian.com writes 0x0C
            }

            0xD8E3 => {
                // XXX HACK REMOVE/HANDLE THIS...
                // some games write here, maybe for sound driver or something?
            }
            _ => println!("out_u8: unhandled port {:04X} = {:02X}", port, data),
        }
    }

    /// write word to I/O port
    pub fn out_u16(&mut self, port: u16, data: u16) {
        if DEBUG_IO {
            println!("out_u16: write to {:04X} = {:04X}", port, data);
        }
        match port {
            // PORT 03C4-03C5 - EGA/VGA - SEQUENCER REGISTERS
            0x03C4 => {
                // XXX if 16bit, its first INDEX byte, then DATA byte
                let _idx = data >> 8 as u8; // TS index register
                let _val = data as u8; // sequencer register index
                // println!("XXX out_u16 03C4 idx {:02X} = {:02X}", idx, val);
            },

            // PORT 03C6-03C9 - EGA/VGA/MCGA - DAC REGISTERS
            0x03C9 => self.gpu.dac.set_pel_data(data as u8),

            // PORT 03D4-03D5 - COLOR VIDEO - CRT CONTROL REGISTERS
            0x03D4 => self.gpu.crtc.set_index(data as u8),
            0x03D5 => self.gpu.crtc.write_current(data as u8),

            _ => println!("out_u16: unhandled port {:04X} = {:04X}", port, data),
        }
    }
}
