// https://wiki.osdev.org/BIOS
// dosbox-x: src/hardware/bios.cpp

use cpu::CPU;

#[derive(Clone)]
pub struct BIOS {
}

impl BIOS {
    pub fn new() -> Self {
        // XXX see ROMBIOS_Init in dosbox-x
        BIOS {
        }
    }
}
