// Programmable Interval Timer
// http://wiki.osdev.org/Programmable_Interval_Timer
//
// A 8253/8254 chip that runs at 18.2065 Hz (or an IRQ every 54.9254 ms)
// with the default divisor of 10000h.

use std::num::Wrapping;

#[derive(Clone, Default)]
pub struct Counter {
    pub counter: u16,
    hi: bool,
}

impl Counter {
    pub fn new() -> Self {
        Counter {
            counter: 0,
            hi: false,
        }
    }

    pub fn dec(&mut self) {
        self.counter = (Wrapping(self.counter) - Wrapping(1)).0;
    }

    pub fn read_next_part(&mut self) -> u8 {
        let res = if self.hi {
            (self.counter >> 8) as u8
        } else {
            (self.counter & 0xFF) as u8
        };
        self.hi = !self.hi;
        res
    }

    pub fn write_next_part(&mut self, val: u8) {
        self.counter = if self.hi {
            (self.counter & 0x00FF) | ((val as u16) << 8)
        } else {
            (self.counter & 0xFF00) | val as u16
        };
        self.hi = !self.hi;
    }
}

#[derive(Clone, Default)]
pub struct PIT {
    pub counter0: Counter,
    pub counter1: Counter,
    pub counter2: Counter,
}

impl PIT {
    pub fn new() -> Self {
        PIT {
            counter0: Counter::new(), // read of i/o port 0x0040
            counter1: Counter::new(), // read of i/o port 0x0041
            counter2: Counter::new(), // read of i/o port 0x0042
        }
    }

    pub fn set_counter_divisor(&mut self, val: u8) {
        // XXX impl
    }

    pub fn set_mode_port(&mut self, val: u8) {
        // control word register for counters 0-2 (see #P0380)
        // XXX impl
    }
}
