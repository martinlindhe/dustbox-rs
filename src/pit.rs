// Programmable Interval Timer
// http://wiki.osdev.org/Programmable_Interval_Timer
//
// A 8253/8254 chip that runs at 18.2065 Hz (or an IRQ every 54.9254 ms)
// with the default divisor of 0x1_0000

use std::num::Wrapping;

#[cfg(test)]
#[path = "./pit_test.rs"]
mod pit_test;

#[derive(Clone)]
pub struct Counter {
    pub count: u16,
    pub reload: u16,
    latch: u16,
    hi: bool,
    channel: u8, // 0-2, for debugging

    // controlled by write to port 0040:
    access_mode: AccessMode,
    operating_mode: OperatingMode,
    bcd_mode: BcdMode,
}

impl Counter {
    pub fn new(channel: u8) -> Self {
        Counter {
            count: 0,
            reload: 0, // 0 = 0x10000
            latch: 0,
            hi: false,
            channel: channel,

            access_mode: AccessMode::LoByteHiByte, // XXX default?
            operating_mode: OperatingMode::Mode0, // XXX default?
            bcd_mode: BcdMode::SixteenBitBinary, // XXX default?
        }
    }

    pub fn dec(&mut self) {
        // XXX dec should happen on each timer interrupt
        self.count = (Wrapping(self.count) - Wrapping(1)).0;
        if self.count == 0 {
            if self.reload == 0 {
                self.count = 0xFFFF; // XXX off-by one, how to handle 0 == 0x1_0000 ?
            } else {
                self.count = self.reload;
            }
        }
    }

    pub fn read_next_part(&mut self) -> u8 {
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

    // sets the reload value for the counter
    pub fn write_reload_part(&mut self, val: u8) {
        match self.access_mode {
            AccessMode::LatchCountValue => {
                panic!("AccessMode::LatchCountValue");
            }
            AccessMode::LoByteHiByte => {
                self.reload = if self.hi {
                    (self.reload & 0x00FF) | ((val as u16) << 8)
                } else {
                    (self.reload & 0xFF00) | val as u16
                };
                self.hi = !self.hi;
            }
            AccessMode::LoByteOnly => {
                self.reload = (self.reload & 0xFF00) | val as u16;
            }
            AccessMode::HiByteOnly => {
                self.reload = (self.reload & 0x00FF) | ((val as u16) << 8);
            }
        }
    }

    pub fn set_mode(&mut self, access_mode: u8, operating_mode: u8, bcd_mode: u8) {
        println!("pit {}: set_mode_command access {:?}, operating {:?}, bcd {:?}", self.channel, access_mode, operating_mode, bcd_mode);
        self.access_mode = match access_mode {
            0 => {
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

#[derive(Clone)]
pub struct PIT {
    pub counter0: Counter,
    pub counter1: Counter,
    pub counter2: Counter,
    //divisor: u32, // XXX size?!?!
}

impl PIT {
    pub fn new() -> Self {
        PIT {
            counter0: Counter::new(0),
            counter1: Counter::new(1),
            counter2: Counter::new(2),
            //divisor: 0x1_0000, // XXX
        }
    }

    fn counter(&mut self, n: u8) -> &mut Counter {
        match n {
            0 => &mut self.counter0,
            1 => &mut self.counter1,
            2 => &mut self.counter2,
            _ => unreachable!(),
        }
    }

    // port 0043: control word register for counters 0-2
    // called "8253/8254 PIT mode control word" in the interrupt list
    pub fn set_mode_command(&mut self, val: u8) {
        let channel = (val >> 6) & 0b11; // bits 7-6
        let access_mode = (val >> 4) & 0b11; // bits 5-4
        let operating_mode = (val >> 1) & 0b11; // bits 3-1
        let bcd_mode = val & 1; // bit 0
        if channel == 3 {
            panic!("TODO channel == 3: Read-back command (8254 only)");
        }
        self.counter(channel).set_mode(access_mode, operating_mode, bcd_mode);
    }
}
