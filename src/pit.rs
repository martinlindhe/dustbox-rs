// Programmable Interval Timer
// http://wiki.osdev.org/Programmable_Interval_Timer
// http://www.sat.dundee.ac.uk/psc/dosemu_time_advanced.html#The_BIOS_maintained_counter
//
// A 8253/8254 chip that runs at 18.2065 Hz (or an IRQ every 54.9254 ms)
// with the default divisor of 0x1_0000

use std::num::Wrapping;

use crate::cpu::{CPU, R};
use crate::machine::{Component, Machine};
use crate::memory::MMU;

#[cfg(test)]
#[path = "./pit_test.rs"]
mod pit_test;

const DEBUG_PIT: bool = false;

#[derive(Clone)]
pub struct PIT {
    pub timer0: Timer,
    pub timer1: Timer,
    pub timer2: Timer,
    //divisor: u32, // XXX size?!?!
}

impl Component for PIT {
    fn in_u8(&mut self, port: u16) -> Option<u8> {
        // PORT 0040-005F - PIT - PROGRAMMABLE INTERVAL TIMER (8253, 8254)
        match port {
            0x0040 => Some(self.timer0.get_next_u8()),
            0x0041 => Some(self.timer1.get_next_u8()),
            0x0042 => Some(self.timer2.get_next_u8()),
            _ => None
        }
    }

    fn out_u8(&mut self, port: u16, data: u8) -> bool {
        match port {
            0x0040 => self.timer0.write_reload_part(data),
            0x0041 => self.timer1.write_reload_part(data),
            0x0042 => self.timer2.write_reload_part(data),
            0x0043 => self.set_mode_command(data),
            _ => return false
        }
        true
    }

    fn int(&mut self, int: u8, cpu: &mut CPU) -> bool {
        if int != 0x1A {
            return false;
        }
        match cpu.get_r8(R::AH) {
            0x00 => {
                // TIME - GET SYSTEM TIME
                // Return:
                // CX:DX = number of clock ticks since midnight
                // AL = midnight flag, nonzero if midnight passed since time last read
                if cpu.deterministic {
                    cpu.set_r16(R::CX, 0);
                    cpu.set_r16(R::DX, 0);
                    cpu.set_r8(R::AL, 0);
                } else {
                    // println!("INT 1A GET TIME: get number of clock ticks since midnight, ticks {}",  hw.pit.timer0.count);
                    let cx = (self.timer0.count >> 16) as u16;
                    let dx = (self.timer0.count & 0xFFFF) as u16;
                    cpu.set_r16(R::CX, cx);
                    cpu.set_r16(R::DX, dx);
                    cpu.set_r8(R::AL, 0); // TODO implement midnight flag
                }
            }
            0x01 => {
                // TIME - SET SYSTEM TIME
                // CX:DX = number of clock ticks since midnight
                let cx = cpu.get_r16(R::CX);
                let dx = cpu.get_r16(R::DX);
                let ticks = (u32::from(cx)) << 16 | u32::from(dx);

                self.timer0.count = ticks;
                // println!("SET SYSTEM TIME to {}", ticks);
            }
            _ => return false
        }
        true
    }
}

impl PIT {
    pub fn default() -> Self {
        PIT {
            timer0: Timer::new(0),
            timer1: Timer::new(1),
            timer2: Timer::new(2),
            //divisor: 0x1_0000, // XXX
        }
    }

    // updates PIT internal state
    pub fn update(&mut self, mmu: &mut MMU) {
        self.timer0.inc();

        // MEM 0040h:006Ch - TIMER TICKS SINCE MIDNIGHT
        // Size:	DWORD
        // Desc:	updated approximately every 55 milliseconds by the BIOS INT 08 handler
        mmu.write_u32(0x0040, 0x006C, self.timer0.count);
    }

    fn counter(&mut self, n: u8) -> &mut Timer {
        match n {
            0 => &mut self.timer0,
            1 => &mut self.timer1,
            2 => &mut self.timer2,
            _ => unreachable!(),
        }
    }

    /// port 0043: control word register for counters 0-2
    /// called "8253/8254 PIT mode control word" in the interrupt list
    pub fn set_mode_command(&mut self, val: u8) {
        let channel = (val >> 6) & 0b11; // bits 7-6
        let access_mode = (val >> 4) & 0b11; // bits 5-4
        let operating_mode = (val >> 1) & 0b111; // bits 3-1
        let bcd_mode = val & 1; // bit 0
        if channel == 3 {
            panic!("TODO channel == 3: Read-back command (8254 only)");
        }
        self.counter(channel).set_mode(access_mode, operating_mode, bcd_mode);
        if DEBUG_PIT {
            println!("PIT set_mode_command channel={}, access_mode={}, operating_mode={}, bcd_mode={}", channel, access_mode, operating_mode, bcd_mode);
        }
    }
}

