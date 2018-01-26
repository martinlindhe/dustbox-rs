// Programmable Interval Timer
// http://wiki.osdev.org/Programmable_Interval_Timer
//
// A 8253/8254 chip that runs at 18.2065 Hz (or an IRQ every 54.9254 ms)
// with the default divisor of 10000h.

use std::num::Wrapping;

#[derive(Clone, Default)]
pub struct PIT {
    pub counter0: u16,
    pub counter0_hi: bool,
}

impl PIT {
    pub fn new() -> Self {
        PIT {
            counter0: 0,
            counter0_hi: false,
        }
    }

    pub fn read_40(&mut self) -> u8 {
        // FIXME: counter should decrement on a steady frequency, not on each read
        let res = if self.counter0_hi {
            (self.counter0 >> 8) as u8
        } else {
            (self.counter0 & 0xFF) as u8
        };
        self.counter0 = (Wrapping(self.counter0) - Wrapping(1)).0;
        self.counter0_hi = !self.counter0_hi;
        res
    }
}
