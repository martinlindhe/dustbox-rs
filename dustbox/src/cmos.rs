// https://wiki.osdev.org/CMOS
// dosbox-x: src/hardware/cmos.cpp

#[derive(Clone)]
pub struct CMOS {
}

impl CMOS {
    pub fn default() -> Self {
        // XXX see CMOS_Init in dosbox-x
        CMOS {
        }
    }
}