#[derive(Clone)]
pub struct Timer {
    pub count: u32,
    pub reload: u16,
    latch: u32,
    hi: bool,
    channel: u8, // 0-2, for debugging

    // controlled by write to port 0040:
    access_mode: AccessMode,
    operating_mode: OperatingMode,
    bcd_mode: BcdMode,
}

impl Timer {
    pub fn new(channel: u8) -> Self {
        Timer {
            count: 0,
            reload: 0,
            latch: 0,
            hi: false,
            channel,
            access_mode: AccessMode::LoByteHiByte, // XXX default?
            operating_mode: OperatingMode::Mode0, // XXX default?
            bcd_mode: BcdMode::SixteenBitBinary, // XXX default?
        }
    }

    pub fn inc(&mut self) {
        // XXX channel 0 is connected to interrupt.
        self.count += 1;
        // println!("XXX Timer.inc {} {}", self.channel, self.count);

        if self.count >= 0x0018_00B0 {
            self.count = 0;
        }
    }

    pub fn get_next_u8(&mut self) -> u8 {
        match self.access_mode {
            AccessMode::LatchCountValue => {
                // Counter Latch Command
                let res = if self.hi {
                    (self.latch >> 8) as u8
                } else {
                    (self.latch & 0xFF) as u8
                };
                self.hi = !self.hi;
                res
            }
            AccessMode::LoByteHiByte => {
                let res = if self.hi {
                    (self.count >> 8) as u8
                } else {
                    (self.count & 0xFF) as u8
                };
                self.hi = !self.hi;
                res
            }
            AccessMode::LoByteOnly => {
                panic!("AccessMode::LoByteOnly");
            }
            AccessMode::HiByteOnly => {
                panic!("AccessMode::HiByteOnly");
            }
        }
    }

    /// sets the reload value for the counter
    pub fn write_reload_part(&mut self, val: u8) {
        match self.access_mode {
            AccessMode::LatchCountValue => {
                panic!("AccessMode::LatchCountValue");
            }
            AccessMode::LoByteHiByte => {
                self.reload = if self.hi {
                    (self.reload & 0x00FF) | (u16::from(val) << 8)
                } else {
                    (self.reload & 0xFF00) | u16::from(val)
                };
                self.hi = !self.hi;
            }
            AccessMode::LoByteOnly => {
                self.reload = (self.reload & 0xFF00) | u16::from(val);
            }
            AccessMode::HiByteOnly => {
                self.reload = (self.reload & 0x00FF) | (u16::from(val) << 8);
            }
        }
    }

    pub fn set_mode(&mut self, access_mode: u8, operating_mode: u8, bcd_mode: u8) {
        // println!("pit {}: set_mode_command access {:?}, operating {:?}, bcd {:?}", self.channel, access_mode, operating_mode, bcd_mode);
        self.access_mode = match access_mode {
            0 => {
                // prepare current count value in the latch register
                self.latch = self.count;
                AccessMode::LatchCountValue
            },
            1 => AccessMode::LoByteOnly,
            2 => AccessMode::HiByteOnly,
            3 => AccessMode::LoByteHiByte,
            _ => panic!("TODO Latch count value command"),
        };
        self.operating_mode = match operating_mode {
            0 => OperatingMode::Mode0,
            1 => OperatingMode::Mode1,
            2 | 6 => OperatingMode::Mode2,
            3 | 7 => OperatingMode::Mode3,
            4 => OperatingMode::Mode4,
            5 => OperatingMode::Mode5,
            _ => unreachable!(),
        };
        self.bcd_mode = match bcd_mode {
            0 => BcdMode::SixteenBitBinary,
            //1 => BcdMode::FourDigitBCD,
            _ => panic!("TODO BCD mode"),
        };
    }
}

#[derive(Clone, Debug)]
enum AccessMode {
    LatchCountValue,
    LoByteOnly,
    HiByteOnly,
    LoByteHiByte,
}

#[derive(Clone, Debug)]
enum OperatingMode {
    Mode0, // Mode 0 (interrupt on terminal count)
    Mode1, // Mode 1 (hardware re-triggerable one-shot)
    Mode2, // Mode 2 (rate generator)
    Mode3, // Mode 3 (square wave generator)
    Mode4, // Mode 4 (software triggered strobe)
    Mode5, // Mode 5 (hardware triggered strobe)
}

#[derive(Clone, Debug)]
enum BcdMode {
    SixteenBitBinary,   // 16-bit binary
    FourDigitBCD,       // four-digit BCD
}
